use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;


/// Find the main repository root (handles worktrees)
/// If in a worktree, returns the main repository root
/// If in the main repository, returns the repository root
pub fn find_main_repo_root(start_path: &Path) -> Result<Option<PathBuf>> {
    // Get the common git directory (main repo's .git)
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--path-format=absolute")
        .arg("--git-common-dir")
        .current_dir(start_path)
        .output()?;

    if !output.status.success() {
        return Ok(None);
    }

    let git_common_dir = String::from_utf8(output.stdout)?
        .trim()
        .to_string();
    
    let git_common_path = PathBuf::from(git_common_dir);
    
    // The parent of .git directory is the main repo root
    if let Some(parent) = git_common_path.parent() {
        Ok(Some(parent.to_path_buf()))
    } else {
        Ok(None)
    }
}

/// List all worktrees for a repository
pub fn list_worktrees(repo_root: &Path) -> Result<Vec<WorktreeInfo>> {
    let output = Command::new("git")
        .arg("worktree")
        .arg("list")
        .arg("--porcelain")
        .current_dir(repo_root)
        .output()
        .context("Failed to list worktrees")?;

    if !output.status.success() {
        anyhow::bail!("git worktree list failed");
    }

    let stdout = String::from_utf8(output.stdout)?;
    parse_worktree_list(&stdout)
}

#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub branch: String,
    pub is_main: bool,
}

fn parse_worktree_list(output: &str) -> Result<Vec<WorktreeInfo>> {
    let mut worktrees = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_branch: Option<String> = None;
    let mut is_main = false;

    for line in output.lines() {
        if line.starts_with("worktree ") {
            // Save previous worktree if exists
            if let (Some(path), Some(branch)) = (current_path.take(), current_branch.take()) {
                worktrees.push(WorktreeInfo {
                    path,
                    branch,
                    is_main,
                });
            }
            
            current_path = Some(PathBuf::from(line.trim_start_matches("worktree ")));
            is_main = false;
        } else if line.starts_with("branch ") {
            let branch = line.trim_start_matches("branch ");
            current_branch = Some(branch.trim_start_matches("refs/heads/").to_string());
        } else if line.starts_with("bare") {
            is_main = true;
        } else if line.is_empty() {
            // End of worktree entry
            if let (Some(path), Some(branch)) = (current_path.take(), current_branch.take()) {
                worktrees.push(WorktreeInfo {
                    path,
                    branch,
                    is_main,
                });
            }
        }
    }

    // Save last worktree if exists
    if let (Some(path), Some(branch)) = (current_path, current_branch) {
        worktrees.push(WorktreeInfo {
            path,
            branch,
            is_main,
        });
    }

    Ok(worktrees)
}

/// Add a new worktree
pub fn add_worktree(repo_root: &Path, worktree_path: &Path, branch: &str, create_branch: bool) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.arg("worktree")
        .arg("add");

    if create_branch {
        // For new branch: git worktree add -b <branch> <path>
        cmd.arg("-b").arg(branch).arg(worktree_path);
    } else {
        // For existing branch: git worktree add <path> <branch>
        cmd.arg(worktree_path).arg(branch);
    }

    cmd.current_dir(repo_root);

    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to add worktree: {}", stderr);
    }

    Ok(())
}

/// Remove a worktree
/// Returns an error if the worktree has uncommitted changes
pub fn remove_worktree(repo_root: &Path, worktree_path: &Path) -> Result<()> {
    let output = Command::new("git")
        .arg("worktree")
        .arg("remove")
        .arg(worktree_path)
        .current_dir(repo_root)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to remove worktree: {}", stderr);
    }

    Ok(())
}
