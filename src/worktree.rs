use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::db;
use crate::git;

/// Get the hashed name for a project
fn get_hashed_name(repo_path: &Path) -> String {
    let path_str = repo_path.to_string_lossy();
    let mut hasher = Sha256::new();
    hasher.update(path_str.as_bytes());
    let result = hasher.finalize();
    hex::encode(&result[..8]) // Use first 8 bytes for shorter hash
}

/// Get the worktree base directory
fn get_worktree_base(repo_path: &Path) -> Result<PathBuf> {
    let home = dirs::home_dir().context("Failed to get home directory")?;
    let repo_name = repo_path
        .file_name()
        .and_then(|n| n.to_str())
        .context("Invalid repository path")?;
    
    let hashed = get_hashed_name(repo_path);
    let wt_base = home.join("_wt").join(format!("{}_{}", repo_name, hashed));
    
    Ok(wt_base)
}

/// Get the full path for a worktree
fn get_worktree_path(repo_path: &Path, branch: &str) -> Result<PathBuf> {
    let wt_base = get_worktree_base(repo_path)?;
    Ok(wt_base.join(branch))
}

/// Run automatic setup based on project files
fn run_auto_setup(worktree_path: &Path) -> Result<()> {
    let mut commands = Vec::new();
    let mut shell_cmd = String::new();

    // Check for .nvmrc
    if worktree_path.join(".nvmrc").exists() {
        commands.push("nvm use");
    }

    // Check for package managers
    if worktree_path.join("pnpm-lock.yaml").exists() {
        commands.push("pnpm install");
    } else if worktree_path.join("yarn.lock").exists() {
        commands.push("yarn install");
    }

    if commands.is_empty() {
        return Ok(());
    }

    // Construct the shell command
    // We try to source zshrc to get nvm if needed, assuming user is on zsh as per env
    if commands.contains(&"nvm use") {
        shell_cmd.push_str("source ~/.zshrc 2>/dev/null || true; ");
    }
    
    shell_cmd.push_str(&commands.join(" && "));
    
    println!("Running automatic setup: {}", shell_cmd);

    // Use zsh to execute the chain
    let output = Command::new("zsh")
        .arg("-c")
        .arg(&shell_cmd)
        .current_dir(worktree_path)
        .output();

    match output {
        Ok(output) if output.status.success() => {
            println!("✓ Setup completed successfully");
            Ok(())
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            eprintln!("Warning: Setup completed with issues.");
            if !stdout.trim().is_empty() {
                println!("Output: {}", stdout);
            }
            if !stderr.trim().is_empty() {
                eprintln!("Error output: {}", stderr);
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("Warning: Could not run setup command: {}", e);
            Ok(())
        }
    }
}

/// Change to the worktree directory and run setup
fn switch_to_worktree(worktree_path: &Path) -> Result<()> {
    // We can't actually change the directory of the parent shell from Rust
    // Instead, we'll print the command for the user to execute
    println!("\n✓ Worktree ready at: {}", worktree_path.display());
    println!("\nTo switch to this worktree, run:");
    println!("  cd {}", worktree_path.display());

    run_auto_setup(worktree_path)?;
    
    Ok(())
}

/// Handle worktree creation or switching
pub fn handle_worktree(repo_root: &Path, branch: &str) -> Result<()> {
    let worktree_path = get_worktree_path(repo_root, branch)?;

    // Check if worktree already exists
    if worktree_path.exists() {
        println!("Worktree already exists for branch '{}'", branch);
        db::update_last_accessed(repo_root)?;
        return switch_to_worktree(&worktree_path);
    }

    // Create worktree base directory
    let wt_base = get_worktree_base(repo_root)?;
    fs::create_dir_all(&wt_base)?;

    // Try to add worktree for existing branch first
    println!("Adding worktree for branch '{}'", branch);
    let result = git::add_worktree(repo_root, &worktree_path, branch, false);

    match result {
        Ok(_) => {
            println!("✓ Worktree added for existing branch '{}'", branch);
        }
        Err(_) => {
            // Branch doesn't exist, create new one
            println!("Branch '{}' not found, creating new branch", branch);
            git::add_worktree(repo_root, &worktree_path, branch, true)
                .context("Failed to create new branch and worktree")?;
            println!("✓ Created new branch '{}' with worktree", branch);
        }
    }

    db::update_last_accessed(repo_root)?;
    switch_to_worktree(&worktree_path)?;

    Ok(())
}
