use anyhow::Result;
use std::fs;
use std::path::{Component, Path};
use std::process::Command;

pub struct SetupManager;

impl SetupManager {
    pub fn run_auto_setup(worktree_path: &Path) -> Result<()> {
        let _ = worktree_path;
        Ok(())
    }

    pub fn run_post_create(repo_root: &Path, worktree_path: &Path) -> Result<()> {
        if let Some(config) = crate::db::get_project_automation(repo_root)? {
            Self::copy_configured_files(repo_root, worktree_path, &config.copy_files)?;
            Self::run_hooks(worktree_path, "post-create", &config.post_create_hooks)?;
        }

        Ok(())
    }

    pub fn run_post_cd(repo_root: &Path, worktree_path: &Path) -> Result<()> {
        if let Some(config) = crate::db::get_project_automation(repo_root)? {
            Self::run_hooks(worktree_path, "post-cd", &config.post_cd_hooks)?;
        }

        Ok(())
    }

    fn copy_configured_files(
        repo_root: &Path,
        worktree_path: &Path,
        copy_files: &[String],
    ) -> Result<()> {
        for relative_path in copy_files {
            let path = Path::new(relative_path);
            let is_safe_relative = !path.is_absolute()
                && path.components().all(|component| {
                    !matches!(
                        component,
                        Component::ParentDir | Component::RootDir | Component::Prefix(_)
                    )
                });

            if !is_safe_relative {
                eprintln!("Warning: skipping unsafe copy path '{}'", relative_path);
                continue;
            }

            let source = repo_root.join(relative_path);
            if !source.exists() {
                eprintln!("Warning: copy source not found '{}'", relative_path);
                continue;
            }

            let destination = worktree_path.join(relative_path);
            if destination.exists() {
                eprintln!(
                    "Warning: copy destination already exists '{}'",
                    relative_path
                );
                continue;
            }

            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }

            fs::copy(&source, &destination)?;
            println!("Copied '{}'", relative_path);
        }

        Ok(())
    }

    fn run_hooks(worktree_path: &Path, hook_name: &str, hooks: &[String]) -> Result<()> {
        for hook in hooks {
            println!("Running {} hook: {}", hook_name, hook);

            let output = Command::new("zsh")
                .arg("-c")
                .arg(format!(
                    "source ~/.zshrc 2>/dev/null || source ~/.bashrc 2>/dev/null || true; {}",
                    hook
                ))
                .current_dir(worktree_path)
                .output();

            match output {
                Ok(output) if output.status.success() => {}
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    eprintln!("Warning: {} hook failed", hook_name);
                    if !stdout.trim().is_empty() {
                        println!("Output: {}", stdout);
                    }
                    if !stderr.trim().is_empty() {
                        eprintln!("Error output: {}", stderr);
                    }
                }
                Err(error) => {
                    eprintln!("Warning: failed to run {} hook: {}", hook_name, error);
                }
            }
        }

        Ok(())
    }
}
