use std::env;
use std::ffi::OsStr;
#[cfg(windows)]
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

/// Resolves an executable name without invoking a shell.
///
/// Commands containing a directory component are treated as explicit paths and
/// are never searched for in `PATH`. Bare command names are checked in each
/// `PATH` entry in order. On Windows, extensionless commands additionally use
/// the standard executable and command-script suffixes.
pub(crate) fn resolve_executable(command: &OsStr) -> Option<PathBuf> {
    resolve_executable_in_path(command, env::var_os("PATH").as_deref())
}

fn resolve_executable_in_path(command: &OsStr, path: Option<&OsStr>) -> Option<PathBuf> {
    if command.is_empty() {
        return None;
    }

    let command_path = Path::new(command);
    if has_directory_component(command_path) {
        return resolve_candidate(command_path);
    }

    path.into_iter()
        .flat_map(env::split_paths)
        .find_map(|directory| resolve_candidate(&directory.join(command_path)))
}

fn has_directory_component(path: &Path) -> bool {
    let mut components = path.components();
    !matches!(
        (components.next(), components.next()),
        (Some(Component::Normal(_)), None)
    )
}

fn resolve_candidate(path: &Path) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        if path.extension().is_some() {
            return path.is_file().then(|| path.to_path_buf());
        }

        for extension in [".com", ".exe", ".bat", ".cmd"] {
            let mut candidate = OsString::from(path.as_os_str());
            candidate.push(extension);
            let candidate = PathBuf::from(candidate);
            if candidate.is_file() {
                return Some(candidate);
            }
        }

        return None;
    }

    #[cfg(not(windows))]
    path.is_file().then(|| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::resolve_executable_in_path;
    use std::env;
    use std::ffi::{OsStr, OsString};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_TEMP_DIR: AtomicU64 = AtomicU64::new(0);

    struct TestDir(PathBuf);

    impl TestDir {
        fn new(label: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock must be after the Unix epoch")
                .as_nanos();
            let sequence = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
            let path = env::temp_dir().join(format!(
                "wt-manager-process-{label}-{}-{nonce}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("temporary test directory must be created");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn joined_path(directory: &Path) -> OsString {
        env::join_paths([directory]).expect("test directory must be a valid PATH entry")
    }

    #[test]
    fn missing_path_and_commands_return_none() {
        let directory = TestDir::new("missing");
        let path = joined_path(directory.path());

        assert_eq!(
            resolve_executable_in_path(OsStr::new("missing"), None),
            None
        );
        assert_eq!(
            resolve_executable_in_path(OsStr::new("missing"), Some(path.as_os_str())),
            None
        );
        assert_eq!(resolve_executable_in_path(OsStr::new(""), None), None);
    }

    #[test]
    fn explicit_file_is_resolved_without_path() {
        let directory = TestDir::new("explicit");
        let executable = directory.path().join("tool.custom");
        fs::write(&executable, b"test").expect("test file must be written");

        assert_eq!(
            resolve_executable_in_path(executable.as_os_str(), None),
            Some(executable)
        );
    }

    #[cfg(windows)]
    #[test]
    fn explicit_extensionless_path_finds_cmd_file() {
        let directory = TestDir::new("explicit-cmd");
        let command = directory.path().join("tool");
        let executable = directory.path().join("tool.cmd");
        fs::write(&executable, b"@echo off\r\n").expect("test command must be written");

        assert_eq!(
            resolve_executable_in_path(command.as_os_str(), None),
            Some(executable)
        );
    }

    #[test]
    fn relative_path_with_directory_component_is_not_searched_in_path() {
        let path_directory = TestDir::new("no-path-fallback");
        let unique_parent = format!(
            "wt-manager-relative-command-{}-{}",
            std::process::id(),
            NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed)
        );
        let relative_command = Path::new(&unique_parent).join("tool");
        let misplaced_parent = path_directory.path().join(&unique_parent);
        fs::create_dir(&misplaced_parent).expect("nested test directory must be created");

        #[cfg(windows)]
        let misplaced_executable = misplaced_parent.join("tool.CMD");
        #[cfg(not(windows))]
        let misplaced_executable = misplaced_parent.join("tool");

        fs::write(misplaced_executable, b"test").expect("test file must be written");
        let path = joined_path(path_directory.path());

        assert_eq!(
            resolve_executable_in_path(relative_command.as_os_str(), Some(path.as_os_str())),
            None
        );
    }

    #[cfg(windows)]
    #[test]
    fn extensionless_command_finds_cmd_file_in_path() {
        let directory = TestDir::new("cmd");
        let executable = directory.path().join("foo.cmd");
        fs::write(&executable, b"@echo off\r\n").expect("test command must be written");
        let path = joined_path(directory.path());

        assert_eq!(
            resolve_executable_in_path(OsStr::new("foo"), Some(path.as_os_str())),
            Some(executable)
        );
    }

    #[cfg(windows)]
    #[test]
    fn extensionless_command_ignores_posix_shim_and_finds_cmd_file() {
        let directory = TestDir::new("cmd-with-posix-shim");
        fs::write(directory.path().join("foo"), b"#!/bin/sh\n").expect("test shim must be written");
        let executable = directory.path().join("foo.cmd");
        fs::write(&executable, b"@echo off\r\n").expect("test command must be written");
        let path = joined_path(directory.path());

        assert_eq!(
            resolve_executable_in_path(OsStr::new("foo"), Some(path.as_os_str())),
            Some(executable)
        );
    }

    #[cfg(windows)]
    #[test]
    fn command_with_extension_does_not_gain_another_extension() {
        let directory = TestDir::new("existing-extension");
        fs::write(directory.path().join("foo.cmd.exe"), b"test")
            .expect("test file must be written");
        let path = joined_path(directory.path());

        assert_eq!(
            resolve_executable_in_path(OsStr::new("foo.cmd"), Some(path.as_os_str())),
            None
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn bare_command_finds_exact_file_in_path() {
        let directory = TestDir::new("path");
        let executable = directory.path().join("foo");
        fs::write(&executable, b"test").expect("test command must be written");
        let path = joined_path(directory.path());

        assert_eq!(
            resolve_executable_in_path(OsStr::new("foo"), Some(path.as_os_str())),
            Some(executable)
        );
    }
}
