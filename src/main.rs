mod db;
mod git;
mod i18n;
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

#[derive(Parser, Debug)]
#[command(name = "wt")]
#[command(
    version,
    about = "Advanced git worktree manager",
    long_about = LONG_HELP,
    after_help = TUI_HELP
)]
struct Args {
    /// Legacy mode: switch or create worktree for BRANCH
    #[arg(value_name = "BRANCH", required = false)]
    branch: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
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
    .expect(messages.ctrlc_handler_error());

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
    /// Initialize shell integration
    Init,
    /// Open interactive TUI
    Tui,
    #[command(hide = true)]
    Worktree {
        #[command(subcommand)]
        command: WorktreeAliasCommands,
    },
    /// List all worktrees in this repository
    List,
    /// Switch to existing worktree or create one if branch does not exist
    Cd { branch: String },
    /// Run a command inside a worktree
    Run {
        branch: String,
        #[arg(required = true, num_args = 1.., trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
    /// Delete a worktree
    Delete {
        branch: String,
        #[arg(
            short,
            long,
            help = "Force delete (use when normal delete fails due to uncommitted changes)."
        )]
        force: bool,
    },
    /// Remove worktrees whose tracking branches were deleted from remote or already merged
    Clean {
        /// Do not delete, only list removable worktrees
        #[arg(short, long)]
        dry_run: bool,
        /// Force delete worktrees even when they have local changes
        #[arg(short, long)]
        force: bool,
        /// Include worktrees whose tracking branch does not exist on remote
        #[arg(long)]
        include_untracked: bool,
        /// Optional remote name to check. If not set, checks all upstream remotes.
        #[arg(short, long)]
        remote: Option<String>,
        /// Skip git fetch --prune before checking remote refs
        #[arg(long)]
        skip_fetch: bool,
        /// Remove worktrees whose branch is already merged into the base branch
        #[arg(long)]
        merged: bool,
        /// Base branch used with --merged
        #[arg(long)]
        base: Option<String>,
    },
    /// Manage saved projects
    Project {
        #[command(subcommand)]
        command: ProjectCommands,
    },
    /// Manage wt configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
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
    /// List saved projects by last accessed order
    List,
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

fn handle_command(cmd: Commands, current_dir: &Path) -> Result<()> {
    match cmd {
        Commands::Init => init_shell_integration(),
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
            dry_run,
            force,
            include_untracked,
            remote.as_deref(),
            skip_fetch,
            merged,
            base.as_deref(),
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
            dry_run,
            force,
            include_untracked,
            remote.as_deref(),
            skip_fetch,
            merged,
            base.as_deref(),
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
    let target = worktrees
        .iter()
        .find(|wt| wt.branch.eq_ignore_ascii_case(branch));

    match target {
        Some(wt) => {
            if wt.is_main {
                eprintln!("{}", messages.cannot_delete_main());
                return Ok(());
            }

            println!("\n{} {}", messages.deleting_worktree(), wt.branch);
            match git::remove_worktree(&repo_root, &wt.path, force) {
                Ok(()) => {
                    println!("{}", messages.worktree_deleted().replace("{}", &wt.branch));
                }
                Err(e) => {
                    eprintln!("\n{} {}", messages.failed_to_delete(), e);
                    if !force {
                        eprintln!("\n{}", messages.uncommitted_changes_tip());
                        eprintln!(
                            "{} {} --force",
                            messages.force_delete_command(),
                            format!("wt delete {}", wt.branch)
                        );
                    }
                }
            }
        }
        None => {
            eprintln!("{}", messages.cannot_find_worktree().replace("{}", branch));
        }
    }

    Ok(())
}

fn handle_clean_command(
    current_dir: &Path,
    dry_run: bool,
    force: bool,
    include_untracked: bool,
    remote: Option<&str>,
    skip_fetch: bool,
    merged: bool,
    base: Option<&str>,
) -> Result<()> {
    let messages = i18n::Messages::new();
    let repo_root = match get_repo_root_or_project_tui(current_dir)? {
        Some(root) => root,
        None => anyhow::bail!("{}", messages.cmd_requires_repo()),
    };

    if merged {
        clean_merged_worktrees(&repo_root, dry_run, force, base)
    } else {
        clean_stale_worktrees(
            &repo_root,
            dry_run,
            force,
            include_untracked,
            remote,
            skip_fetch,
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
    let repo_root = match get_repo_root_or_project_tui(current_dir)? {
        Some(root) => root,
        None => anyhow::bail!("This command requires a git repository."),
    };

    match command {
        ConfigCommands::Show => {
            match db::get_project_automation(&repo_root)? {
                Some(config) => {
                    println!("Repo config for {}", repo_root.display());
                    println!("copy_files:");
                    for entry in config.copy_files {
                        println!("  - {}", entry);
                    }
                    println!("post_create_hooks:");
                    for (index, entry) in config.post_create_hooks.iter().enumerate() {
                        println!("  [{}] {}", index, entry);
                    }
                    println!("post_cd_hooks:");
                    for (index, entry) in config.post_cd_hooks.iter().enumerate() {
                        println!("  [{}] {}", index, entry);
                    }
                }
                None => println!("No repo-specific wt config found."),
            }
            Ok(())
        }
        ConfigCommands::Copy { command } => match command {
            CopyCommands::Add { path } => {
                db::add_copy_file(&repo_root, &path)?;
                println!("Added copy file '{}'", path);
                Ok(())
            }
            CopyCommands::Remove { path } => {
                if db::remove_copy_file(&repo_root, &path)? {
                    println!("Removed copy file '{}'", path);
                } else {
                    println!("Copy file '{}' was not configured", path);
                }
                Ok(())
            }
        },
        ConfigCommands::Hook { command } => match command {
            HookCommands::Add { hook, command } => {
                db::add_hook(&repo_root, hook_kind_name(&hook), &command)?;
                println!("Added {} hook '{}'", hook_kind_name(&hook), command);
                Ok(())
            }
            HookCommands::Remove { hook, index } => {
                if db::remove_hook(&repo_root, hook_kind_name(&hook), index)? {
                    println!("Removed {} hook at index {}", hook_kind_name(&hook), index);
                } else {
                    println!("No {} hook found at index {}", hook_kind_name(&hook), index);
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
        if let Some(pr) = pull_requests.get(&wt.branch) {
            println!("  {}{} #{} {}", wt.branch, marker, pr.number, pr.title);
        } else {
            println!("  {}{}", wt.branch, marker);
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
        let reason = messages.failed_fetch_prune().replace("{}", &stderr.trim());
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
        let upstream = branches.get(&wt.branch).cloned().flatten();

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
                        format!(
                            "{}",
                            messages
                                .stale_upstream_missing_reason()
                                .replace("{}", &value)
                        ),
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
        println!("  {} ({})", wt.branch, reason);
        println!("    {}", wt.path.display());
    }

    if dry_run {
        println!("\n{}", messages.dry_run_enabled());
        return Ok(());
    }

    let messages = i18n::Messages::new();
    for (wt, _) in stale_worktrees {
        println!("\n{} {}", messages.deleting_worktree(), wt.branch);
        match git::remove_worktree(repo_root, &wt.path, force) {
            Ok(()) => {
                println!("{}", messages.worktree_deleted().replace("{}", &wt.branch));
            }
            Err(e) => {
                eprintln!("\n{} {}", messages.failed_to_delete(), e);
                if !force {
                    eprintln!("\n{}", messages.uncommitted_changes_tip());
                    eprintln!(
                        "{} {}",
                        messages.force_delete_command(),
                        format!("wt delete {} --force", wt.branch)
                    );
                }
            }
        }
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
        if wt.branch == base_branch || wt.branch == base_short {
            continue;
        }
        if git::is_branch_merged_into(repo_root, &wt.branch, &base_branch)? {
            merged_worktrees.push((wt, format!("merged into '{}'", base_branch)));
        }
    }

    if merged_worktrees.is_empty() {
        println!("No merged worktrees found.");
        return Ok(());
    }

    println!("Found {} merged worktree(s):", merged_worktrees.len());
    for (wt, reason) in &merged_worktrees {
        println!("  {} ({})", wt.branch, reason);
        println!("    {}", wt.path.display());
    }

    if dry_run {
        println!("\n{}", messages.dry_run_enabled());
        return Ok(());
    }

    for (wt, _) in merged_worktrees {
        println!("\n{} {}", messages.deleting_worktree(), wt.branch);
        match git::remove_worktree(repo_root, &wt.path, force) {
            Ok(()) => {
                println!("{}", messages.worktree_deleted().replace("{}", &wt.branch));
            }
            Err(error) => {
                eprintln!("\n{} {}", messages.failed_to_delete(), error);
                if !force {
                    eprintln!("\n{}", messages.uncommitted_changes_tip());
                    eprintln!(
                        "{} {}",
                        messages.force_delete_command(),
                        format!("wt delete {} --force", wt.branch)
                    );
                }
            }
        }
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

fn init_shell_integration() -> Result<()> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Failed to get home directory"))?;
    let shell_integration_path = home.join(".wt-manager.sh");
    let current_exe = env::current_exe()?;
    let script = format!(
        "# wt-manager shell integration\nwt() {{\n    local wt_bin=\"{}\"\n\n    if [[ ! -f \"$wt_bin\" ]]; then\n        echo \"Error: wt binary not found. Run 'cargo install ...' and 'wt init' first.\"\n        return 1\n    fi\n\n    local tmp_output=$(mktemp)\n    local exit_code\n    if [[ -n \"$ZSH_VERSION\" ]]; then\n        \"$wt_bin\" \"$@\" | tee \"$tmp_output\"\n        local -a pipe_status=(\"${{pipestatus[@]}}\")\n        exit_code=${{pipe_status[1]}}\n    else\n        \"$wt_bin\" \"$@\" | tee \"$tmp_output\"\n        local -a pipe_status=(\"${{PIPESTATUS[@]}}\")\n        exit_code=${{pipe_status[0]}}\n    fi\n    local cd_line=$(grep \"^  cd \" \"$tmp_output\" | head -n1)\n\n    if [[ -n \"$cd_line\" ]]; then\n        local target_dir=$(echo \"$cd_line\" | sed 's/^  cd //')\n        if [[ -d \"$target_dir\" ]]; then\n            cd \"$target_dir\" || {{\n                rm -f \"$tmp_output\"\n                return 1\n            }}\n            echo \"\"\n            echo \"Changed to: $(pwd)\"\n        fi\n    fi\n\n    rm -f \"$tmp_output\"\n    return $exit_code\n}}\n",
        current_exe.display()
    );
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
            println!("Already configured {}", rc_path.display());
            continue;
        }

        let mut rewritten = lines.join("\n");
        if !rewritten.ends_with('\n') && !rewritten.is_empty() {
            rewritten.push('\n');
        }
        rewritten.push_str("# wt-manager shell integration\nsource ~/.wt-manager.sh\n");
        fs::write(&rc_path, rewritten)?;
        println!("Updated {}", rc_path.display());
        updated_any = true;
    }

    println!("Generated {}", shell_integration_path.display());
    if !updated_any {
        println!("Shell integration was already configured.");
    }

    Ok(())
}

const LONG_HELP: &str = r#"Advanced Git worktree manager for terminal users.

Usage:
  wt init                     # install shell integration into ~/.wt-manager.sh and your shell rc
  wt                          # interactive project/worktree selector (default)
  wt <branch>                 # legacy mode: create or switch worktree
  wt tui                      # explicitly open interactive TUI
  wt list                     # list worktrees (main repo)
  wt cd <branch>              # same as `wt <branch>`
  wt run <branch> -- <cmd...>
  wt clean                    # delete worktrees with deleted remote tracking branches
  wt clean --merged [--base origin/main]
  wt clean --remote origin --dry-run
  wt delete <branch> [--force] # delete a worktree
  wt project list             # list registered projects (recent first)
  wt config show
  wt config copy add .env.local
  wt config hook add post-create "pnpm install"

Branch behavior:
  wt <branch> / wt cd <branch>
  - Try existing branch first
  - If branch does not exist, create it automatically
  - Run configured postCreate/postCd hooks from ~/.wt-manager/db.json

Delete safety:
  - Main worktree is protected and cannot be removed
  - If deletion fails (e.g., uncommitted changes), retry with --force

Note:
  Actual directory change is performed by the generated shell integration (`~/.wt-manager.sh`)
  by parsing `cd` output from the wt command.

Examples:
  wt
  wt feature/login
  wt list
  wt run feature/login -- cargo test
  wt delete feature/login --force
  wt project list
"#;

const TUI_HELP: &str = r#"TUI mode keys:
  Type:        search/fuzzy input
  Tab:         autocomplete with top match
  Enter:       select
  Ctrl+B:      create (worktree mode only)
  Ctrl+X:      delete exact match (worktree mode only)
  Ctrl+C/Esc:  cancel

TUI is opened automatically when:
- no argument and current dir is a git repository -> worktree selector
- no argument and outside git repository -> project selector"#;
