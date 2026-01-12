use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use std::io;
use std::path::Path;

use crate::{db, git, worktree};

pub fn show_project_selector() -> Result<()> {
    let messages = crate::i18n::Messages::new();
    let projects = db::get_projects()?;

    if projects.is_empty() {
        println!("{}", messages.no_projects_found());
        println!("{}", messages.navigate_to_git_repo());
        return Ok(());
    }

    let items: Vec<String> = projects
        .iter()
        .map(|p| format!("{} ({})", p.name, p.path.display()))
        .collect();

    let action = run_input_selector(messages.select_project(), &items, false, false, &messages)?;

    match action {
        SelectorAction::Select(input) => {
            // Find exact or fuzzy match
            let matcher = SkimMatcherV2::default();
            let mut matches: Vec<(usize, i64)> = items
                .iter()
                .enumerate()
                .filter_map(|(idx, item)| {
                    matcher.fuzzy_match(item, &input).map(|score| (idx, score))
                })
                .collect();
            
            matches.sort_by(|a, b| b.1.cmp(&a.1));

            if let Some((idx, _)) = matches.first() {
                let project = &projects[*idx];
                // Navigate directly to the project root
                println!("\n{} {}", messages.switching_to_project(), project.name);
                println!("  cd {}", project.path.display());
                
                // Run pnpm install
                let output = std::process::Command::new("pnpm")
                    .arg("install")
                    .current_dir(&project.path)
                    .output();

                match output {
                    Ok(output) if output.status.success() => {
                        println!("{}", messages.deps_installed());
                    }
                    _ => {
                        eprintln!("{}", messages.pnpm_install_warning());
                    }
                }
            }
        }
        SelectorAction::Delete(_) | SelectorAction::Cancel => {
            // Do nothing for delete (not supported for projects) or cancel
        }
    }

    Ok(())
}

pub fn show_worktree_selector(repo_root: &Path) -> Result<()> {
    let messages = crate::i18n::Messages::new();
    let worktrees = git::list_worktrees(repo_root)?;

    let items: Vec<String> = worktrees
        .iter()
        .map(|wt| {
            let marker = if wt.is_main { " (main)" } else { "" };
            wt.branch.clone() + marker
        })
        .collect();

    let action = run_input_selector(messages.select_or_create_worktree(), &items, true, true, &messages)?;

    match action {
        SelectorAction::Select(input) => {
            if input.is_empty() {
                return Ok(());
            }

            // Check if user explicitly wants to create new branch (Ctrl+B)
            let (branch_name, force_create) = if input.starts_with("__CREATE_NEW__") {
                (input.trim_start_matches("__CREATE_NEW__").to_string(), true)
            } else {
                (input, false)
            };

            if force_create {
                // Explicitly create new worktree
                println!("\n{} {}", messages.creating_new_worktree(), branch_name);
                worktree::handle_worktree(repo_root, &branch_name)?;
            } else {
                // Check for exact match (case-insensitive)
                let exact_match = worktrees.iter().find(|wt| 
                    wt.branch.eq_ignore_ascii_case(&branch_name)
                );

                if let Some(wt) = exact_match {
                    // Existing worktree - switch to it
                    println!("\n{} {}", messages.switching_to_worktree(), wt.branch);
                    println!("  cd {}", wt.path.display());
                    
                    // Run pnpm install
                    let output = std::process::Command::new("pnpm")
                        .arg("install")
                        .current_dir(&wt.path)
                        .output();

                    match output {
                        Ok(output) if output.status.success() => {
                            println!("{}", messages.deps_installed());
                        }
                        _ => {
                            eprintln!("{}", messages.pnpm_install_warning());
                        }
                    }
                } else {
                    // No exact match - this shouldn't happen with new logic
                    println!("\n{} {}", messages.creating_new_worktree(), branch_name);
                    worktree::handle_worktree(repo_root, &branch_name)?;
                }
            }
        }
        SelectorAction::Delete(branch_name) => {
            // Find the worktree to delete
            let worktree_to_delete = worktrees.iter().find(|wt| 
                wt.branch.eq_ignore_ascii_case(&branch_name)
            );

            if let Some(wt) = worktree_to_delete {
                if wt.is_main {
                    eprintln!("{}", messages.cannot_delete_main());
                } else {
                    println!("\n{} {}", messages.deleting_worktree(), wt.branch);
                    match git::remove_worktree(repo_root, &wt.path) {
                        Ok(_) => {
                            println!("{}", messages.worktree_deleted().replace("{}", &wt.branch));
                        }
                        Err(e) => {
                            eprintln!("\n{} {}", messages.failed_to_delete(), e);
                            eprintln!("\n{}", messages.uncommitted_changes_tip());
                            eprintln!("{} {}", messages.force_delete_command(), wt.path.display());
                        }
                    }
                }
            }
        }
        SelectorAction::Cancel => {
            // Do nothing
        }
    }

    Ok(())
}

#[derive(Debug)]
enum SelectorAction {
    Select(String),
    Delete(String),
    Cancel,
}

fn run_input_selector(title: &str, items: &[String], allow_create: bool, allow_delete: bool, messages: &crate::i18n::Messages) -> Result<SelectorAction> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut input = String::new();
    let matcher = SkimMatcherV2::default();

    let result = loop {
        let filtered_items: Vec<(String, i64)> = if input.is_empty() {
            items.iter().map(|s| (s.clone(), 0)).collect()
        } else {
            let mut matches: Vec<(String, i64)> = items
                .iter()
                .filter_map(|item| {
                    matcher.fuzzy_match(item, &input).map(|score| (item.clone(), score))
                })
                .collect();
            matches.sort_by(|a, b| b.1.cmp(&a.1));
            matches
        };

        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(3),
                ])
                .split(f.area());

            // Title
            let title_block = Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Cyan));
            let title_text = Paragraph::new(title)
                .block(title_block)
                .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
            f.render_widget(title_text, chunks[0]);

            // Input field
            let input_block = Block::default()
                .borders(Borders::ALL)
                .title(messages.help_search())
                .style(Style::default().fg(Color::Yellow));
            let input_text = Paragraph::new(input.as_str())
                .block(input_block)
                .style(Style::default().fg(Color::White));
            f.render_widget(input_text, chunks[1]);

            // Filtered list
            let list_items: Vec<ListItem> = if filtered_items.is_empty() && !input.is_empty() {
                vec![ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("→ Create new: '{}'", input),
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                    )
                ]))]
            } else {
                filtered_items
                    .iter()
                    .take(10)
                    .map(|(item, _)| {
                        ListItem::new(Line::from(vec![Span::raw(item)]))
                    })
                    .collect()
            };

            let list = List::new(list_items)
                .block(Block::default().borders(Borders::ALL).title(format!("Matches ({})", filtered_items.len())))
                .style(Style::default().fg(Color::White));
            f.render_widget(list, chunks[2]);

            // Help
            let help_text = if allow_create && allow_delete {
                // Check if input exactly matches an item
                let has_exact_match = items.iter().any(|item| {
                    let item_name = item.split(" (").next().unwrap_or(item);
                    item_name.eq_ignore_ascii_case(&input)
                });

                if input.is_empty() {
                    format!("{} | {} | {} | {} | {} {} | {}", 
                        messages.help_search(), messages.help_tab(), messages.help_enter_select(), 
                        messages.help_ctrl_b_create(), messages.help_ctrl_x_delete(), messages.help_exact_match(), messages.help_cancel())
                } else if filtered_items.is_empty() {
                    format!("{} | {} | {}", messages.help_create_new_branch(), messages.help_backspace(), messages.help_cancel())
                } else if has_exact_match {
                    format!("{} | {} | {} | {} | {} | {}", 
                        messages.help_tab(), messages.help_enter_select(), messages.help_ctrl_b_create(), 
                        messages.help_ctrl_x_delete(), messages.help_backspace(), messages.help_cancel())
                } else {
                     format!("{} | {} | {} | {} | {}", 
                        messages.help_tab(), messages.help_enter_select(), messages.help_ctrl_b_create(), 
                        messages.help_backspace(), messages.help_cancel())
                }
            } else if allow_create {
                if input.is_empty() {
                     format!("{} | {} | {} | {} | {}", 
                        messages.help_search(), messages.help_tab(), messages.help_enter_select(), 
                        messages.help_ctrl_b_create(), messages.help_cancel())
                } else if filtered_items.is_empty() {
                    format!("{} | {} | {}", messages.help_create_new_branch(), messages.help_backspace(), messages.help_cancel())
                } else {
                     format!("{} | {} | {} | {} | {}", 
                        messages.help_tab(), messages.help_enter_select(), messages.help_ctrl_b_create(), 
                        messages.help_backspace(), messages.help_cancel())
                }
            } else {
                 format!("{} | {} | {} | {}", 
                    messages.help_search(), messages.help_tab(), messages.help_enter_select(), messages.help_cancel())
            };
            
            let help = Paragraph::new(help_text)
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center);
            f.render_widget(help, chunks[3]);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        break SelectorAction::Cancel;
                    }
                    KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        // Ctrl+B: Create new branch with current input (only if allowed)
                        if allow_create && !input.is_empty() {
                            break SelectorAction::Select(format!("__CREATE_NEW__{}", input));
                        }
                    }
                    KeyCode::Char('x') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        // Ctrl+X: Delete exact match (only if allowed and input exactly matches)
                        if allow_delete && !input.is_empty() {
                            // Check for exact match
                            let exact_match = items.iter().find(|item| {
                                let item_name = item.split(" (").next().unwrap_or(item);
                                item_name.eq_ignore_ascii_case(&input)
                            });

                            if let Some(matched) = exact_match {
                                let branch = matched.split(" (").next().unwrap_or(matched).to_string();
                                break SelectorAction::Delete(branch);
                            }
                        }
                    }
                    KeyCode::Esc => break SelectorAction::Cancel,
                    KeyCode::Char(c) => {
                        input.push(c);
                    }
                    KeyCode::Backspace => {
                        input.pop();
                    }
                    KeyCode::Tab => {
                        // Autocomplete with top match
                        if let Some((matched, _)) = filtered_items.first() {
                            // Extract branch name (remove markers like " (main)")
                            let branch = matched.split(" (").next().unwrap_or(matched).to_string();
                            input = branch;
                        }
                    }
                    KeyCode::Enter => {
                        // Select top fuzzy match
                        if let Some((matched, _)) = filtered_items.first() {
                            // Extract branch name (remove markers like " (main)")
                            let branch = matched.split(" (").next().unwrap_or(matched).to_string();
                            break SelectorAction::Select(branch);
                        }
                    }
                    _ => {}
                }
            }
        }
    };

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(result)
}
