use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

// PR metadata display is intentionally capped to the first 100 open PRs.
const GH_PR_LIST_LIMIT: &str = "100";

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

    let git_common_dir = String::from_utf8(output.stdout)?.trim().to_string();

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
    let messages = crate::i18n::Messages::new();
    let output = Command::new("git")
        .arg("worktree")
        .arg("list")
        .arg("--porcelain")
        .current_dir(repo_root)
        .output()
        .context(messages.failed_list_worktrees().to_string())?;

    if !output.status.success() {
        anyhow::bail!("{}", messages.failed_list_worktrees());
    }

    let stdout = String::from_utf8(output.stdout)?;
    parse_worktree_list(&stdout, repo_root)
}

#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    pub path: PathBuf,
    branch: Option<String>,
    name: String,
    pub is_main: bool,
}

impl WorktreeInfo {
    pub fn branch_name(&self) -> Option<&str> {
        self.branch.as_deref()
    }

    pub fn matches_name(&self, candidate: &str) -> bool {
        self.name.eq_ignore_ascii_case(candidate)
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Clone)]
pub struct PullRequestInfo {
    pub number: u64,
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct RepositorySlug {
    pub host: String,
    pub owner: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
struct RepositoryOwnerResponse {
    login: String,
}

#[derive(Debug, Deserialize)]
struct RepositoryResponse {
    name: String,
}

#[derive(Debug, Deserialize)]
struct PullRequestResponse {
    #[serde(rename = "headRefName")]
    head_ref_name: String,
    #[serde(rename = "headRepository")]
    head_repository: Option<RepositoryResponse>,
    #[serde(rename = "headRepositoryOwner")]
    head_repository_owner: Option<RepositoryOwnerResponse>,
    #[serde(rename = "isCrossRepository")]
    is_cross_repository: bool,
    number: u64,
    title: String,
}

fn same_worktree_path(lhs: &Path, rhs: &Path) -> bool {
    if lhs == rhs {
        return true;
    }

    match (lhs.canonicalize(), rhs.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

fn detached_worktree_name(head: &str) -> String {
    let short_head: String = head.chars().take(7).collect();
    format!("detached@{short_head}")
}

fn build_worktree_info(
    path: PathBuf,
    branch: Option<String>,
    head: Option<String>,
    detached: bool,
    is_main: bool,
    repo_root: &Path,
) -> Option<WorktreeInfo> {
    let name = match branch.as_deref() {
        Some(branch_name) => branch_name.to_string(),
        None if detached => detached_worktree_name(head.as_deref()?),
        None => return None,
    };

    Some(WorktreeInfo {
        is_main: is_main || same_worktree_path(&path, repo_root),
        path,
        branch,
        name,
    })
}

fn parse_worktree_list(output: &str, repo_root: &Path) -> Result<Vec<WorktreeInfo>> {
    let mut worktrees = Vec::new();
    let mut current_path: Option<PathBuf> = None;
    let mut current_branch: Option<String> = None;
    let mut current_head: Option<String> = None;
    let mut is_detached = false;
    let mut is_main = false;

    for line in output.lines() {
        if line.starts_with("worktree ") {
            // Save previous worktree if exists
            if let Some(path) = current_path.take() {
                if let Some(worktree) = build_worktree_info(
                    path,
                    current_branch.take(),
                    current_head.take(),
                    is_detached,
                    is_main,
                    repo_root,
                ) {
                    worktrees.push(worktree);
                }
            }

            current_path = Some(PathBuf::from(line.trim_start_matches("worktree ")));
            current_branch = None;
            current_head = None;
            is_detached = false;
            is_main = false;
        } else if line.starts_with("HEAD ") {
            current_head = Some(line.trim_start_matches("HEAD ").to_string());
        } else if line.starts_with("branch ") {
            let branch = line.trim_start_matches("branch ");
            current_branch = Some(branch.trim_start_matches("refs/heads/").to_string());
        } else if line == "detached" {
            is_detached = true;
        } else if line.starts_with("bare") {
            is_main = true;
        } else if line.is_empty() {
            // End of worktree entry
            if let Some(path) = current_path.take() {
                if let Some(worktree) = build_worktree_info(
                    path,
                    current_branch.take(),
                    current_head.take(),
                    is_detached,
                    is_main,
                    repo_root,
                ) {
                    worktrees.push(worktree);
                }
            }

            is_detached = false;
            is_main = false;
        }
    }

    // Save last worktree if exists
    if let Some(path) = current_path {
        if let Some(worktree) = build_worktree_info(
            path,
            current_branch,
            current_head,
            is_detached,
            is_main,
            repo_root,
        ) {
            worktrees.push(worktree);
        }
    }

    Ok(worktrees)
}

pub fn get_repository_slug(repo_root: &Path) -> Result<Option<RepositorySlug>> {
    let output = Command::new("git")
        .arg("remote")
        .arg("get-url")
        .arg("origin")
        .current_dir(repo_root)
        .output()?;

    if !output.status.success() {
        return Ok(None);
    }

    let remote_url = String::from_utf8(output.stdout)?;
    Ok(parse_repository_slug(remote_url.trim()))
}

fn parse_repository_slug(remote_url: &str) -> Option<RepositorySlug> {
    let remote_url = remote_url.trim_end_matches(".git");
    let (host, path) = if let Some(rest) = remote_url.strip_prefix("git@") {
        let (host, path) = rest.split_once(':')?;
        (host, path)
    } else if let Some((_, rest)) = remote_url.split_once("://") {
        let (host, path) = rest.split_once('/')?;
        (host, path)
    } else {
        return None;
    };

    let mut parts = path.split('/').filter(|segment| !segment.is_empty()).rev();
    let name = parts.next()?.to_string();
    let owner = parts.next()?.to_string();
    let canonical_host = canonicalize_remote_host(host);

    Some(RepositorySlug {
        host: canonical_host,
        owner,
        name,
    })
}

fn canonicalize_remote_host(host: &str) -> String {
    let without_user = host.rsplit('@').next().unwrap_or(host);
    let normalized = without_user.to_ascii_lowercase();

    if normalized.starts_with('[') {
        return normalized;
    }

    if let Some((hostname, port)) = normalized.rsplit_once(':') {
        if !hostname.contains(':') && port.chars().all(|character| character.is_ascii_digit()) {
            return hostname.to_string();
        }
    }

    normalized
}

fn is_gh_available() -> bool {
    Command::new("gh")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn is_same_repository_pr(pull_request: &PullRequestResponse, repository: &RepositorySlug) -> bool {
    if pull_request.is_cross_repository {
        return false;
    }

    let Some(head_repository) = pull_request.head_repository.as_ref() else {
        return false;
    };
    let Some(head_repository_owner) = pull_request.head_repository_owner.as_ref() else {
        return false;
    };

    head_repository.name == repository.name
        && head_repository_owner
            .login
            .eq_ignore_ascii_case(&repository.owner)
}

pub fn get_open_prs_by_branch(repo_root: &Path) -> Result<HashMap<String, PullRequestInfo>> {
    if !is_gh_available() {
        return Ok(HashMap::new());
    }
    let Some(repository) = get_repository_slug(repo_root)? else {
        return Ok(HashMap::new());
    };

    let output = Command::new("gh")
        .arg("pr")
        .arg("list")
        .arg("--json")
        .arg("number,title,headRefName,headRepository,headRepositoryOwner,isCrossRepository")
        .arg("--state")
        .arg("open")
        .arg("--limit")
        .arg(GH_PR_LIST_LIMIT)
        .current_dir(repo_root)
        .output()
        .context("Failed to execute gh command")?;

    if !output.status.success() {
        return Ok(HashMap::new());
    }

    let stdout = String::from_utf8(output.stdout)?;
    let parsed: Vec<PullRequestResponse> = serde_json::from_str(&stdout)?;
    let pull_requests = parsed
        .into_iter()
        .filter(|pull_request| is_same_repository_pr(pull_request, &repository))
        .map(|pull_request| {
            (
                pull_request.head_ref_name,
                PullRequestInfo {
                    number: pull_request.number,
                    title: pull_request.title,
                },
            )
        })
        .collect();

    Ok(pull_requests)
}

pub fn resolve_merge_base_branch(repo_root: &Path, explicit_base: Option<&str>) -> Result<String> {
    if let Some(base) = explicit_base {
        return Ok(base.to_string());
    }

    let output = Command::new("git")
        .arg("symbolic-ref")
        .arg("--quiet")
        .arg("--short")
        .arg("refs/remotes/origin/HEAD")
        .current_dir(repo_root)
        .output()?;

    if output.status.success() {
        let branch = String::from_utf8(output.stdout)?.trim().to_string();
        if !branch.is_empty() {
            return Ok(branch);
        }
    }

    for candidate in ["origin/main", "origin/master", "main", "master"] {
        if git_ref_exists(repo_root, candidate)? {
            return Ok(candidate.to_string());
        }
    }

    anyhow::bail!("Failed to resolve default base branch for merged cleanup")
}

fn git_ref_exists(repo_root: &Path, reference: &str) -> Result<bool> {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--verify")
        .arg("--quiet")
        .arg(reference)
        .current_dir(repo_root)
        .output()?;

    Ok(output.status.success())
}

pub fn is_branch_merged_into(repo_root: &Path, branch: &str, base: &str) -> Result<bool> {
    let output = Command::new("git")
        .arg("merge-base")
        .arg("--is-ancestor")
        .arg(branch)
        .arg(base)
        .current_dir(repo_root)
        .output()?;

    if output.status.success() {
        return Ok(true);
    }

    if output.status.code() == Some(1) {
        return Ok(false);
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    anyhow::bail!(
        "Failed to check merge status for '{}' against '{}': {}",
        branch,
        base,
        stderr
    );
}

pub fn worktree_target_exists(repo_root: &Path, target: &str) -> Result<bool> {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--verify")
        .arg("--quiet")
        .arg(format!("{target}^{{commit}}"))
        .current_dir(repo_root)
        .output()?;

    Ok(output.status.success())
}

pub fn run_command_in_dir(worktree_path: &Path, command: &[String]) -> Result<ExitStatus> {
    if command.is_empty() {
        anyhow::bail!("No command provided");
    }

    #[cfg(windows)]
    {
        let executable = crate::process::resolve_executable(command[0].as_ref())
            .ok_or_else(|| anyhow::anyhow!("Command not found: '{}'", command[0]))?;
        return Command::new(executable)
            .args(&command[1..])
            .current_dir(worktree_path)
            .status()
            .with_context(|| format!("Failed to run command '{}'", command[0]));
    }

    #[cfg(not(windows))]
    {
        Command::new(&command[0])
            .args(&command[1..])
            .current_dir(worktree_path)
            .status()
            .with_context(|| format!("Failed to run command '{}'", command[0]))
    }
}

/// Add a new worktree
pub fn add_worktree(
    repo_root: &Path,
    worktree_path: &Path,
    branch: &str,
    create_branch: bool,
) -> Result<()> {
    let messages = crate::i18n::Messages::new();
    let mut cmd = Command::new("git");
    cmd.arg("worktree").arg("add");

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
        anyhow::bail!("{}: {}", messages.failed_add_worktree(), stderr);
    }

    Ok(())
}

/// Remove a worktree
/// Returns an error if the worktree has uncommitted changes
pub fn remove_worktree(repo_root: &Path, worktree_path: &Path, force: bool) -> Result<()> {
    let messages = crate::i18n::Messages::new();
    let mut cmd = Command::new("git");

    cmd.arg("worktree").arg("remove");

    if force {
        cmd.arg("-f");
    }

    let output = cmd.arg(worktree_path).current_dir(repo_root).output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("{}: {}", messages.failed_remove_worktree(), stderr);
    }

    Ok(())
}

/// Prune stale worktree administrative files
pub fn prune_worktrees(repo_root: &Path) -> Result<()> {
    let output = Command::new("git")
        .arg("worktree")
        .arg("prune")
        .current_dir(repo_root)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git worktree prune failed: {}", stderr);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    use super::run_command_in_dir;
    use super::{list_worktrees, parse_worktree_list, worktree_target_exists, WorktreeInfo};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn names(worktrees: &[WorktreeInfo]) -> Vec<String> {
        worktrees
            .iter()
            .map(|worktree| worktree.name().to_string())
            .collect()
    }

    fn make_temp_dir(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("wt-manager-{prefix}-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn git(repo_root: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(repo_root)
            .env("GIT_AUTHOR_NAME", "wt-manager")
            .env("GIT_AUTHOR_EMAIL", "wt-manager@example.com")
            .env("GIT_COMMITTER_NAME", "wt-manager")
            .env("GIT_COMMITTER_EMAIL", "wt-manager@example.com")
            .status()
            .unwrap();
        assert!(status.success(), "git command failed: {:?}", args);
    }

    fn init_repo() -> PathBuf {
        let repo_root = make_temp_dir("repo");
        git(&repo_root, &["init", "-b", "main"]);
        fs::write(repo_root.join("README.md"), "hello\n").unwrap();
        git(&repo_root, &["add", "README.md"]);
        git(&repo_root, &["commit", "-m", "init"]);
        repo_root
    }

    #[test]
    fn parse_worktree_list_keeps_branch_worktrees() {
        let output = "worktree /repo\nHEAD 1111111111111111111111111111111111111111\nbranch refs/heads/main\n\nworktree /wt/feature\nHEAD 2222222222222222222222222222222222222222\nbranch refs/heads/feature\n\n";

        let worktrees = parse_worktree_list(output, Path::new("/repo")).unwrap();

        assert_eq!(names(&worktrees), vec!["main", "feature"]);
        assert_eq!(worktrees[0].branch_name(), Some("main"));
        assert_eq!(worktrees[1].branch_name(), Some("feature"));
    }

    #[test]
    fn parse_worktree_list_keeps_detached_worktrees() {
        let output = "worktree /repo\nHEAD 1111111111111111111111111111111111111111\nbranch refs/heads/main\n\nworktree /wt/detached\nHEAD abcdef1234567890abcdef1234567890abcdef12\ndetached\n\n";

        let worktrees = parse_worktree_list(output, Path::new("/repo")).unwrap();

        assert_eq!(names(&worktrees), vec!["main", "detached@abcdef1"]);
        assert_eq!(worktrees[1].branch_name(), None);
        assert!(worktrees[1].matches_name("detached@abcdef1"));
    }

    #[test]
    fn worktree_target_exists_checks_refs_before_creation() {
        let repo_root = init_repo();
        git(&repo_root, &["branch", "feature"]);

        assert!(worktree_target_exists(&repo_root, "feature").unwrap());
        assert!(!worktree_target_exists(&repo_root, "missing-branch").unwrap());

        fs::remove_dir_all(repo_root).unwrap();
    }

    #[test]
    fn list_worktrees_includes_live_detached_worktrees() {
        let repo_root = init_repo();
        let worktree_path = make_temp_dir("detached-worktree");
        git(
            &repo_root,
            &[
                "worktree",
                "add",
                "--detach",
                worktree_path.to_str().unwrap(),
                "HEAD",
            ],
        );

        let worktrees = list_worktrees(&repo_root).unwrap();
        let expected_path = worktree_path.canonicalize().unwrap();
        let detached = worktrees
            .iter()
            .find(|worktree| worktree.path.canonicalize().unwrap() == expected_path)
            .unwrap();

        assert_eq!(detached.branch_name(), None);
        assert!(detached.name().starts_with("detached@"));

        fs::remove_dir_all(worktree_path).unwrap();
        fs::remove_dir_all(repo_root).unwrap();
    }
    #[cfg(windows)]
    #[test]
    fn run_command_in_dir_executes_cmd_with_original_arguments() {
        let worktree_path = make_temp_dir("cmd-worktree");
        let script_path = worktree_path.join("argv-probe.cmd");
        fs::write(
            &script_path,
            concat!(
                "@echo off\r\n",
                "if not \"%~1\"==\"alpha\" exit /b 11\r\n",
                "if not \"%~2\"==\"two words\" exit /b 12\r\n",
                "if not \"%~3\"==\"--flag=value\" exit /b 13\r\n",
                "exit /b 0\r\n",
            ),
        )
        .unwrap();
        let command = vec![
            script_path
                .with_extension("")
                .to_string_lossy()
                .into_owned(),
            "alpha".to_string(),
            "two words".to_string(),
            "--flag=value".to_string(),
        ];

        let status = run_command_in_dir(&worktree_path, &command).unwrap();

        assert!(status.success(), "batch command exited with {status}");
        fs::remove_dir_all(worktree_path).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn run_command_in_dir_reports_missing_command_name() {
        let worktree_path = make_temp_dir("missing-command");
        let command_name = "wt-manager-command-that-does-not-exist";

        let error = run_command_in_dir(&worktree_path, &[command_name.to_string()]).unwrap_err();

        assert!(
            error.to_string().contains(command_name),
            "missing command error did not include its name: {error:#}"
        );
        fs::remove_dir_all(worktree_path).unwrap();
    }
}
