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
use std::collections::HashMap;
use std::io;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::{db, git, worktree};

const WORKTREE_PR_PREVIEW_LIMIT: usize = 6;
const WORKTREE_PR_SEARCH_DEBOUNCE_MS: u64 = 300;

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
        .map(|p| messages.list_items_item_path(&p.name, &p.path.display().to_string()))
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
                
                crate::setup::SetupManager::run_auto_setup(&project.path)?;
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
    let action = run_worktree_selector(repo_root, &worktrees, &messages)?;

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
                    
                    crate::setup::SetupManager::run_auto_setup(&wt.path)?;
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
                    match git::remove_worktree(repo_root, &wt.path, false) {
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

fn format_worktree_item(
    worktree: &git::WorktreeInfo,
    messages: &crate::i18n::Messages,
    pull_request: Option<&git::PullRequestInfo>,
) -> String {
    let marker = if worktree.is_main {
        messages.main_marker()
    } else {
        ""
    };

    if let Some(pr) = pull_request {
        format!("{}{} #{} {}", worktree.branch, marker, pr.number, pr.title)
    } else {
        worktree.branch.clone() + marker
    }
}

fn run_worktree_selector(
    repo_root: &Path,
    worktrees: &[git::WorktreeInfo],
    messages: &crate::i18n::Messages,
) -> Result<SelectorAction> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut input = String::new();
    let matcher = SkimMatcherV2::default();
    let mut pr_cache: HashMap<String, Option<git::PullRequestInfo>> = HashMap::new();
    let mut last_input_change =
        Instant::now() - Duration::from_millis(WORKTREE_PR_SEARCH_DEBOUNCE_MS);

    let result = loop {
        let filtered_worktrees: Vec<(&git::WorktreeInfo, i64)> = if input.is_empty() {
            worktrees.iter().map(|worktree| (worktree, 0)).collect()
        } else {
            let mut matches: Vec<(&git::WorktreeInfo, i64)> = worktrees
                .iter()
                .filter_map(|worktree| {
                    let item = format_worktree_item(
                        worktree,
                        messages,
                        pr_cache
                            .get(&worktree.branch)
                            .and_then(|pull_request| pull_request.as_ref()),
                    );
                    matcher
                        .fuzzy_match(&item, &input)
                        .map(|score| (worktree, score))
                })
                .collect();
            matches.sort_by(|left, right| right.1.cmp(&left.1));
            matches
        };

        if last_input_change.elapsed() >= Duration::from_millis(WORKTREE_PR_SEARCH_DEBOUNCE_MS) {
            let missing_branches: Vec<String> = filtered_worktrees
                .iter()
                .take(WORKTREE_PR_PREVIEW_LIMIT)
                .filter_map(|(worktree, _)| {
                    if pr_cache.contains_key(&worktree.branch) {
                        None
                    } else {
                        Some(worktree.branch.clone())
                    }
                })
                .collect();

            if !missing_branches.is_empty() {
                let fetched_pull_requests =
                    git::get_open_prs_for_branches(repo_root, &missing_branches)?;
                for branch in missing_branches {
                    pr_cache.insert(branch.clone(), fetched_pull_requests.get(&branch).cloned());
                }
            }
        }

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

            let title_block = Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Cyan));
            let title_text = Paragraph::new(messages.select_or_create_worktree())
                .block(title_block)
                .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
            f.render_widget(title_text, chunks[0]);

            let input_block = Block::default()
                .borders(Borders::ALL)
                .title(messages.help_search())
                .style(Style::default().fg(Color::Yellow));
            let input_text = Paragraph::new(input.as_str())
                .block(input_block)
                .style(Style::default().fg(Color::White));
            f.render_widget(input_text, chunks[1]);

            let list_items: Vec<ListItem> = if filtered_worktrees.is_empty() && !input.is_empty() {
                vec![ListItem::new(Line::from(vec![Span::styled(
                    messages.create_new_prefix().replace("{}", &input),
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                )]))]
            } else {
                filtered_worktrees
                    .iter()
                    .take(10)
                    .map(|(worktree, _)| {
                        ListItem::new(Line::from(vec![Span::raw(format_worktree_item(
                            worktree,
                            messages,
                            pr_cache
                                .get(&worktree.branch)
                                .and_then(|pull_request| pull_request.as_ref()),
                        ))]))
                    })
                    .collect()
            };

            let list = List::new(list_items)
                .block(Block::default().borders(Borders::ALL).title(format!(
                    "{} ({})",
                    messages.matches_label(),
                    filtered_worktrees.len()
                )))
                .style(Style::default().fg(Color::White));
            f.render_widget(list, chunks[2]);

            let has_exact_match = worktrees
                .iter()
                .any(|worktree| worktree.branch.eq_ignore_ascii_case(&input));

            let help_text = if input.is_empty() {
                format!(
                    "{} | {} | {} | {} | {} {} | {}",
                    messages.help_search(),
                    messages.help_tab(),
                    messages.help_enter_select(),
                    messages.help_ctrl_b_create(),
                    messages.help_ctrl_x_delete(),
                    messages.help_exact_match(),
                    messages.help_cancel()
                )
            } else if filtered_worktrees.is_empty() {
                format!(
                    "{} | {} | {}",
                    messages.help_create_new_branch(),
                    messages.help_backspace(),
                    messages.help_cancel()
                )
            } else if has_exact_match {
                format!(
                    "{} | {} | {} | {} | {} | {}",
                    messages.help_tab(),
                    messages.help_enter_select(),
                    messages.help_ctrl_b_create(),
                    messages.help_ctrl_x_delete(),
                    messages.help_backspace(),
                    messages.help_cancel()
                )
            } else {
                format!(
                    "{} | {} | {} | {} | {}",
                    messages.help_tab(),
                    messages.help_enter_select(),
                    messages.help_ctrl_b_create(),
                    messages.help_backspace(),
                    messages.help_cancel()
                )
            };

            let help = Paragraph::new(help_text)
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center);
            f.render_widget(help, chunks[3]);
        })?;

        if !event::poll(Duration::from_millis(100))? {
            continue;
        }

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        break SelectorAction::Cancel;
                    }
                    KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if !input.is_empty() {
                            break SelectorAction::Select(format!("__CREATE_NEW__{}", input));
                        }
                    }
                    KeyCode::Char('x') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if !input.is_empty() {
                            if let Some(worktree) = worktrees
                                .iter()
                                .find(|worktree| worktree.branch.eq_ignore_ascii_case(&input))
                            {
                                break SelectorAction::Delete(worktree.branch.clone());
                            }
                        }
                    }
                    KeyCode::Esc => break SelectorAction::Cancel,
                    KeyCode::Char(character) => {
                        input.push(character);
                        last_input_change = Instant::now();
                    }
                    KeyCode::Backspace => {
                        input.pop();
                        last_input_change = Instant::now();
                    }
                    KeyCode::Tab => {
                        if let Some((worktree, _)) = filtered_worktrees.first() {
                            input = worktree.branch.clone();
                            last_input_change = Instant::now();
                        }
                    }
                    KeyCode::Enter => {
                        if let Some((worktree, _)) = filtered_worktrees.first() {
                            break SelectorAction::Select(worktree.branch.clone());
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
                        messages.create_new_prefix().replace("{}", &input),
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
                    .block(Block::default().borders(Borders::ALL).title(format!(
                        "{} ({})",
                        messages.matches_label(),
                        filtered_items.len()
                    )))
                .style(Style::default().fg(Color::White));
            f.render_widget(list, chunks[2]);

            // Help
            let help_text = if allow_create && allow_delete {
                // Check if input exactly matches an item
                let has_exact_match = items.iter().any(|item| {
                    let item_name = extract_worktree_name(item);
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
                                let item_name = extract_worktree_name(item);
                                item_name.eq_ignore_ascii_case(&input)
                            });

                            if let Some(matched) = exact_match {
                                let branch = extract_worktree_name(matched).to_string();
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
                            let branch = extract_worktree_name(matched).to_string();
                            input = branch;
                        }
                    }
                    KeyCode::Enter => {
                        // Select top fuzzy match
                        if let Some((matched, _)) = filtered_items.first() {
                            // Extract branch name (remove markers like " (main)")
                            let branch = extract_worktree_name(matched).to_string();
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

fn extract_worktree_name(item: &str) -> &str {
    let item = item.split(" #").next().unwrap_or(item);
    item.split(" (").next().unwrap_or(item)
}
