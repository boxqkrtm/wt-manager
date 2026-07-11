use anyhow::Result;
#[cfg(any(not(windows), test))]
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

pub struct SetupManager;

#[cfg(any(not(windows), test))]
fn split_path_entries(path: Option<&std::ffi::OsStr>) -> Vec<PathBuf> {
    path.map(env::split_paths).into_iter().flatten().collect()
}

#[cfg(any(not(windows), test))]
fn command_exists_in_path(command: &str, path: Option<&std::ffi::OsStr>) -> bool {
    split_path_entries(path)
        .into_iter()
        .map(|entry| entry.join(command))
        .any(|candidate| candidate.is_file())
}

#[cfg(not(windows))]
fn hook_shell() -> (&'static str, &'static str) {
    hook_shell_for_path(env::var_os("PATH").as_deref())
}

#[cfg(not(windows))]
fn hook_shell_for_path(path: Option<&std::ffi::OsStr>) -> (&'static str, &'static str) {
    if command_exists_in_path("zsh", path) {
        (
            "zsh",
            "source ~/.zshrc 2>/dev/null || source ~/.bashrc 2>/dev/null || true; ",
        )
    } else if command_exists_in_path("bash", path) {
        (
            "bash",
            "source ~/.bashrc 2>/dev/null || source ~/.zshrc 2>/dev/null || true; ",
        )
    } else {
        ("sh", ". ~/.profile 2>/dev/null || true; ")
    }
}
#[cfg(windows)]
const LEGACY_POSIX_PROFILE_SOURCE_PREFIX: &str =
    "source ~/.zshrc 2>/dev/null || source ~/.bashrc 2>/dev/null || true; ";

#[cfg(windows)]
fn normalize_windows_hook(hook: &str) -> &str {
    hook.strip_prefix(LEGACY_POSIX_PROFILE_SOURCE_PREFIX)
        .unwrap_or(hook)
}

#[cfg(windows)]
fn windows_hook_shell_with(
    mut resolve: impl FnMut(&std::ffi::OsStr) -> Option<PathBuf>,
) -> Option<PathBuf> {
    ["pwsh", "powershell"]
        .into_iter()
        .find_map(|name| resolve(std::ffi::OsStr::new(name)))
}

#[cfg(windows)]
fn windows_hook_shell() -> Option<PathBuf> {
    windows_hook_shell_with(crate::process::resolve_executable)
}

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
        if hooks.is_empty() {
            return Ok(());
        }

        #[cfg(not(windows))]
        let (shell, shell_setup) = hook_shell();

        #[cfg(windows)]
        let shell = windows_hook_shell().ok_or_else(|| {
            anyhow::anyhow!(
                "Cannot run {} hooks: neither 'pwsh' nor 'powershell' was found on PATH",
                hook_name
            )
        })?;

        for hook in hooks {
            println!("Running {} hook: {}", hook_name, hook);

            #[cfg(not(windows))]
            let output = Command::new(shell)
                .arg("-c")
                .arg(format!("{}{}", shell_setup, hook))
                .current_dir(worktree_path)
                .output();

            #[cfg(windows)]
            let output = Command::new(&shell)
                .arg("-NoLogo")
                .arg("-Command")
                .arg(normalize_windows_hook(hook))
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

#[cfg(test)]
mod tests {
    #[cfg(not(windows))]
    use super::hook_shell_for_path;
    use super::{command_exists_in_path, split_path_entries};
    #[cfg(windows)]
    use super::{normalize_windows_hook, windows_hook_shell_with, SetupManager};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_temp_dir(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("wt-manager-{prefix}-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn split_path_entries_handles_missing_path() {
        assert!(split_path_entries(None).is_empty());
    }

    #[test]
    fn command_exists_in_path_checks_each_directory() {
        let dir = make_temp_dir("path");
        let fake_bash = dir.join("bash");
        fs::write(&fake_bash, "#!/bin/sh\n").unwrap();

        let path = std::env::join_paths([&dir]).unwrap();

        assert!(command_exists_in_path("bash", Some(path.as_os_str())));
        assert!(!command_exists_in_path("zsh", Some(path.as_os_str())));

        fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(not(windows))]
    #[test]
    fn hook_shell_prefers_zsh_then_bash_then_sh() {
        let zsh_dir = make_temp_dir("zsh-path");
        let bash_dir = make_temp_dir("bash-path");
        fs::write(zsh_dir.join("zsh"), "#!/bin/sh\n").unwrap();
        fs::write(bash_dir.join("bash"), "#!/bin/sh\n").unwrap();

        let preferred = std::env::join_paths([&zsh_dir, &bash_dir]).unwrap();
        let fallback = std::env::join_paths([&bash_dir]).unwrap();
        let missing_dir = make_temp_dir("missing-path");
        let missing = std::env::join_paths([&missing_dir]).unwrap();

        assert_eq!(hook_shell_for_path(Some(preferred.as_os_str())).0, "zsh");
        assert_eq!(hook_shell_for_path(Some(fallback.as_os_str())).0, "bash");
        assert_eq!(hook_shell_for_path(Some(missing.as_os_str())).0, "sh");

        fs::remove_dir_all(zsh_dir).unwrap();
        fs::remove_dir_all(bash_dir).unwrap();
        fs::remove_dir_all(missing_dir).unwrap();
    }
    #[cfg(windows)]
    #[test]
    fn windows_hook_shell_prefers_pwsh_then_windows_powershell() {
        use std::ffi::OsStr;

        let pwsh = PathBuf::from(r"C:\Program Files\PowerShell\7\pwsh.exe");
        let windows_powershell =
            PathBuf::from(r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe");

        let selected = windows_hook_shell_with(|name| match name.to_str().unwrap() {
            "pwsh" => Some(pwsh.clone()),
            "powershell" => Some(windows_powershell.clone()),
            _ => None,
        });
        assert_eq!(selected, Some(pwsh));

        let mut attempts = Vec::new();
        let selected = windows_hook_shell_with(|name: &OsStr| {
            attempts.push(name.to_owned());
            (name == OsStr::new("powershell")).then(|| windows_powershell.clone())
        });
        assert_eq!(selected, Some(windows_powershell));
        assert_eq!(
            attempts,
            [
                OsStr::new("pwsh").to_owned(),
                OsStr::new("powershell").to_owned()
            ]
        );
        assert_eq!(windows_hook_shell_with(|_| None), None);
    }

    #[cfg(windows)]
    #[test]
    fn windows_hook_normalization_removes_only_legacy_seed_prefix() {
        let legacy =
            "source ~/.zshrc 2>/dev/null || source ~/.bashrc 2>/dev/null || true; mise install";
        assert_eq!(normalize_windows_hook(legacy), "mise install");
        assert_eq!(normalize_windows_hook("pnpm install"), "pnpm install");
        assert_eq!(
            normalize_windows_hook(
                "source ~/.bashrc 2>/dev/null || source ~/.zshrc 2>/dev/null || true; nvm use"
            ),
            "source ~/.bashrc 2>/dev/null || source ~/.zshrc 2>/dev/null || true; nvm use"
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_hooks_execute_in_powershell_from_the_worktree_directory() {
        let worktree = make_temp_dir("powershell-hook");
        let hook = format!(
            "{}Set-Content -LiteralPath 'hook-ran.txt' -Value 'ok' -NoNewline",
            super::LEGACY_POSIX_PROFILE_SOURCE_PREFIX
        );

        SetupManager::run_hooks(&worktree, "post-create", &[hook]).unwrap();

        assert!(worktree.join("hook-ran.txt").is_file());
        fs::remove_dir_all(worktree).unwrap();
    }
}
