mod db;
mod git;
mod i18n;
mod tui;
mod setup;
mod worktree;

use anyhow::Result;
use clap::Parser;
use std::env;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "wt")]
#[command(about = "Advanced git worktree manager", long_about = None)]
struct Args {
    /// Branch name for worktree
    branch: Option<String>,
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

    // Check if we're in a git repository (use main repo root to handle worktrees)
    if let Some(repo_root) = git::find_main_repo_root(&current_dir)? {
        handle_git_repo(repo_root, args)?;
    } else {
        // Not in a git repo - show TUI to select from saved projects
        tui::show_project_selector()?;
    }

    Ok(())
}

fn handle_git_repo(repo_root: PathBuf, args: Args) -> Result<()> {
    // Save this project to the database
    db::save_project(&repo_root)?;

    if let Some(branch) = args.branch {
        // User specified a branch - create or switch to worktree
        worktree::handle_worktree(&repo_root, &branch)?;
    } else {
        // No branch specified - show TUI to select worktree
        tui::show_worktree_selector(&repo_root)?;
    }

    Ok(())
}
