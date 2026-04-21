use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

use crate::db;
use crate::git;

pub struct PreparedWorktree {
    pub path: PathBuf,
    pub existed: bool,
}

fn short_hash(value: &str, length: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)[..length].to_string()
}

/// Get the hashed name for a project
fn get_hashed_name(repo_path: &Path) -> String {
    short_hash(&repo_path.to_string_lossy(), 16)
}

fn sanitize_segment(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '-'
            }
        })
        .collect()
}

fn get_worktree_base_candidates(repo_path: &Path) -> Result<Vec<PathBuf>> {
    let messages = crate::i18n::Messages::new();
    let home = dirs::home_dir().context(messages.failed_get_home_dir().to_string())?;
    let repo_name = repo_path
        .file_name()
        .and_then(|n| n.to_str())
        .context(messages.invalid_repository_path().to_string())?;

    let hashed = get_hashed_name(repo_path);
    let legacy_base = home.join("_wt").join(format!("{}_{}", repo_name, hashed));
    let mut candidates = Vec::new();

    if let Some(repository) = git::get_repository_slug(repo_path)? {
        let remote_hash = short_hash(
            &format!(
                "{}:{}/{}",
                repository.host, repository.owner, repository.name
            ),
            5,
        );
        let owner_repo_hash_base = home.join("_wt").join(format!(
            "{}-{}-{}",
            sanitize_segment(&repository.owner),
            sanitize_segment(&repository.name),
            remote_hash
        ));

        candidates.push(owner_repo_hash_base);
    }

    // Keep supporting the oldest repo_hash layout for backward compatibility.
    // This legacy fallback can be removed once existing users have migrated.
    if !candidates.contains(&legacy_base) {
        candidates.push(legacy_base);
    }

    Ok(candidates)
}

/// Get the worktree base directory
fn get_worktree_base(repo_path: &Path) -> Result<PathBuf> {
    let candidates = get_worktree_base_candidates(repo_path)?;

    if let Some(existing_base) = candidates.iter().find(|candidate| candidate.exists()) {
        return Ok(existing_base.clone());
    }

    Ok(candidates
        .into_iter()
        .next()
        .expect("worktree base candidates should never be empty"))
}

fn find_existing_worktree_path(repo_path: &Path, branch: &str) -> Result<Option<PathBuf>> {
    for base in get_worktree_base_candidates(repo_path)? {
        let candidate = base.join(branch);
        if candidate.exists() {
            return Ok(Some(candidate));
        }
    }

    Ok(None)
}

/// Get the full path for a worktree
fn get_worktree_path(repo_path: &Path, branch: &str) -> Result<PathBuf> {
    let wt_base = get_worktree_base(repo_path)?;
    Ok(wt_base.join(branch))
}

/// Change to the worktree directory and run setup
fn switch_to_worktree(repo_root: &Path, worktree_path: &Path) -> Result<()> {
    let messages = crate::i18n::Messages::new();
    // We can't actually change the directory of the parent shell from Rust
    // Instead, we'll print the command for the user to execute
    println!(
        "\n{} {}",
        messages.worktree_ready(),
        worktree_path.display()
    );
    println!("\n{}", messages.switch_to_worktree_guide());
    println!("  cd {}", worktree_path.display());

    crate::setup::SetupManager::run_post_cd(repo_root, worktree_path)?;
    crate::setup::SetupManager::run_auto_setup(worktree_path)?;

    Ok(())
}

pub fn prepare_worktree(repo_root: &Path, branch: &str) -> Result<PreparedWorktree> {
    let messages = crate::i18n::Messages::new();
    if let Some(worktree_path) = find_existing_worktree_path(repo_root, branch)? {
        println!(
            "{}",
            messages.worktree_already_exists().replace("{}", branch)
        );
        db::update_last_accessed(repo_root)?;
        return Ok(PreparedWorktree {
            path: worktree_path,
            existed: true,
        });
    }

    let worktree_path = get_worktree_path(repo_root, branch)?;

    // Create worktree base directory
    let wt_base = get_worktree_base(repo_root)?;
    fs::create_dir_all(&wt_base)?;

    if git::worktree_target_exists(repo_root, branch)? {
        println!(
            "{}",
            messages.adding_worktree_for_branch().replace("{}", branch)
        );
        git::add_worktree(repo_root, &worktree_path, branch, false)?;
        println!(
            "{}",
            messages
                .worktree_added_existing_branch()
                .replace("{}", branch)
        );
    } else {
        println!(
            "{}",
            messages.branch_not_found_create_new().replace("{}", branch)
        );
        let err_message = messages.failed_create_worktree_context().to_string();
        git::add_worktree(repo_root, &worktree_path, branch, true).context(err_message)?;
        println!(
            "{}",
            messages
                .created_new_branch_with_worktree()
                .replace("{}", branch)
        );
    }

    db::update_last_accessed(repo_root)?;
    Ok(PreparedWorktree {
        path: worktree_path,
        existed: false,
    })
}

/// Handle worktree creation or switching
pub fn handle_worktree(repo_root: &Path, branch: &str) -> Result<()> {
    let prepared = prepare_worktree(repo_root, branch)?;
    if !prepared.existed {
        crate::setup::SetupManager::run_post_create(repo_root, &prepared.path)?;
    }
    switch_to_worktree(repo_root, &prepared.path)?;

    Ok(())
}
