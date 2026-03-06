mod db;
mod git;
mod i18n;
mod tui;
mod setup;
mod worktree;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::env;
use std::path::Path;
use std::path::PathBuf;

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
    // Set up Ctrl+C handler
    ctrlc::set_handler(|| {
        // Clean up terminal state if needed
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::LeaveAlternateScreen
        );
        std::process::exit(0);
    })
    .expect("Error setting Ctrl+C handler");

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
    /// Open interactive TUI
    Tui,
    /// Manage worktrees
    Worktree {
        #[command(subcommand)]
        command: WorktreeCommands,
    },
    /// Manage saved projects
    Project {
        #[command(subcommand)]
        command: ProjectCommands,
    },
}

#[derive(Subcommand, Debug)]
enum WorktreeCommands {
    /// List all worktrees in this repository
    List,
    /// Switch to existing worktree or create one if branch does not exist
    Switch { branch: String },
    /// Delete a worktree
    Delete {
        branch: String,
        #[arg(short, long, help = "Force delete (use when normal delete fails due to uncommitted changes).")]
        force: bool,
    },
}

#[derive(Subcommand, Debug)]
enum ProjectCommands {
    /// List saved projects by last accessed order
    List,
}

fn handle_command(cmd: Commands, current_dir: &Path) -> Result<()> {
    match cmd {
        Commands::Tui => handle_default(current_dir),
        Commands::Worktree { command } => handle_worktree_command(command, current_dir),
        Commands::Project { command } => match command {
            ProjectCommands::List => list_projects(),
        },
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

fn handle_worktree_command(command: WorktreeCommands, current_dir: &Path) -> Result<()> {
    let repo_root = match get_repo_root_or_project_tui(current_dir)? {
        Some(root) => root,
        None => {
            anyhow::bail!("This command requires a git repository. Run `wt` in a repository or with a project selected.");
        }
    };

    match command {
        WorktreeCommands::List => list_worktrees(&repo_root),
        WorktreeCommands::Switch { branch } => {
            worktree::handle_worktree(&repo_root, &branch)?;
            Ok(())
        }
        WorktreeCommands::Delete { branch, force } => {
            let messages = i18n::Messages::new();
            let worktrees = git::list_worktrees(&repo_root)?;
            let target = worktrees
                .iter()
                .find(|wt| wt.branch.eq_ignore_ascii_case(&branch));

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
                                    format!("wt worktree delete {}", wt.branch)
                                );
                            }
                        }
                    }
                }
                None => {
                    eprintln!("Worktree not found: {}", branch);
                }
            }

            Ok(())
        }
    }
}

fn list_worktrees(repo_root: &Path) -> Result<()> {
    let messages = i18n::Messages::new();
    let worktrees = git::list_worktrees(repo_root)?;

    println!("\n{}", messages.select_or_create_worktree());
    for wt in worktrees {
        let marker = if wt.is_main { " (main)" } else { "" };
        println!("  {}{}", wt.branch, marker);
        println!("    path: {}", wt.path.display());
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

    println!("\nSaved Projects:");
    for project in projects {
        println!("  {} ({})", project.name, project.path.display());
    }

    Ok(())
}

const LONG_HELP: &str = r#"Advanced Git worktree manager for terminal users.

Usage:
  wt                          # interactive project/worktree selector (default)
  wt <branch>                 # legacy mode: create or switch worktree
  wt tui                      # explicitly open interactive TUI
  wt worktree list            # list worktrees (main repo)
  wt worktree switch <branch> # same as `wt <branch>`
  wt worktree delete <branch> [--force] # delete a worktree
  wt project list             # list registered projects (recent first)

Branch behavior:
  wt <branch> / wt worktree switch <branch>
  - Try existing branch first
  - If branch does not exist, create it automatically
  - Run environment setup automatically after switch/create

Delete safety:
  - Main worktree is protected and cannot be removed
  - If deletion fails (e.g., uncommitted changes), retry with --force

Note:
  Actual directory change is performed by the wt shell wrapper (`wt-wrapper.sh`) by parsing `cd` output.
  To move the shell working directory, use the wrapper-included shell function.

Examples:
  wt
  wt feature/login
  wt worktree list
  wt worktree delete feature/login --force
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
