use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

use crate::db;
use crate::git;

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
            &format!("{}:{}/{}", repository.host, repository.owner, repository.name),
            5,
        );
        let owner_repo_hash_base = home.join("_wt").join(format!(
            "{}-{}-{}",
            sanitize_segment(&repository.owner),
            sanitize_segment(&repository.name),
            remote_hash
        ));
        let owner_repo_base = home.join("_wt").join(format!(
            "{}-{}",
            sanitize_segment(&repository.owner),
            sanitize_segment(&repository.name)
        ));

        candidates.push(owner_repo_hash_base);
        // Keep supporting the pre-hash owner-repo layout for backward compatibility.
        // This legacy fallback can be removed once existing users have migrated.
        if !candidates.contains(&owner_repo_base) {
            candidates.push(owner_repo_base);
        }
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
fn switch_to_worktree(worktree_path: &Path) -> Result<()> {
    let messages = crate::i18n::Messages::new();
    // We can't actually change the directory of the parent shell from Rust
    // Instead, we'll print the command for the user to execute
    println!("\n{} {}", messages.worktree_ready(), worktree_path.display());
    println!("\n{}", messages.switch_to_worktree_guide());
    println!("  cd {}", worktree_path.display());

    crate::setup::SetupManager::run_auto_setup(worktree_path)?;
    
    Ok(())
}

/// Handle worktree creation or switching
pub fn handle_worktree(repo_root: &Path, branch: &str) -> Result<()> {
    let messages = crate::i18n::Messages::new();
    let worktree_path = find_existing_worktree_path(repo_root, branch)?
        .unwrap_or(get_worktree_path(repo_root, branch)?);

    // Check if worktree already exists
    if worktree_path.exists() {
        println!("{}", messages.worktree_already_exists().replace("{}", branch));
        db::update_last_accessed(repo_root)?;
        return switch_to_worktree(&worktree_path);
    }

    // Create worktree base directory
    let wt_base = get_worktree_base(repo_root)?;
    fs::create_dir_all(&wt_base)?;

    // Try to add worktree for existing branch first
    println!("{}", messages.adding_worktree_for_branch().replace("{}", branch));
    let result = git::add_worktree(repo_root, &worktree_path, branch, false);

    match result {
        Ok(_) => {
            println!("{}", messages.worktree_added_existing_branch().replace("{}", branch));
        }
        Err(_) => {
            // Branch doesn't exist, create new one
            println!("{}", messages.branch_not_found_create_new().replace("{}", branch));
            let err_message = messages.failed_create_worktree_context().to_string();
            git::add_worktree(repo_root, &worktree_path, branch, true).context(err_message)?;
            println!("{}", messages.created_new_branch_with_worktree().replace("{}", branch));
        }
    }

    db::update_last_accessed(repo_root)?;
    switch_to_worktree(&worktree_path)?;

    Ok(())
}
