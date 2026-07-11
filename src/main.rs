mod db;
mod git;
mod i18n;
mod process;
mod setup;
mod tui;
mod worktree;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

pub(crate) const SHELL_CD_MARKER_PREFIX: &str = "__WT_MANAGER_CD__=";
const SHELL_CD_MARKER_FILE_ENV: &str = "WT_MANAGER_CD_MARKER_FILE";

pub(crate) fn shell_cd_marker_line(path: &Path) -> String {
    format!("{}{}", SHELL_CD_MARKER_PREFIX, path.display())
}

pub(crate) fn maybe_print_shell_cd_marker(path: &Path) {
    if env::var_os("WT_MANAGER_CAPTURE_CD").is_none() {
        return;
    }

    let marker = shell_cd_marker_line(path);
    if let Some(marker_file) = env::var_os(SHELL_CD_MARKER_FILE_ENV) {
        let _ = fs::write(marker_file, format!("{marker}\n"));
    } else {
        println!("{marker}");
    }
}

#[cfg(windows)]
pub(crate) fn print_cd_command(path: &Path) {
    println!(
        "  Set-Location -LiteralPath {}",
        powershell_single_quoted_literal(path)
    );
}

#[cfg(unix)]
pub(crate) fn print_cd_command(path: &Path) {
    println!("  cd {}", path.display());
}

#[derive(Parser, Debug)]
#[command(name = "wt")]
#[command(
    version,
    about = cmd_about(),
    long_about = long_help(),
    after_help = tui_help()
)]
struct Args {
    #[arg(value_name = "BRANCH", required = false, help = arg_branch_help())]
    branch: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

fn cmd_about() -> &'static str {
    i18n::Messages::new().cmd_about()
}

fn long_help() -> &'static str {
    i18n::Messages::new().long_help()
}

fn tui_help() -> &'static str {
    i18n::Messages::new().tui_help()
}

fn arg_branch_help() -> &'static str {
    i18n::Messages::new().arg_branch_help()
}

fn main() -> Result<()> {
    let messages = i18n::Messages::new();
    // Set up Ctrl+C handler
    ctrlc::set_handler(|| {
        // Clean up terminal state if needed
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
        std::process::exit(0);
    })
    .unwrap_or_else(|_| panic!("{}", messages.ctrlc_handler_error()));

    let args = Args::parse();
    let current_dir = env::current_dir()?;

    match (args.branch, args.command) {
        (Some(branch), None) => handle_legacy_branch(&branch, &current_dir)?,
        (None, Some(command)) => handle_command(command, &current_dir)?,
        (None, None) => handle_default(&current_dir)?,
        (Some(branch), Some(_)) => {
            // Keep branch argument as legacy mode to avoid ambiguity for named commands.
            // This path is intentionally unreachable in normal clap parsing, but kept as a guard.
            handle_legacy_branch(&branch, &current_dir)?;
        }
    }

    Ok(())
}

fn get_repo_root_or_project_tui(current_dir: &Path) -> Result<Option<PathBuf>> {
    // Save current project if this is a git repo
    let repo_root = git::find_main_repo_root(current_dir)?;
    if let Some(ref root) = repo_root {
        // Keep project list updated whenever running inside a git repo.
        db::save_project(root)?;
    }
    Ok(repo_root)
}

fn handle_legacy_branch(branch: &str, current_dir: &Path) -> Result<()> {
    if let Some(repo_root) = get_repo_root_or_project_tui(current_dir)? {
        worktree::handle_worktree(&repo_root, branch)?;
    } else {
        // Keep compatibility: when not in a git repo, fallback to project selector.
        tui::show_project_selector()?;
    }

    Ok(())
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = cmd_init_about())]
    Init {
        #[arg(long, value_name = "PATH", help = arg_profile_help())]
        profile: Option<PathBuf>,
    },
    #[command(about = cmd_tui_about())]
    Tui,
    #[command(hide = true)]
    Worktree {
        #[command(subcommand)]
        command: WorktreeAliasCommands,
    },
    #[command(about = cmd_list_about())]
    List,
    #[command(about = cmd_cd_about())]
    Cd { branch: String },
    #[command(about = cmd_run_about())]
    Run {
        branch: String,
        #[arg(required = true, num_args = 1.., trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
    #[command(about = cmd_delete_about())]
    Delete {
        branch: String,
        #[arg(
            short,
            long,
            help = arg_force_delete_help()
        )]
        force: bool,
    },
    #[command(about = cmd_clean_about())]
    Clean {
        #[arg(short, long, help = arg_dry_run_help())]
        dry_run: bool,
        #[arg(short, long, help = arg_force_clean_help())]
        force: bool,
        #[arg(long, help = arg_include_untracked_help())]
        include_untracked: bool,
        #[arg(short, long, help = arg_remote_help())]
        remote: Option<String>,
        #[arg(long, help = arg_skip_fetch_help())]
        skip_fetch: bool,
        #[arg(long, help = arg_merged_help())]
        merged: bool,
        #[arg(long, help = arg_base_help())]
        base: Option<String>,
    },
    #[command(about = cmd_project_about())]
    Project {
        #[command(subcommand)]
        command: ProjectCommands,
    },
    #[command(about = cmd_config_about())]
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
}

fn cmd_init_about() -> &'static str {
    i18n::Messages::new().cmd_init_about()
}
fn arg_profile_help() -> &'static str {
    i18n::Messages::new().arg_profile_help()
}
fn cmd_tui_about() -> &'static str {
    i18n::Messages::new().cmd_tui_about()
}
fn cmd_list_about() -> &'static str {
    i18n::Messages::new().cmd_list_about()
}
fn cmd_cd_about() -> &'static str {
    i18n::Messages::new().cmd_cd_about()
}
fn cmd_run_about() -> &'static str {
    i18n::Messages::new().cmd_run_about()
}
fn cmd_delete_about() -> &'static str {
    i18n::Messages::new().cmd_delete_about()
}
fn arg_force_delete_help() -> &'static str {
    i18n::Messages::new().arg_force_delete_help()
}
fn cmd_clean_about() -> &'static str {
    i18n::Messages::new().cmd_clean_about()
}
fn arg_dry_run_help() -> &'static str {
    i18n::Messages::new().arg_dry_run_help()
}
fn arg_force_clean_help() -> &'static str {
    i18n::Messages::new().arg_force_clean_help()
}
fn arg_include_untracked_help() -> &'static str {
    i18n::Messages::new().arg_include_untracked_help()
}
fn arg_remote_help() -> &'static str {
    i18n::Messages::new().arg_remote_help()
}
fn arg_skip_fetch_help() -> &'static str {
    i18n::Messages::new().arg_skip_fetch_help()
}
fn arg_merged_help() -> &'static str {
    i18n::Messages::new().arg_merged_help()
}
fn arg_base_help() -> &'static str {
    i18n::Messages::new().arg_base_help()
}
fn cmd_project_about() -> &'static str {
    i18n::Messages::new().cmd_project_about()
}
fn cmd_config_about() -> &'static str {
    i18n::Messages::new().cmd_config_about()
}

#[derive(Subcommand, Debug)]
enum WorktreeAliasCommands {
    List,
    Switch {
        branch: String,
    },
    Run {
        branch: String,
        #[arg(required = true, num_args = 1.., trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
    Delete {
        branch: String,
        #[arg(short, long)]
        force: bool,
    },
    Clean {
        #[arg(short, long)]
        dry_run: bool,
        #[arg(short, long)]
        force: bool,
        #[arg(long)]
        include_untracked: bool,
        #[arg(short, long)]
        remote: Option<String>,
        #[arg(long)]
        skip_fetch: bool,
        #[arg(long)]
        merged: bool,
        #[arg(long)]
        base: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum ProjectCommands {
    #[command(about = cmd_project_list_about())]
    List,
}

fn cmd_project_list_about() -> &'static str {
    i18n::Messages::new().cmd_project_list_about()
}

#[derive(Subcommand, Debug)]
enum ConfigCommands {
    Show,
    Copy {
        #[command(subcommand)]
        command: CopyCommands,
    },
    Hook {
        #[command(subcommand)]
        command: HookCommands,
    },
}

#[derive(Subcommand, Debug)]
enum CopyCommands {
    Add { path: String },
    Remove { path: String },
}

#[derive(Subcommand, Debug)]
enum HookCommands {
    Add { hook: HookKind, command: String },
    Remove { hook: HookKind, index: usize },
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum HookKind {
    #[value(name = "post-create")]
    PostCreate,
    #[value(name = "post-cd")]
    PostCd,
}

struct CleanCommandOptions<'a> {
    dry_run: bool,
    force: bool,
    include_untracked: bool,
    remote: Option<&'a str>,
    skip_fetch: bool,
    merged: bool,
    base: Option<&'a str>,
}

fn handle_command(cmd: Commands, current_dir: &Path) -> Result<()> {
    match cmd {
        Commands::Init { profile } => init_shell_integration(profile),
        Commands::Tui => handle_default(current_dir),
        Commands::Worktree { command } => handle_worktree_alias_command(command, current_dir),
        Commands::List => handle_list_command(current_dir),
        Commands::Cd { branch } => handle_cd_command(current_dir, &branch),
        Commands::Run { branch, command } => handle_run_command(current_dir, &branch, &command),
        Commands::Delete { branch, force } => handle_delete_command(current_dir, &branch, force),
        Commands::Clean {
            dry_run,
            force,
            include_untracked,
            remote,
            skip_fetch,
            merged,
            base,
        } => handle_clean_command(
            current_dir,
            CleanCommandOptions {
                dry_run,
                force,
                include_untracked,
                remote: remote.as_deref(),
                skip_fetch,
                merged,
                base: base.as_deref(),
            },
        ),
        Commands::Project { command } => match command {
            ProjectCommands::List => list_projects(),
        },
        Commands::Config { command } => handle_config_command(command, current_dir),
    }
}

fn handle_worktree_alias_command(command: WorktreeAliasCommands, current_dir: &Path) -> Result<()> {
    match command {
        WorktreeAliasCommands::List => handle_list_command(current_dir),
        WorktreeAliasCommands::Switch { branch } => handle_cd_command(current_dir, &branch),
        WorktreeAliasCommands::Run { branch, command } => {
            handle_run_command(current_dir, &branch, &command)
        }
        WorktreeAliasCommands::Delete { branch, force } => {
            handle_delete_command(current_dir, &branch, force)
        }
        WorktreeAliasCommands::Clean {
            dry_run,
            force,
            include_untracked,
            remote,
            skip_fetch,
            merged,
            base,
        } => handle_clean_command(
            current_dir,
            CleanCommandOptions {
                dry_run,
                force,
                include_untracked,
                remote: remote.as_deref(),
                skip_fetch,
                merged,
                base: base.as_deref(),
            },
        ),
    }
}

fn handle_default(current_dir: &Path) -> Result<()> {
    // Check if we're in a git repository (use main repo root to handle worktrees)
    if let Some(repo_root) = get_repo_root_or_project_tui(current_dir)? {
        tui::show_worktree_selector(&repo_root)?;
    } else {
        // Not in a git repo - show TUI to select from saved projects
        tui::show_project_selector()?;
    }

    Ok(())
}

fn handle_list_command(current_dir: &Path) -> Result<()> {
    let messages = i18n::Messages::new();
    let repo_root = match get_repo_root_or_project_tui(current_dir)? {
        Some(root) => root,
        None => {
            anyhow::bail!("{}", messages.cmd_requires_repo());
        }
    };

    list_worktrees(&repo_root)
}

fn handle_cd_command(current_dir: &Path, branch: &str) -> Result<()> {
    let messages = i18n::Messages::new();
    let repo_root = match get_repo_root_or_project_tui(current_dir)? {
        Some(root) => root,
        None => anyhow::bail!("{}", messages.cmd_requires_repo()),
    };

    worktree::handle_worktree(&repo_root, branch)
}

fn handle_run_command(current_dir: &Path, branch: &str, command: &[String]) -> Result<()> {
    let messages = i18n::Messages::new();
    let repo_root = match get_repo_root_or_project_tui(current_dir)? {
        Some(root) => root,
        None => anyhow::bail!("{}", messages.cmd_requires_repo()),
    };

    run_worktree_command(&repo_root, branch, command)
}

fn handle_delete_command(current_dir: &Path, branch: &str, force: bool) -> Result<()> {
    let messages = i18n::Messages::new();
    let repo_root = match get_repo_root_or_project_tui(current_dir)? {
        Some(root) => root,
        None => anyhow::bail!("{}", messages.cmd_requires_repo()),
    };

    let worktrees = git::list_worktrees(&repo_root)?;
    let target = worktrees.iter().find(|wt| wt.matches_name(branch));

    let Some(wt) = target else {
        anyhow::bail!("{}", messages.cannot_find_worktree().replace("{}", branch));
    };

    if wt.is_main {
        anyhow::bail!("{}", messages.cannot_delete_main());
    }

    println!("\n{} {}", messages.deleting_worktree(), wt.name());
    match git::remove_worktree(&repo_root, &wt.path, force) {
        Ok(()) => {
            println!("{}", messages.worktree_deleted().replace("{}", wt.name()));
            let _ = git::prune_worktrees(&repo_root);
        }
        Err(error) => {
            if !force {
                eprintln!("\n{}", messages.uncommitted_changes_tip());
                eprintln!(
                    "{} wt delete {} --force",
                    messages.force_delete_command(),
                    wt.name()
                );
            }
            return Err(error);
        }
    }

    Ok(())
}

fn handle_clean_command(current_dir: &Path, options: CleanCommandOptions<'_>) -> Result<()> {
    let messages = i18n::Messages::new();
    let repo_root = match get_repo_root_or_project_tui(current_dir)? {
        Some(root) => root,
        None => anyhow::bail!("{}", messages.cmd_requires_repo()),
    };

    if options.merged {
        clean_merged_worktrees(&repo_root, options.dry_run, options.force, options.base)
    } else {
        clean_stale_worktrees(
            &repo_root,
            options.dry_run,
            options.force,
            options.include_untracked,
            options.remote,
            options.skip_fetch,
        )
    }
}

fn hook_kind_name(hook: &HookKind) -> &'static str {
    match hook {
        HookKind::PostCreate => "post-create",
        HookKind::PostCd => "post-cd",
    }
}

fn handle_config_command(command: ConfigCommands, current_dir: &Path) -> Result<()> {
    let messages = i18n::Messages::new();
    let repo_root = match get_repo_root_or_project_tui(current_dir)? {
        Some(root) => root,
        None => anyhow::bail!("{}", messages.cmd_requires_repo()),
    };

    match command {
        ConfigCommands::Show => {
            match db::get_project_automation(&repo_root)? {
                Some(config) => {
                    println!("{} {}", messages.config_show_header(), repo_root.display());
                    println!("{}", messages.config_copy_files_label());
                    for entry in config.copy_files {
                        println!("  - {}", entry);
                    }
                    println!("{}", messages.config_post_create_hooks_label());
                    for (index, entry) in config.post_create_hooks.iter().enumerate() {
                        println!("  [{}] {}", index, entry);
                    }
                    println!("{}", messages.config_post_cd_hooks_label());
                    for (index, entry) in config.post_cd_hooks.iter().enumerate() {
                        println!("  [{}] {}", index, entry);
                    }
                }
                None => println!("{}", messages.config_no_config_found()),
            }
            Ok(())
        }
        ConfigCommands::Copy { command } => match command {
            CopyCommands::Add { path } => {
                db::add_copy_file(&repo_root, &path)?;
                println!("{}", messages.config_added_copy_file().replace("{}", &path));
                Ok(())
            }
            CopyCommands::Remove { path } => {
                if db::remove_copy_file(&repo_root, &path)? {
                    println!(
                        "{}",
                        messages.config_removed_copy_file().replace("{}", &path)
                    );
                } else {
                    println!(
                        "{}",
                        messages
                            .config_copy_file_not_configured()
                            .replace("{}", &path)
                    );
                }
                Ok(())
            }
        },
        ConfigCommands::Hook { command } => match command {
            HookCommands::Add { hook, command } => {
                db::add_hook(&repo_root, hook_kind_name(&hook), &command)?;
                println!(
                    "{}",
                    messages
                        .config_added_hook()
                        .replacen("{}", hook_kind_name(&hook), 1)
                        .replacen("{}", &command, 1)
                );
                Ok(())
            }
            HookCommands::Remove { hook, index } => {
                if db::remove_hook(&repo_root, hook_kind_name(&hook), index)? {
                    println!(
                        "{}",
                        messages
                            .config_removed_hook_at_index()
                            .replacen("{}", hook_kind_name(&hook), 1)
                            .replacen("{}", &index.to_string(), 1)
                    );
                } else {
                    println!(
                        "{}",
                        messages
                            .config_no_hook_found_at_index()
                            .replacen("{}", hook_kind_name(&hook), 1)
                            .replacen("{}", &index.to_string(), 1)
                    );
                }
                Ok(())
            }
        },
    }
}

fn run_worktree_command(repo_root: &Path, branch: &str, command: &[String]) -> Result<()> {
    let prepared = worktree::prepare_worktree(repo_root, branch)?;
    if !prepared.existed {
        setup::SetupManager::run_post_create(repo_root, &prepared.path)?;
    }
    setup::SetupManager::run_post_cd(repo_root, &prepared.path)?;
    setup::SetupManager::run_auto_setup(&prepared.path)?;

    let status = git::run_command_in_dir(&prepared.path, command)?;
    std::process::exit(status.code().unwrap_or(1));
}

fn list_worktrees(repo_root: &Path) -> Result<()> {
    let messages = i18n::Messages::new();
    let worktrees = git::list_worktrees(repo_root)?;
    let pull_requests = git::get_open_prs_by_branch(repo_root)?;

    println!("\n{}", messages.select_or_create_worktree());
    for wt in worktrees {
        let marker = if wt.is_main {
            messages.main_marker()
        } else {
            ""
        };
        if let Some(pr) = wt
            .branch_name()
            .and_then(|branch| pull_requests.get(branch))
        {
            println!("  {}{} #{} {}", wt.name(), marker, pr.number, pr.title);
        } else {
            println!("  {}{}", wt.name(), marker);
        }
        println!("    {}: {}", messages.path_label(), wt.path.display());
    }

    Ok(())
}

fn collect_branch_upstreams(repo_root: &Path) -> Result<HashMap<String, Option<String>>> {
    let messages = i18n::Messages::new();
    let output = Command::new("git")
        .arg("for-each-ref")
        .arg("--format=%(refname:short) %(upstream:short)")
        .arg("refs/heads")
        .current_dir(repo_root)
        .output()?;

    if !output.status.success() {
        anyhow::bail!("{}", messages.failed_collect_upstream());
    }

    let stdout = String::from_utf8(output.stdout)?;
    let mut result = HashMap::new();

    for line in stdout.lines() {
        let mut parts = line.splitn(2, ' ');
        let branch = match parts.next() {
            Some(name) if !name.is_empty() => name.to_string(),
            _ => continue,
        };
        let upstream = match parts.next() {
            Some(name) if !name.is_empty() => Some(name.to_string()),
            _ => None,
        };
        result.insert(branch, upstream);
    }

    Ok(result)
}

fn remote_ref_exists(repo_root: &Path, upstream: &str) -> Result<bool> {
    let messages = i18n::Messages::new();
    let status = Command::new("git")
        .arg("show-ref")
        .arg("--verify")
        .arg("--quiet")
        .arg(format!("refs/remotes/{upstream}"))
        .current_dir(repo_root)
        .output()?;

    if status.status.success() {
        return Ok(true);
    }

    if let Some(code) = status.status.code() {
        if code == 1 {
            return Ok(false);
        }

        let stderr = String::from_utf8(status.stderr)?;
        let reason = messages.failed_verify_remote_ref().replace("{}", upstream);
        anyhow::bail!("{} {}", reason, stderr.trim());
    }

    let reason = messages
        .failed_verify_remote_ref_interrupted()
        .replace("{}", upstream);
    anyhow::bail!("{}", reason);
}

fn fetch_prune_remote(repo_root: &Path, remote: Option<&str>) -> Result<()> {
    let messages = i18n::Messages::new();
    let mut cmd = Command::new("git");
    cmd.arg("fetch").arg("--prune");

    if let Some(remote_name) = remote {
        cmd.arg(remote_name);
    } else {
        cmd.arg("--all");
    }

    let output = cmd.current_dir(repo_root).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr)?;
        let reason = messages.failed_fetch_prune().replace("{}", stderr.trim());
        anyhow::bail!("{}", reason);
    }

    Ok(())
}

fn clean_stale_worktrees(
    repo_root: &Path,
    dry_run: bool,
    force: bool,
    include_untracked: bool,
    remote: Option<&str>,
    skip_fetch: bool,
) -> Result<()> {
    let messages = i18n::Messages::new();
    if !skip_fetch {
        fetch_prune_remote(repo_root, remote)?;
    }

    let branches = collect_branch_upstreams(repo_root)?;
    let worktrees = git::list_worktrees(repo_root)?;

    let mut stale_worktrees = Vec::new();
    for wt in worktrees.into_iter().filter(|wt| !wt.is_main) {
        let Some(branch) = wt.branch_name().map(|branch| branch.to_string()) else {
            continue;
        };
        let upstream = branches.get(&branch).cloned().flatten();

        match upstream {
            Some(value) => {
                if let Some(remote_name) = remote {
                    if !value.starts_with(&format!("{remote_name}/")) {
                        continue;
                    }
                }

                if !remote_ref_exists(repo_root, &value)? {
                    stale_worktrees.push((
                        wt,
                        messages
                            .stale_upstream_missing_reason()
                            .replace("{}", &value),
                    ));
                }
            }
            None => {
                if include_untracked {
                    stale_worktrees.push((wt, messages.no_upstream_reason().to_string()));
                }
            }
        }
    }

    if stale_worktrees.is_empty() {
        println!("{}", messages.no_stale_worktrees_found());
        return Ok(());
    }

    println!(
        "{}",
        messages
            .found_stale_worktrees()
            .replace("{}", &stale_worktrees.len().to_string())
    );
    for (wt, reason) in &stale_worktrees {
        println!("  {} ({})", wt.name(), reason);
        println!("    {}", wt.path.display());
    }

    if dry_run {
        println!("\n{}", messages.dry_run_enabled());
        return Ok(());
    }

    let mut failed_deletions = 0usize;
    for (wt, _) in stale_worktrees {
        println!("\n{} {}", messages.deleting_worktree(), wt.name());
        match git::remove_worktree(repo_root, &wt.path, force) {
            Ok(()) => {
                println!("{}", messages.worktree_deleted().replace("{}", wt.name()));
            }
            Err(e) => {
                failed_deletions += 1;
                eprintln!("\n{} {}", messages.failed_to_delete(), e);
                if !force {
                    eprintln!("\n{}", messages.uncommitted_changes_tip());
                    eprintln!(
                        "{} wt delete {} --force",
                        messages.force_delete_command(),
                        wt.name()
                    );
                }
            }
        }
    }

    let _ = git::prune_worktrees(repo_root);

    if failed_deletions > 0 {
        anyhow::bail!("Failed to delete {} stale worktree(s)", failed_deletions);
    }

    Ok(())
}

fn clean_merged_worktrees(
    repo_root: &Path,
    dry_run: bool,
    force: bool,
    base: Option<&str>,
) -> Result<()> {
    let messages = i18n::Messages::new();
    let base_branch = git::resolve_merge_base_branch(repo_root, base)?;
    let base_short = base_branch
        .rsplit('/')
        .next()
        .unwrap_or(&base_branch)
        .to_string();
    let worktrees = git::list_worktrees(repo_root)?;

    let mut merged_worktrees = Vec::new();
    for wt in worktrees.into_iter().filter(|wt| !wt.is_main) {
        let Some(branch) = wt.branch_name().map(|branch| branch.to_string()) else {
            continue;
        };

        if branch == base_branch || branch == base_short {
            continue;
        }
        if git::is_branch_merged_into(repo_root, &branch, &base_branch)? {
            merged_worktrees.push((wt, messages.merged_into().replace("{}", &base_branch)));
        }
    }

    if merged_worktrees.is_empty() {
        println!("{}", messages.no_merged_worktrees_found());
        return Ok(());
    }

    println!(
        "{}",
        messages
            .found_merged_worktrees()
            .replace("{}", &merged_worktrees.len().to_string())
    );
    for (wt, reason) in &merged_worktrees {
        println!("  {} ({})", wt.name(), reason);
        println!("    {}", wt.path.display());
    }

    if dry_run {
        println!("\n{}", messages.dry_run_enabled());
        return Ok(());
    }

    let mut failed_deletions = 0usize;
    for (wt, _) in merged_worktrees {
        println!("\n{} {}", messages.deleting_worktree(), wt.name());
        match git::remove_worktree(repo_root, &wt.path, force) {
            Ok(()) => {
                println!("{}", messages.worktree_deleted().replace("{}", wt.name()));
            }
            Err(error) => {
                failed_deletions += 1;
                eprintln!("\n{} {}", messages.failed_to_delete(), error);
                if !force {
                    eprintln!("\n{}", messages.uncommitted_changes_tip());
                    eprintln!(
                        "{} wt delete {} --force",
                        messages.force_delete_command(),
                        wt.name()
                    );
                }
            }
        }
    }

    let _ = git::prune_worktrees(repo_root);

    if failed_deletions > 0 {
        anyhow::bail!("Failed to delete {} merged worktree(s)", failed_deletions);
    }

    Ok(())
}

fn list_projects() -> Result<()> {
    let projects = db::get_projects()?;
    let messages = i18n::Messages::new();

    if projects.is_empty() {
        println!("{}", messages.no_projects_found());
        return Ok(());
    }

    println!("\n{}", messages.saved_projects_title());
    for project in projects {
        println!(
            "  {}",
            messages.list_items_item_path(&project.name, &project.path.display().to_string())
        );
    }

    Ok(())
}

#[cfg(unix)]
fn init_shell_integration(_profile: Option<PathBuf>) -> Result<()> {
    let messages = i18n::Messages::new();
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("{}", messages.failed_get_home_dir()))?;
    let shell_integration_path = home.join(".wt-manager.sh");
    let current_exe = env::current_exe()?;
    let current_exe = process::resolve_executable(current_exe.as_os_str()).unwrap_or(current_exe);
    let script = render_shell_integration_script(&current_exe);
    fs::write(&shell_integration_path, script)?;

    let rc_targets = [home.join(".zshrc"), home.join(".bashrc")];
    let mut updated_any = false;

    for rc_path in rc_targets {
        if !rc_path.exists() && rc_path.file_name().and_then(|name| name.to_str()) != Some(".zshrc")
        {
            continue;
        }

        let content = if rc_path.exists() {
            fs::read_to_string(&rc_path)?
        } else {
            String::new()
        };

        let mut lines: Vec<&str> = content.lines().collect();
        let original_len = lines.len();
        lines.retain(|line| {
            let trimmed = line.trim();
            trimmed != "# wt-manager shell integration"
                && trimmed != "source ~/.wt-manager.sh"
                && !trimmed.contains("wt-wrapper.sh")
        });

        let already_configured = original_len == lines.len()
            && content
                .lines()
                .any(|line| line.trim() == "source ~/.wt-manager.sh");

        if already_configured {
            println!(
                "{}",
                messages
                    .init_already_configured()
                    .replace("{}", &rc_path.display().to_string())
            );
            continue;
        }

        let mut rewritten = lines.join("\n");
        if !rewritten.ends_with('\n') && !rewritten.is_empty() {
            rewritten.push('\n');
        }
        rewritten.push_str("# wt-manager shell integration\nsource ~/.wt-manager.sh\n");
        fs::write(&rc_path, rewritten)?;
        println!(
            "{}",
            messages
                .init_updated()
                .replace("{}", &rc_path.display().to_string())
        );
        updated_any = true;
    }

    println!(
        "{}",
        messages
            .init_generated()
            .replace("{}", &shell_integration_path.display().to_string())
    );
    if !updated_any {
        println!("{}", messages.init_already_set_up());
    }

    Ok(())
}

#[cfg(windows)]
const POWERSHELL_PROFILE_BLOCK_START: &str = "# >>> wt-manager PowerShell integration >>>";
#[cfg(windows)]
const POWERSHELL_PROFILE_BLOCK_END: &str = "# <<< wt-manager PowerShell integration <<<";

#[cfg(windows)]
fn init_shell_integration(profile: Option<PathBuf>) -> Result<()> {
    let profile = profile.ok_or_else(|| {
        anyhow::anyhow!(
            "PowerShell integration requires an explicit profile path. Run: wt init --profile $PROFILE\ncmd.exe shell integration is not supported."
        )
    })?;
    let home = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to determine the home directory"))?;
    let current_exe = env::current_exe()?;
    let current_exe = process::resolve_executable(current_exe.as_os_str()).ok_or_else(|| {
        anyhow::anyhow!(
            "Failed to resolve the absolute wt.exe path: {}",
            current_exe.display()
        )
    })?;

    install_powershell_integration(&home, &profile, &current_exe)
}

#[cfg(windows)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProfileEncoding {
    Utf8Bom,
    Utf8,
    Utf16Le,
    Utf16Be,
}

#[cfg(windows)]
fn decode_powershell_profile(bytes: &[u8]) -> Result<(String, ProfileEncoding)> {
    if bytes.is_empty() {
        return Ok((String::new(), ProfileEncoding::Utf8Bom));
    }
    if let Some(payload) = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]) {
        return String::from_utf8(payload.to_vec())
            .map(|content| (content, ProfileEncoding::Utf8Bom))
            .map_err(|_| anyhow::anyhow!("PowerShell profile has an invalid UTF-8 BOM encoding"));
    }
    if bytes.starts_with(&[0xff, 0xfe, 0x00, 0x00]) || bytes.starts_with(&[0x00, 0x00, 0xfe, 0xff])
    {
        anyhow::bail!("Unsupported PowerShell profile encoding; UTF-32 profiles are not modified");
    }
    if let Some(payload) = bytes.strip_prefix(&[0xff, 0xfe]) {
        if payload.len() % 2 != 0 {
            anyhow::bail!("PowerShell profile has a truncated UTF-16LE encoding");
        }
        let units = payload
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect::<Vec<_>>();
        return String::from_utf16(&units)
            .map(|content| (content, ProfileEncoding::Utf16Le))
            .map_err(|_| anyhow::anyhow!("PowerShell profile has an invalid UTF-16LE encoding"));
    }
    if let Some(payload) = bytes.strip_prefix(&[0xfe, 0xff]) {
        if payload.len() % 2 != 0 {
            anyhow::bail!("PowerShell profile has a truncated UTF-16BE encoding");
        }
        let units = payload
            .chunks_exact(2)
            .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
            .collect::<Vec<_>>();
        return String::from_utf16(&units)
            .map(|content| (content, ProfileEncoding::Utf16Be))
            .map_err(|_| anyhow::anyhow!("PowerShell profile has an invalid UTF-16BE encoding"));
    }

    if bytes.contains(&0) {
        anyhow::bail!(
            "Unsupported PowerShell profile encoding; UTF-16 and UTF-32 profiles require a byte-order mark"
        );
    }

    String::from_utf8(bytes.to_vec())
        .map(|content| (content, ProfileEncoding::Utf8))
        .map_err(|_| {
            anyhow::anyhow!(
                "Unsupported PowerShell profile encoding; expected UTF-8, UTF-8 BOM, UTF-16LE BOM, or UTF-16BE BOM"
            )
        })
}

#[cfg(windows)]
fn encode_powershell_profile(content: &str, encoding: ProfileEncoding) -> Vec<u8> {
    match encoding {
        ProfileEncoding::Utf8Bom => {
            let mut bytes = Vec::with_capacity(content.len() + 3);
            bytes.extend_from_slice(&[0xef, 0xbb, 0xbf]);
            bytes.extend_from_slice(content.as_bytes());
            bytes
        }
        ProfileEncoding::Utf8 => content.as_bytes().to_vec(),
        ProfileEncoding::Utf16Le => {
            let mut bytes = vec![0xff, 0xfe];
            for unit in content.encode_utf16() {
                bytes.extend_from_slice(&unit.to_le_bytes());
            }
            bytes
        }
        ProfileEncoding::Utf16Be => {
            let mut bytes = vec![0xfe, 0xff];
            for unit in content.encode_utf16() {
                bytes.extend_from_slice(&unit.to_be_bytes());
            }
            bytes
        }
    }
}

#[cfg(windows)]
fn powershell_single_quoted_literal(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "''"))
}

#[cfg(windows)]
fn update_powershell_profile(content: &str, integration_path: &Path) -> Result<String> {
    let newline = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut rewritten = String::with_capacity(content.len() + 256);
    let mut inside_block = false;

    for line in content.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\r', '\n']).trim();
        if trimmed == POWERSHELL_PROFILE_BLOCK_START {
            if inside_block {
                anyhow::bail!("PowerShell profile contains a nested wt-manager integration block");
            }
            inside_block = true;
            continue;
        }
        if trimmed == POWERSHELL_PROFILE_BLOCK_END {
            if !inside_block {
                anyhow::bail!(
                    "PowerShell profile contains an unmatched wt-manager integration marker"
                );
            }
            inside_block = false;
            continue;
        }
        if !inside_block {
            rewritten.push_str(line);
        }
    }
    if inside_block {
        anyhow::bail!("PowerShell profile contains an incomplete wt-manager integration block");
    }

    if !rewritten.is_empty() && !rewritten.ends_with('\n') {
        rewritten.push_str(newline);
    }
    rewritten.push_str(POWERSHELL_PROFILE_BLOCK_START);
    rewritten.push_str(newline);
    rewritten.push_str(". ");
    rewritten.push_str(&powershell_single_quoted_literal(integration_path));
    rewritten.push_str(newline);
    rewritten.push_str(POWERSHELL_PROFILE_BLOCK_END);
    rewritten.push_str(newline);
    Ok(rewritten)
}

#[cfg(windows)]
fn write_utf8_bom(path: &Path, content: &str) -> Result<()> {
    let mut bytes = Vec::with_capacity(content.len() + 3);
    bytes.extend_from_slice(&[0xef, 0xbb, 0xbf]);
    bytes.extend_from_slice(content.as_bytes());
    fs::write(path, bytes)?;
    Ok(())
}

#[cfg(windows)]
fn install_powershell_integration(home: &Path, profile: &Path, current_exe: &Path) -> Result<()> {
    let profile_bytes = if profile.exists() {
        fs::read(profile)?
    } else {
        Vec::new()
    };
    let (profile_content, encoding) = decode_powershell_profile(&profile_bytes)?;
    let integration_path = home.join(".wt-manager.ps1");
    let rewritten_profile = update_powershell_profile(&profile_content, &integration_path)?;
    let script = render_powershell_integration_script(current_exe)?;

    fs::create_dir_all(home)?;
    if let Some(parent) = profile.parent() {
        fs::create_dir_all(parent)?;
    }
    write_utf8_bom(&integration_path, &script)?;

    let rewritten_bytes = encode_powershell_profile(&rewritten_profile, encoding);
    if rewritten_bytes != profile_bytes {
        fs::write(profile, rewritten_bytes)?;
        println!("Updated PowerShell profile: {}", profile.display());
    } else {
        println!(
            "PowerShell profile is already configured: {}",
            profile.display()
        );
    }
    println!(
        "Generated PowerShell integration: {}",
        integration_path.display()
    );
    Ok(())
}

#[cfg(windows)]
fn render_powershell_integration_script(current_exe: &Path) -> Result<String> {
    if !current_exe.is_absolute() {
        anyhow::bail!("PowerShell integration requires an absolute wt.exe path");
    }

    let template = r#"# wt-manager PowerShell integration
function global:wt {
    $wtBin = __WT_EXECUTABLE__
    if (-not (Test-Path -LiteralPath $wtBin -PathType Leaf)) {
        Write-Error "wt executable not found: $wtBin"
        $global:LASTEXITCODE = 1
        return
    }

    $markerFile = [IO.Path]::GetTempFileName()
    $hadCapture = Test-Path Env:WT_MANAGER_CAPTURE_CD
    $hadMarkerFile = Test-Path Env:WT_MANAGER_CD_MARKER_FILE
    $oldCapture = [Environment]::GetEnvironmentVariable('WT_MANAGER_CAPTURE_CD', 'Process')
    $oldMarkerFile = [Environment]::GetEnvironmentVariable('WT_MANAGER_CD_MARKER_FILE', 'Process')
    $exitCode = 1

    try {
        [Environment]::SetEnvironmentVariable('WT_MANAGER_CAPTURE_CD', '1', 'Process')
        [Environment]::SetEnvironmentVariable('WT_MANAGER_CD_MARKER_FILE', $markerFile, 'Process')
        & $wtBin @args
        $exitCode = $LASTEXITCODE

        if ($exitCode -eq 0 -and [IO.File]::Exists($markerFile)) {
            $utf8 = New-Object Text.UTF8Encoding($false, $true)
            $marker = [IO.File]::ReadAllText($markerFile, $utf8).TrimEnd([char[]]"`r`n")
            if ($marker.StartsWith('__WT_MARKER_PREFIX__', [StringComparison]::Ordinal)) {
                $targetPath = $marker.Substring('__WT_MARKER_PREFIX__'.Length)
                if ($targetPath.Length -gt 0) {
                    Set-Location -LiteralPath $targetPath
                }
            }
        }
    }
    catch {
        Write-Error -ErrorRecord $_
        $exitCode = 1
    }
    finally {
        if ($hadCapture) {
            [Environment]::SetEnvironmentVariable('WT_MANAGER_CAPTURE_CD', $oldCapture, 'Process')
        }
        else {
            [Environment]::SetEnvironmentVariable('WT_MANAGER_CAPTURE_CD', $null, 'Process')
        }
        if ($hadMarkerFile) {
            [Environment]::SetEnvironmentVariable('WT_MANAGER_CD_MARKER_FILE', $oldMarkerFile, 'Process')
        }
        else {
            [Environment]::SetEnvironmentVariable('WT_MANAGER_CD_MARKER_FILE', $null, 'Process')
        }
        Remove-Item -LiteralPath $markerFile -Force -ErrorAction SilentlyContinue
    }

    $global:LASTEXITCODE = $exitCode
}

Register-ArgumentCompleter -Native -CommandName wt -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $elements = @($commandAst.CommandElements | ForEach-Object { $_.Extent.Text })
    if ([string]::IsNullOrEmpty($wordToComplete)) {
        $argumentIndex = $elements.Count
    }
    else {
        $argumentIndex = $elements.Count - 1
    }

    $candidates = @()
    if ($argumentIndex -eq 1) {
        $candidates += @('init', 'tui', 'list', 'cd', 'run', 'delete', 'clean', 'project', 'config')
    }

    $subcommand = if ($elements.Count -gt 1) { $elements[1].Trim("'`"") } else { '' }
    $worktreeAction = if ($elements.Count -gt 2) { $elements[2].Trim("'`"") } else { '' }
    $needsBranch = ($argumentIndex -eq 1) -or
        (($subcommand -in @('cd', 'delete', 'run')) -and $argumentIndex -eq 2) -or
        (($subcommand -eq 'worktree') -and
            ($worktreeAction -in @('switch', 'delete', 'run')) -and
            $argumentIndex -eq 3)
    if ($needsBranch -and (Get-Command git -ErrorAction SilentlyContinue)) {
        $insideWorkTree = & git rev-parse --is-inside-work-tree 2>$null
        if ($LASTEXITCODE -eq 0 -and $insideWorkTree -eq 'true') {
            $candidates += & git for-each-ref '--format=%(refname:short)' refs/heads 2>$null
        }
    }

    $candidates |
        Where-Object { $_ -like "$wordToComplete*" } |
        Sort-Object -Unique |
        ForEach-Object {
            [Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
        }
}
"#;

    Ok(template
        .replace(
            "__WT_EXECUTABLE__",
            &powershell_single_quoted_literal(current_exe),
        )
        .replace("__WT_MARKER_PREFIX__", SHELL_CD_MARKER_PREFIX))
}

#[cfg(any(unix, test))]
fn render_shell_integration_script(current_exe: &Path) -> String {
    format!(
        r#"# wt-manager shell integration
wt() {{
    local wt_bin="{current_exe}"

    if [[ ! -f "$wt_bin" ]]; then
        echo "Error: wt binary not found. Run 'cargo install ...' and 'wt init' first."
        return 1
    fi

    local tmp_marker=$(mktemp)
    local exit_code

    WT_MANAGER_CAPTURE_CD=1 {marker_file_env}="$tmp_marker" "$wt_bin" "$@"
    exit_code=$?

    if [[ $exit_code -eq 0 && -s "$tmp_marker" ]]; then
        local target_marker=""
        local target_dir=""
        while IFS= read -r marker_line; do
            target_marker="$marker_line"
        done < "$tmp_marker"
        target_dir="${{target_marker#{marker_prefix}}}"
        if [[ -n "$target_dir" ]]; then
            cd "$target_dir" || exit_code=$?
        fi
    fi

    rm -f "$tmp_marker"
    return "$exit_code"
}}

_wt_command_candidates() {{
    printf '%s\n' init tui list cd run delete clean project config
}}

_wt_branch_candidates() {{
    git rev-parse --is-inside-work-tree >/dev/null 2>&1 || return 0
    git for-each-ref --format='%(refname:short)' refs/heads 2>/dev/null
}}

_wt_completion() {{
    if [[ -n "$ZSH_VERSION" ]]; then
        local subcommand="${{words[2]}}"

        if (( CURRENT == 2 )); then
            compadd -- $(_wt_command_candidates)
            compadd -- $(_wt_branch_candidates)
        elif [[ "$subcommand" == "cd" || "$subcommand" == "delete" || "$subcommand" == "run" ]] && (( CURRENT == 3 )); then
            compadd -- $(_wt_branch_candidates)
        elif [[ "$subcommand" == "worktree" && ( "${{words[3]}}" == "switch" || "${{words[3]}}" == "delete" || "${{words[3]}}" == "run" ) ]] && (( CURRENT == 4 )); then
            compadd -- $(_wt_branch_candidates)
        fi

        return 0
    fi

    local cur="${{COMP_WORDS[COMP_CWORD]}}"
    local candidates=""

    if [[ $COMP_CWORD -eq 1 ]]; then
        candidates="$(_wt_command_candidates; _wt_branch_candidates)"
    elif [[ "${{COMP_WORDS[1]}}" == "cd" || "${{COMP_WORDS[1]}}" == "delete" || "${{COMP_WORDS[1]}}" == "run" ]] && [[ $COMP_CWORD -eq 2 ]]; then
        candidates="$(_wt_branch_candidates)"
    elif [[ "${{COMP_WORDS[1]}}" == "worktree" && ( "${{COMP_WORDS[2]}}" == "switch" || "${{COMP_WORDS[2]}}" == "delete" || "${{COMP_WORDS[2]}}" == "run" ) ]] && [[ $COMP_CWORD -eq 3 ]]; then
        candidates="$(_wt_branch_candidates)"
    fi

    COMPREPLY=( $(compgen -W "$candidates" -- "$cur") )
}}

if [[ -n "$ZSH_VERSION" ]] && type compdef >/dev/null 2>&1; then
    compdef _wt_completion wt
elif [[ -n "$BASH_VERSION" ]] && type complete >/dev/null 2>&1; then
    complete -F _wt_completion wt
fi
"#,
        current_exe = current_exe.display(),
        marker_prefix = SHELL_CD_MARKER_PREFIX,
        marker_file_env = SHELL_CD_MARKER_FILE_ENV,
    )
}

#[cfg(test)]
mod tests {
    use super::{
        clean_merged_worktrees, handle_delete_command, maybe_print_shell_cd_marker,
        render_shell_integration_script, shell_cd_marker_line, SHELL_CD_MARKER_FILE_ENV,
        SHELL_CD_MARKER_PREFIX,
    };
    #[cfg(windows)]
    use super::{
        decode_powershell_profile, encode_powershell_profile, init_shell_integration,
        install_powershell_integration, powershell_single_quoted_literal,
        render_powershell_integration_script, update_powershell_profile, write_utf8_bom,
        ProfileEncoding, POWERSHELL_PROFILE_BLOCK_END, POWERSHELL_PROFILE_BLOCK_START,
    };
    #[cfg(windows)]
    use std::ffi::OsStr;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
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
    fn handle_delete_command_returns_error_for_failed_removal() {
        let _guard = env_lock().lock().unwrap();
        let temp_home = make_temp_dir("home");
        let previous_home = std::env::var_os("HOME");
        std::env::set_var("HOME", &temp_home);

        let repo_root = init_repo();
        let worktree_path = temp_home.join("feature-wt");
        git(
            &repo_root,
            &[
                "worktree",
                "add",
                worktree_path.to_str().unwrap(),
                "-b",
                "feature",
            ],
        );
        fs::write(worktree_path.join("dirty.txt"), "dirty\n").unwrap();

        let result = handle_delete_command(&repo_root, "feature", false);

        assert!(result.is_err());

        match previous_home {
            Some(home) => std::env::set_var("HOME", home),
            None => std::env::remove_var("HOME"),
        }
        fs::remove_dir_all(repo_root).unwrap();
        fs::remove_dir_all(worktree_path).unwrap();
        fs::remove_dir_all(temp_home).unwrap();
    }

    #[test]
    fn clean_merged_worktrees_returns_error_for_failed_removal() {
        let _guard = env_lock().lock().unwrap();
        let temp_home = make_temp_dir("home");
        let previous_home = std::env::var_os("HOME");
        std::env::set_var("HOME", &temp_home);

        let repo_root = init_repo();
        let worktree_path = temp_home.join("feature-wt");
        git(
            &repo_root,
            &[
                "worktree",
                "add",
                worktree_path.to_str().unwrap(),
                "-b",
                "feature",
            ],
        );
        fs::write(worktree_path.join("dirty.txt"), "dirty\n").unwrap();

        let result = clean_merged_worktrees(&repo_root, false, false, Some("main"));

        assert!(result.is_err());

        match previous_home {
            Some(home) => std::env::set_var("HOME", home),
            None => std::env::remove_var("HOME"),
        }
        fs::remove_dir_all(repo_root).unwrap();
        fs::remove_dir_all(worktree_path).unwrap();
        fs::remove_dir_all(temp_home).unwrap();
    }

    #[test]
    fn shell_cd_marker_line_uses_structured_prefix() {
        let marker = shell_cd_marker_line(Path::new("/tmp/project"));

        assert_eq!(
            marker,
            format!("{}{}", SHELL_CD_MARKER_PREFIX, "/tmp/project")
        );
    }

    #[test]
    fn maybe_print_shell_cd_marker_writes_marker_file_when_configured() {
        let _guard = env_lock().lock().unwrap();
        let temp_dir = make_temp_dir("marker-file");
        let marker_file = temp_dir.join("cd-marker");
        let previous_capture = std::env::var_os("WT_MANAGER_CAPTURE_CD");
        let previous_marker_file = std::env::var_os(SHELL_CD_MARKER_FILE_ENV);
        std::env::set_var("WT_MANAGER_CAPTURE_CD", "1");
        std::env::set_var(SHELL_CD_MARKER_FILE_ENV, &marker_file);

        maybe_print_shell_cd_marker(Path::new("/tmp/project"));

        let marker = fs::read_to_string(&marker_file).unwrap();
        assert_eq!(
            marker,
            format!("{}{}\n", SHELL_CD_MARKER_PREFIX, "/tmp/project")
        );

        match previous_capture {
            Some(value) => std::env::set_var("WT_MANAGER_CAPTURE_CD", value),
            None => std::env::remove_var("WT_MANAGER_CAPTURE_CD"),
        }
        match previous_marker_file {
            Some(value) => std::env::set_var(SHELL_CD_MARKER_FILE_ENV, value),
            None => std::env::remove_var(SHELL_CD_MARKER_FILE_ENV),
        }
        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn shell_integration_script_uses_marker_file_without_piping_stdout() {
        let script = render_shell_integration_script(Path::new("/tmp/wt"));

        assert!(script.contains(
            "WT_MANAGER_CAPTURE_CD=1 WT_MANAGER_CD_MARKER_FILE=\"$tmp_marker\" \"$wt_bin\" \"$@\""
        ));
        assert!(script.contains(SHELL_CD_MARKER_PREFIX));
        assert!(!script.contains("| tee"));
        assert!(!script.contains("| grep"));
        assert!(!script.contains("grep \"^  cd \""));
    }

    #[cfg(unix)]
    #[test]
    fn shell_integration_changes_directory_from_marker_file() {
        let temp_dir = make_temp_dir("shell-marker");
        let target_dir = temp_dir.join("target");
        fs::create_dir_all(&target_dir).unwrap();
        let fake_bin = temp_dir.join("fake-wt");
        fs::write(
            &fake_bin,
            format!(
                "#!/bin/sh\nprintf 'visible-output\\n'\nprintf '{}{}\\n' > \"${}\"\n",
                SHELL_CD_MARKER_PREFIX,
                target_dir.display(),
                SHELL_CD_MARKER_FILE_ENV
            ),
        )
        .unwrap();
        let mut permissions = fs::metadata(&fake_bin).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&fake_bin, permissions).unwrap();

        let script_path = temp_dir.join("wt-manager.sh");
        fs::write(&script_path, render_shell_integration_script(&fake_bin)).unwrap();

        let output = Command::new("bash")
            .arg("-lc")
            .arg("source \"$WT_SCRIPT\"; wt; printf 'PWD=%s\n' \"$PWD\"")
            .current_dir(&temp_dir)
            .env("WT_SCRIPT", &script_path)
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "shell integration command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.lines().any(|line| line == "visible-output"));
        assert!(stdout
            .lines()
            .any(|line| line == format!("PWD={}", target_dir.display())));

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn shell_integration_script_registers_branch_completion() {
        let script = render_shell_integration_script(Path::new("/tmp/wt"));

        assert!(script.contains("git for-each-ref --format='%(refname:short)' refs/heads"));
        assert!(script.contains("compdef _wt_completion wt"));
        assert!(script.contains("complete -F _wt_completion wt"));
        assert!(script.contains("COMP_CWORD -eq 1"));
    }

    #[cfg(unix)]
    #[test]
    fn shell_integration_bash_completion_lists_existing_branches() {
        let repo_root = init_repo();
        git(&repo_root, &["branch", "feature/login"]);
        git(&repo_root, &["branch", "bugfix"]);

        let script_path = repo_root.join("wt-manager.sh");
        fs::write(
            &script_path,
            render_shell_integration_script(Path::new("/bin/true")),
        )
        .unwrap();

        let output = Command::new("bash")
            .arg("-lc")
            .arg(
                "source \"$WT_SCRIPT\"; \
                 COMP_WORDS=(wt feat); \
                 COMP_CWORD=1; \
                 _wt_completion; \
                 printf '%s\n' \"${COMPREPLY[@]}\"",
            )
            .current_dir(&repo_root)
            .env("WT_SCRIPT", &script_path)
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "bash completion command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let completions = String::from_utf8(output.stdout).unwrap();
        assert!(completions.lines().any(|line| line == "feature/login"));
        assert!(!completions.lines().any(|line| line == "bugfix"));

        fs::remove_dir_all(repo_root).unwrap();
    }
    #[cfg(windows)]
    #[test]
    fn powershell_renderer_escapes_executable_and_preserves_wrapper_contract() {
        let executable = Path::new(r"C:\Program Files\wt's tools\wt.exe");
        let script = render_powershell_integration_script(executable).unwrap();

        assert!(script.contains("$wtBin = 'C:\\Program Files\\wt''s tools\\wt.exe'"));
        assert!(script.contains("& $wtBin @args"));
        assert!(script.contains("[IO.Path]::GetTempFileName()"));
        assert!(script.contains("New-Object Text.UTF8Encoding($false, $true)"));
        assert!(script.contains("Set-Location -LiteralPath $targetPath"));
        assert!(script.contains("$global:LASTEXITCODE = $exitCode"));
        assert!(script.contains("finally {"));
        assert!(script.contains("Remove-Item -LiteralPath $markerFile"));
        assert!(script.contains("Register-ArgumentCompleter -Native -CommandName wt"));
        assert!(script.contains("git for-each-ref '--format=%(refname:short)' refs/heads"));
        assert!(script.contains(SHELL_CD_MARKER_PREFIX));
        assert!(!script.contains("cmd /C"));
    }

    #[cfg(windows)]
    #[test]
    fn powershell_profile_update_is_idempotent_and_literal() {
        let integration_path = Path::new(r"C:\Users\O'Brien\.wt-manager.ps1");
        let initial = "Set-Alias ll Get-ChildItem\r\n";

        let once = update_powershell_profile(initial, integration_path).unwrap();
        let twice = update_powershell_profile(&once, integration_path).unwrap();

        assert_eq!(once, twice);
        assert_eq!(once.matches(POWERSHELL_PROFILE_BLOCK_START).count(), 1);
        assert_eq!(once.matches(POWERSHELL_PROFILE_BLOCK_END).count(), 1);
        assert!(once.contains(". 'C:\\Users\\O''Brien\\.wt-manager.ps1'\r\n"));
        assert!(once.starts_with(initial));
    }

    #[cfg(windows)]
    #[test]
    fn powershell_profile_encodings_round_trip() {
        let content = "Write-Output '한글'\r\n";
        for encoding in [
            ProfileEncoding::Utf8Bom,
            ProfileEncoding::Utf8,
            ProfileEncoding::Utf16Le,
            ProfileEncoding::Utf16Be,
        ] {
            let bytes = encode_powershell_profile(content, encoding);
            let (decoded, detected) = decode_powershell_profile(&bytes).unwrap();
            assert_eq!(decoded, content);
            assert_eq!(detected, encoding);
        }
    }

    #[cfg(windows)]
    #[test]
    fn powershell_install_preserves_profile_encoding_and_does_not_duplicate_block() {
        let temp_dir = make_temp_dir("powershell-profile");
        let home = temp_dir.join("home");
        let original = "Write-Output 'existing'\r\n";
        let executable = temp_dir.join("wt.exe");
        let cases = [
            (ProfileEncoding::Utf8Bom, &[0xef, 0xbb, 0xbf][..], "utf8"),
            (ProfileEncoding::Utf16Le, &[0xff, 0xfe][..], "utf16le"),
            (ProfileEncoding::Utf16Be, &[0xfe, 0xff][..], "utf16be"),
        ];

        for (expected_encoding, expected_bom, name) in cases {
            let profile = temp_dir
                .join("profiles")
                .join(format!("{name}-profile.ps1"));
            fs::create_dir_all(profile.parent().unwrap()).unwrap();
            fs::write(
                &profile,
                encode_powershell_profile(original, expected_encoding),
            )
            .unwrap();

            install_powershell_integration(&home, &profile, &executable).unwrap();
            let after_first = fs::read(&profile).unwrap();
            install_powershell_integration(&home, &profile, &executable).unwrap();
            let after_second = fs::read(&profile).unwrap();

            assert_eq!(after_first, after_second);
            assert!(after_second.starts_with(expected_bom));
            let (profile_text, actual_encoding) = decode_powershell_profile(&after_second).unwrap();
            assert_eq!(actual_encoding, expected_encoding);
            assert_eq!(
                profile_text.matches(POWERSHELL_PROFILE_BLOCK_START).count(),
                1
            );
        }

        let new_profile = temp_dir.join("profiles").join("new-profile.ps1");
        install_powershell_integration(&home, &new_profile, &executable).unwrap();
        assert!(fs::read(new_profile)
            .unwrap()
            .starts_with(&[0xef, 0xbb, 0xbf]));

        let generated = fs::read(home.join(".wt-manager.ps1")).unwrap();
        assert!(generated.starts_with(&[0xef, 0xbb, 0xbf]));

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn unsupported_profile_encoding_is_not_modified() {
        let temp_dir = make_temp_dir("unsupported-profile");
        let home = temp_dir.join("home");
        let profile = temp_dir.join("profile.ps1");
        let unsupported = [0xff, 0xfe, 0x00, 0x00, b'#', 0x00, 0x00, 0x00];
        fs::write(&profile, unsupported).unwrap();

        let result = install_powershell_integration(&home, &profile, &temp_dir.join("wt.exe"));

        assert!(result.is_err());
        assert_eq!(fs::read(&profile).unwrap(), unsupported);
        assert!(!home.join(".wt-manager.ps1").exists());
        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn powershell_init_requires_explicit_profile() {
        let error = init_shell_integration(None).unwrap_err().to_string();

        assert!(error.contains("wt init --profile $PROFILE"));
        assert!(error.contains("cmd.exe"));
        assert!(error.contains("not supported"));
    }

    #[cfg(windows)]
    #[test]
    fn powershell_wrapper_changes_the_calling_shell_directory() {
        let powershell_hosts = ["pwsh.exe", "powershell.exe"]
            .iter()
            .filter_map(|name| crate::process::resolve_executable(OsStr::new(name)))
            .collect::<Vec<_>>();
        if powershell_hosts.is_empty() {
            return;
        }

        let temp_dir = make_temp_dir("powershell-wrapper");
        let target_dir = temp_dir.join("target's directory");
        fs::create_dir_all(&target_dir).unwrap();
        git(&target_dir, &["init", "-b", "main"]);
        fs::write(target_dir.join("README.md"), "completion fixture\n").unwrap();
        git(&target_dir, &["add", "README.md"]);
        git(&target_dir, &["commit", "-m", "completion fixture"]);
        git(&target_dir, &["branch", "feature/login"]);
        let fake_wt = temp_dir.join("fake-wt.ps1");
        let marker = format!("{}{}", SHELL_CD_MARKER_PREFIX, target_dir.display());
        let fake_script = format!(
            "if ($args.Count -ne 2 -or $args[0] -ne 'arg with spaces' -or $args[1] -ne 'quote''value') {{ $global:LASTEXITCODE = 23; return }}\r\n\
             $utf8 = New-Object Text.UTF8Encoding($false)\r\n\
             [IO.File]::WriteAllText($env:WT_MANAGER_CD_MARKER_FILE, {}, $utf8)\r\n\
             $global:LASTEXITCODE = 0\r\n",
            powershell_single_quoted_literal(Path::new(&marker))
        );
        write_utf8_bom(&fake_wt, &fake_script).unwrap();
        let wrapper = temp_dir.join("wt-manager.ps1");
        write_utf8_bom(
            &wrapper,
            &render_powershell_integration_script(&fake_wt).unwrap(),
        )
        .unwrap();
        let command = format!(
            ". {}; wt 'wrong'; \
             if ($global:LASTEXITCODE -ne 23) {{ exit 30 }}; \
             if ((Get-Location).Path -ne {}) {{ exit 34 }}; \
             if ($env:WT_MANAGER_CAPTURE_CD -ne 'before') {{ exit 32 }}; \
             if ($env:WT_MANAGER_CD_MARKER_FILE -ne 'existing-marker') {{ exit 33 }}; \
             wt 'arg with spaces' 'quote''value'; \
             if ((Get-Location).Path -ne {}) {{ exit 31 }}; \
             if ($env:WT_MANAGER_CAPTURE_CD -ne 'before') {{ exit 32 }}; \
             if ($env:WT_MANAGER_CD_MARKER_FILE -ne 'existing-marker') {{ exit 33 }}; \
             $completions = @((TabExpansion2 'wt cd feat' 10).CompletionMatches.CompletionText); \
             if ($completions -notcontains 'feature/login') {{ exit 35 }}; \
             $subcommands = @((TabExpansion2 'wt cl' 5).CompletionMatches.CompletionText); \
             if ($subcommands -notcontains 'clean') {{ exit 36 }}; \
             exit $global:LASTEXITCODE",
            powershell_single_quoted_literal(&wrapper),
            powershell_single_quoted_literal(&temp_dir),
            powershell_single_quoted_literal(&target_dir)
        );

        for powershell in powershell_hosts {
            let status = Command::new(&powershell)
                .args([
                    "-NoLogo",
                    "-NoProfile",
                    "-NonInteractive",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-Command",
                    &command,
                ])
                .current_dir(&temp_dir)
                .env("WT_MANAGER_CAPTURE_CD", "before")
                .env("WT_MANAGER_CD_MARKER_FILE", "existing-marker")
                .status()
                .unwrap();

            assert!(
                status.success(),
                "PowerShell wrapper exited with {status} under {}",
                powershell.display()
            );
        }
        fs::remove_dir_all(temp_dir).unwrap();
    }
}
