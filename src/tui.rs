use anyhow::Result;
use crossterm::{
    cursor::Show,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};
use std::collections::HashMap;
use std::io;
use std::path::Path;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;

use crate::{db, git, worktree};

struct TerminalCleanup;

impl Drop for TerminalCleanup {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, Show);
    }
}

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
                crate::maybe_print_shell_cd_marker(&project.path);
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
    let (action, worktrees) = run_worktree_selector(repo_root, &messages)?;

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
                let exact_match = worktrees.iter().find(|wt| wt.matches_name(&branch_name));

                if let Some(wt) = exact_match {
                    // Existing worktree - switch to it
                    println!("\n{} {}", messages.switching_to_worktree(), wt.name());
                    crate::maybe_print_shell_cd_marker(&wt.path);
                    println!("  cd {}", wt.path.display());

                    crate::setup::SetupManager::run_post_cd(repo_root, &wt.path)?;
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
            let worktree_to_delete = worktrees.iter().find(|wt| wt.matches_name(&branch_name));

            if let Some(wt) = worktree_to_delete {
                if wt.is_main {
                    return Err(anyhow::anyhow!(messages.cannot_delete_main().to_string()));
                } else {
                    println!("\n{} {}", messages.deleting_worktree(), wt.name());
                    match git::remove_worktree(repo_root, &wt.path, false) {
                        Ok(_) => {
                            println!("{}", messages.worktree_deleted().replace("{}", wt.name()));
                            let _ = git::prune_worktrees(repo_root);
                        }
                        Err(e) => {
                            eprintln!("\n{}", messages.uncommitted_changes_tip());
                            eprintln!(
                                "{} wt delete {} --force",
                                messages.force_delete_command(),
                                wt.name()
                            );
                            return Err(e);
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

#[derive(Debug, Default)]
struct InputState {
    input: String,
    cursor_index: usize,
    selected_index: usize,
}

enum BackgroundMessage {
    WorktreesLoaded(Result<Vec<git::WorktreeInfo>, String>),
    PullRequestsLoaded(Result<HashMap<String, git::PullRequestInfo>, String>),
}

enum WorktreeLoadState {
    Loading,
    Loaded,
    Failed(String),
}

enum PullRequestLoadState {
    NotStarted,
    Loading,
    Loaded,
    Failed,
}

fn byte_index(value: &str, char_index: usize) -> usize {
    value
        .char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or_else(|| value.len())
}

fn char_len(value: &str) -> usize {
    value.chars().count()
}

fn insert_char(state: &mut InputState, character: char) {
    let byte_index = byte_index(&state.input, state.cursor_index);
    state.input.insert(byte_index, character);
    state.cursor_index += 1;
}

fn backspace_char(state: &mut InputState) {
    if state.cursor_index == 0 {
        return;
    }

    let end = byte_index(&state.input, state.cursor_index);
    let start = byte_index(&state.input, state.cursor_index - 1);
    state.input.replace_range(start..end, "");
    state.cursor_index -= 1;
}

fn delete_char(state: &mut InputState) {
    if state.cursor_index >= char_len(&state.input) {
        return;
    }

    let start = byte_index(&state.input, state.cursor_index);
    let end = byte_index(&state.input, state.cursor_index + 1);
    state.input.replace_range(start..end, "");
}

fn clamp_selected_index(selected_index: &mut usize, visible_len: usize) {
    if visible_len == 0 {
        *selected_index = 0;
    } else if *selected_index >= visible_len {
        *selected_index = visible_len - 1;
    }
}

fn selection_highlight_style() -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}

fn spawn_worktree_loader(repo_root: &Path, sender: Sender<BackgroundMessage>) {
    let repo_root = repo_root.to_path_buf();
    thread::spawn(move || {
        let result = git::list_worktrees(&repo_root).map_err(|error| error.to_string());
        let _ = sender.send(BackgroundMessage::WorktreesLoaded(result));
    });
}

fn spawn_pull_request_loader(repo_root: &Path, sender: Sender<BackgroundMessage>) {
    let repo_root = repo_root.to_path_buf();
    thread::spawn(move || {
        let result = git::get_open_prs_by_branch(&repo_root).map_err(|error| error.to_string());
        let _ = sender.send(BackgroundMessage::PullRequestsLoaded(result));
    });
}

fn filter_worktrees<'a>(
    worktrees: &'a [git::WorktreeInfo],
    input: &str,
    matcher: &SkimMatcherV2,
    messages: &crate::i18n::Messages,
    pr_cache: &HashMap<String, git::PullRequestInfo>,
) -> Vec<(&'a git::WorktreeInfo, i64)> {
    if input.is_empty() {
        return worktrees.iter().map(|worktree| (worktree, 0)).collect();
    }

    let mut matches: Vec<(&git::WorktreeInfo, i64)> = worktrees
        .iter()
        .filter_map(|worktree| {
            let item = format_worktree_item(
                worktree,
                messages,
                worktree
                    .branch_name()
                    .and_then(|branch| pr_cache.get(branch)),
            );
            matcher
                .fuzzy_match(&item, input)
                .map(|score| (worktree, score))
        })
        .collect();
    matches.sort_by(|left, right| right.1.cmp(&left.1));
    matches
}

fn selected_worktree_branch(
    worktrees: &[git::WorktreeInfo],
    state: &InputState,
    matcher: &SkimMatcherV2,
    messages: &crate::i18n::Messages,
    pr_cache: &HashMap<String, git::PullRequestInfo>,
) -> Option<String> {
    filter_worktrees(worktrees, &state.input, matcher, messages, pr_cache)
        .get(state.selected_index)
        .map(|(worktree, _)| worktree.name().to_string())
}

fn worktree_status_text(
    messages: &crate::i18n::Messages,
    worktree_state: &WorktreeLoadState,
    pr_state: &PullRequestLoadState,
) -> Option<String> {
    match worktree_state {
        WorktreeLoadState::Loading => Some(messages.loading_worktrees().to_string()),
        WorktreeLoadState::Failed(_) => Some(messages.failed_list_worktrees().to_string()),
        WorktreeLoadState::Loaded => match pr_state {
            PullRequestLoadState::Loading => Some(messages.loading_pull_requests().to_string()),
            PullRequestLoadState::Failed => Some(messages.pr_preview_unavailable().to_string()),
            PullRequestLoadState::NotStarted | PullRequestLoadState::Loaded => None,
        },
    }
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
        format!("{}{} #{} {}", worktree.name(), marker, pr.number, pr.title)
    } else {
        worktree.name().to_string() + marker
    }
}

fn run_worktree_selector(
    repo_root: &Path,
    messages: &crate::i18n::Messages,
) -> Result<(SelectorAction, Vec<git::WorktreeInfo>)> {
    enable_raw_mode()?;
    let _cleanup = TerminalCleanup;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (background_tx, background_rx) = mpsc::channel();
    spawn_worktree_loader(repo_root, background_tx.clone());

    let mut state = InputState::default();
    let matcher = SkimMatcherV2::default();
    let mut worktrees = Vec::new();
    let mut worktree_state = WorktreeLoadState::Loading;
    let mut pr_cache: HashMap<String, git::PullRequestInfo> = HashMap::new();
    let mut pr_state = PullRequestLoadState::NotStarted;

    let result = loop {
        let selected_branch_before_update = match &worktree_state {
            WorktreeLoadState::Loaded => {
                selected_worktree_branch(&worktrees, &state, &matcher, messages, &pr_cache)
            }
            WorktreeLoadState::Loading | WorktreeLoadState::Failed(_) => None,
        };
        let mut should_restore_selection = false;

        while let Ok(message) = background_rx.try_recv() {
            match message {
                BackgroundMessage::WorktreesLoaded(Ok(loaded_worktrees)) => {
                    worktrees = loaded_worktrees;
                    worktree_state = WorktreeLoadState::Loaded;
                    should_restore_selection = true;

                    if matches!(&pr_state, PullRequestLoadState::NotStarted) {
                        pr_state = PullRequestLoadState::Loading;
                        spawn_pull_request_loader(repo_root, background_tx.clone());
                    }
                }
                BackgroundMessage::WorktreesLoaded(Err(error)) => {
                    worktree_state = WorktreeLoadState::Failed(error);
                }
                BackgroundMessage::PullRequestsLoaded(Ok(loaded_pull_requests)) => {
                    pr_cache = loaded_pull_requests;
                    pr_state = PullRequestLoadState::Loaded;
                    should_restore_selection = true;
                }
                BackgroundMessage::PullRequestsLoaded(Err(_error)) => {
                    pr_state = PullRequestLoadState::Failed;
                }
            }
        }

        let filtered_worktrees = match &worktree_state {
            WorktreeLoadState::Loaded => {
                filter_worktrees(&worktrees, &state.input, &matcher, messages, &pr_cache)
            }
            WorktreeLoadState::Loading | WorktreeLoadState::Failed(_) => Vec::new(),
        };

        if should_restore_selection {
            if let Some(selected_branch) = selected_branch_before_update {
                if let Some(index) = filtered_worktrees
                    .iter()
                    .position(|(worktree, _)| worktree.name() == selected_branch)
                {
                    state.selected_index = index;
                }
            }
        }

        let visible_len = filtered_worktrees.len().min(10);
        clamp_selected_index(&mut state.selected_index, visible_len);

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
                .style(
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                );
            f.render_widget(title_text, chunks[0]);

            let input_block = Block::default()
                .borders(Borders::ALL)
                .title(messages.help_search())
                .style(Style::default().fg(Color::Yellow));
            let input_text = Paragraph::new(state.input.as_str())
                .block(input_block)
                .style(Style::default().fg(Color::White));
            f.render_widget(input_text, chunks[1]);
            f.set_cursor_position((chunks[1].x + 1 + state.cursor_index as u16, chunks[1].y + 1));

            let list_items: Vec<ListItem> = match &worktree_state {
                WorktreeLoadState::Loading => vec![ListItem::new(Line::from(vec![Span::styled(
                    messages.loading_worktrees(),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::ITALIC),
                )]))],
                WorktreeLoadState::Failed(error) => {
                    vec![ListItem::new(Line::from(vec![Span::styled(
                        format!("{}: {}", messages.failed_list_worktrees(), error),
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    )]))]
                }
                WorktreeLoadState::Loaded
                    if filtered_worktrees.is_empty() && !state.input.is_empty() =>
                {
                    vec![ListItem::new(Line::from(vec![Span::styled(
                        messages.create_new_prefix().replace("{}", &state.input),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    )]))]
                }
                WorktreeLoadState::Loaded => filtered_worktrees
                    .iter()
                    .take(10)
                    .map(|(worktree, _)| {
                        ListItem::new(Line::from(vec![Span::raw(format_worktree_item(
                            worktree,
                            messages,
                            worktree
                                .branch_name()
                                .and_then(|branch| pr_cache.get(branch)),
                        ))]))
                    })
                    .collect(),
            };

            let list = List::new(list_items)
                .block(Block::default().borders(Borders::ALL).title(format!(
                    "{} ({})",
                    messages.matches_label(),
                    filtered_worktrees.len()
                )))
                .style(Style::default().fg(Color::White))
                .highlight_style(selection_highlight_style())
                .highlight_symbol("> ");
            let mut list_state = ListState::default();
            if visible_len > 0 {
                list_state.select(Some(state.selected_index));
            }
            f.render_stateful_widget(list, chunks[2], &mut list_state);

            let has_exact_match = worktrees
                .iter()
                .any(|worktree| worktree.matches_name(&state.input));
            let status_text = worktree_status_text(messages, &worktree_state, &pr_state);

            let base_help_text = if !matches!(&worktree_state, WorktreeLoadState::Loaded) {
                format!("{} | {}", messages.help_search(), messages.help_cancel())
            } else if state.input.is_empty() {
                format!(
                    "{} | {} | {} | {} | {} {} | {} | {}",
                    messages.help_search(),
                    messages.help_tab(),
                    messages.help_enter_select(),
                    messages.help_ctrl_b_create(),
                    messages.help_ctrl_x_delete(),
                    messages.help_exact_match(),
                    messages.help_cancel(),
                    "Arrows"
                )
            } else if filtered_worktrees.is_empty() {
                format!(
                    "{} | {} | {} | {}",
                    messages.help_create_new_branch(),
                    messages.help_backspace(),
                    messages.help_cancel(),
                    "Arrows"
                )
            } else if has_exact_match {
                format!(
                    "{} | {} | {} | {} | {} | {} | {}",
                    messages.help_tab(),
                    messages.help_enter_select(),
                    messages.help_ctrl_b_create(),
                    messages.help_ctrl_x_delete(),
                    messages.help_backspace(),
                    messages.help_cancel(),
                    "Arrows"
                )
            } else {
                format!(
                    "{} | {} | {} | {} | {} | {}",
                    messages.help_tab(),
                    messages.help_enter_select(),
                    messages.help_ctrl_b_create(),
                    messages.help_backspace(),
                    messages.help_cancel(),
                    "Arrows"
                )
            };
            let help_text = match status_text {
                Some(status_text) => format!("{} | {}", base_help_text, status_text),
                None => base_help_text,
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
                let worktrees_loaded = matches!(&worktree_state, WorktreeLoadState::Loaded);

                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        break SelectorAction::Cancel;
                    }
                    KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if worktrees_loaded && !state.input.is_empty() {
                            break SelectorAction::Select(format!("__CREATE_NEW__{}", state.input));
                        }
                    }
                    KeyCode::Char('x') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if worktrees_loaded && !state.input.is_empty() {
                            if let Some(worktree) = worktrees
                                .iter()
                                .find(|worktree| worktree.matches_name(&state.input))
                            {
                                break SelectorAction::Delete(worktree.name().to_string());
                            }
                        }
                    }
                    KeyCode::Esc => break SelectorAction::Cancel,
                    KeyCode::Char(character) => {
                        insert_char(&mut state, character);
                        state.selected_index = 0;
                    }
                    KeyCode::Backspace => {
                        backspace_char(&mut state);
                        state.selected_index = 0;
                    }
                    KeyCode::Delete => {
                        delete_char(&mut state);
                        state.selected_index = 0;
                    }
                    KeyCode::Left => {
                        state.cursor_index = state.cursor_index.saturating_sub(1);
                    }
                    KeyCode::Right => {
                        state.cursor_index = (state.cursor_index + 1).min(char_len(&state.input));
                    }
                    KeyCode::Home => {
                        state.cursor_index = 0;
                    }
                    KeyCode::End => {
                        state.cursor_index = char_len(&state.input);
                    }
                    KeyCode::Up => {
                        state.selected_index = state.selected_index.saturating_sub(1);
                    }
                    KeyCode::Down => {
                        if visible_len > 0 {
                            state.selected_index = (state.selected_index + 1).min(visible_len - 1);
                        }
                    }
                    KeyCode::Tab => {
                        if worktrees_loaded {
                            if let Some((worktree, _)) =
                                filtered_worktrees.get(state.selected_index)
                            {
                                state.input = worktree.name().to_string();
                                state.cursor_index = char_len(&state.input);
                            }
                        }
                    }
                    KeyCode::Enter => {
                        if worktrees_loaded {
                            if let Some((worktree, _)) =
                                filtered_worktrees.get(state.selected_index)
                            {
                                break SelectorAction::Select(worktree.name().to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    };

    Ok((result, worktrees))
}

fn run_input_selector(
    title: &str,
    items: &[String],
    allow_create: bool,
    allow_delete: bool,
    messages: &crate::i18n::Messages,
) -> Result<SelectorAction> {
    enable_raw_mode()?;
    let _cleanup = TerminalCleanup;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = InputState::default();
    let matcher = SkimMatcherV2::default();

    let result = loop {
        let filtered_items: Vec<(String, i64)> = if state.input.is_empty() {
            items.iter().map(|s| (s.clone(), 0)).collect()
        } else {
            let mut matches: Vec<(String, i64)> = items
                .iter()
                .filter_map(|item| {
                    matcher
                        .fuzzy_match(item, &state.input)
                        .map(|score| (item.clone(), score))
                })
                .collect();
            matches.sort_by(|a, b| b.1.cmp(&a.1));
            matches
        };
        let visible_len = filtered_items.len().min(10);
        clamp_selected_index(&mut state.selected_index, visible_len);

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
            let title_text = Paragraph::new(title).block(title_block).style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            );
            f.render_widget(title_text, chunks[0]);

            // Input field
            let input_block = Block::default()
                .borders(Borders::ALL)
                .title(messages.help_search())
                .style(Style::default().fg(Color::Yellow));
            let input_text = Paragraph::new(state.input.as_str())
                .block(input_block)
                .style(Style::default().fg(Color::White));
            f.render_widget(input_text, chunks[1]);
            f.set_cursor_position((chunks[1].x + 1 + state.cursor_index as u16, chunks[1].y + 1));

            // Filtered list
            let list_items: Vec<ListItem> = if filtered_items.is_empty() && !state.input.is_empty()
            {
                vec![ListItem::new(Line::from(vec![Span::styled(
                    messages.create_new_prefix().replace("{}", &state.input),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )]))]
            } else {
                filtered_items
                    .iter()
                    .take(10)
                    .map(|(item, _)| ListItem::new(Line::from(vec![Span::raw(item)])))
                    .collect()
            };

            let list = List::new(list_items)
                .block(Block::default().borders(Borders::ALL).title(format!(
                    "{} ({})",
                    messages.matches_label(),
                    filtered_items.len()
                )))
                .style(Style::default().fg(Color::White))
                .highlight_style(selection_highlight_style())
                .highlight_symbol("> ");
            let mut list_state = ListState::default();
            if visible_len > 0 {
                list_state.select(Some(state.selected_index));
            }
            f.render_stateful_widget(list, chunks[2], &mut list_state);

            // Help
            let help_text = if allow_create && allow_delete {
                // Check if input exactly matches an item
                let has_exact_match = items.iter().any(|item| {
                    let item_name = extract_worktree_name(item);
                    item_name.eq_ignore_ascii_case(&state.input)
                });

                if state.input.is_empty() {
                    format!(
                        "{} | {} | {} | {} | {} {} | {} | {}",
                        messages.help_search(),
                        messages.help_tab(),
                        messages.help_enter_select(),
                        messages.help_ctrl_b_create(),
                        messages.help_ctrl_x_delete(),
                        messages.help_exact_match(),
                        messages.help_cancel(),
                        "Arrows"
                    )
                } else if filtered_items.is_empty() {
                    format!(
                        "{} | {} | {} | {}",
                        messages.help_create_new_branch(),
                        messages.help_backspace(),
                        messages.help_cancel(),
                        "Arrows"
                    )
                } else if has_exact_match {
                    format!(
                        "{} | {} | {} | {} | {} | {} | {}",
                        messages.help_tab(),
                        messages.help_enter_select(),
                        messages.help_ctrl_b_create(),
                        messages.help_ctrl_x_delete(),
                        messages.help_backspace(),
                        messages.help_cancel(),
                        "Arrows"
                    )
                } else {
                    format!(
                        "{} | {} | {} | {} | {} | {}",
                        messages.help_tab(),
                        messages.help_enter_select(),
                        messages.help_ctrl_b_create(),
                        messages.help_backspace(),
                        messages.help_cancel(),
                        "Arrows"
                    )
                }
            } else if allow_create {
                if state.input.is_empty() {
                    format!(
                        "{} | {} | {} | {} | {} | {}",
                        messages.help_search(),
                        messages.help_tab(),
                        messages.help_enter_select(),
                        messages.help_ctrl_b_create(),
                        messages.help_cancel(),
                        "Arrows"
                    )
                } else if filtered_items.is_empty() {
                    format!(
                        "{} | {} | {} | {}",
                        messages.help_create_new_branch(),
                        messages.help_backspace(),
                        messages.help_cancel(),
                        "Arrows"
                    )
                } else {
                    format!(
                        "{} | {} | {} | {} | {} | {}",
                        messages.help_tab(),
                        messages.help_enter_select(),
                        messages.help_ctrl_b_create(),
                        messages.help_backspace(),
                        messages.help_cancel(),
                        "Arrows"
                    )
                }
            } else {
                format!(
                    "{} | {} | {} | {} | {}",
                    messages.help_search(),
                    messages.help_tab(),
                    messages.help_enter_select(),
                    messages.help_cancel(),
                    "Arrows"
                )
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
                        if allow_create && !state.input.is_empty() {
                            break SelectorAction::Select(format!("__CREATE_NEW__{}", state.input));
                        }
                    }
                    KeyCode::Char('x') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        // Ctrl+X: Delete exact match (only if allowed and input exactly matches)
                        if allow_delete && !state.input.is_empty() {
                            // Check for exact match
                            let exact_match = items.iter().find(|item| {
                                let item_name = extract_worktree_name(item);
                                item_name.eq_ignore_ascii_case(&state.input)
                            });

                            if let Some(matched) = exact_match {
                                let branch = extract_worktree_name(matched).to_string();
                                break SelectorAction::Delete(branch);
                            }
                        }
                    }
                    KeyCode::Esc => break SelectorAction::Cancel,
                    KeyCode::Char(c) => {
                        insert_char(&mut state, c);
                        state.selected_index = 0;
                    }
                    KeyCode::Backspace => {
                        backspace_char(&mut state);
                        state.selected_index = 0;
                    }
                    KeyCode::Delete => {
                        delete_char(&mut state);
                        state.selected_index = 0;
                    }
                    KeyCode::Left => {
                        state.cursor_index = state.cursor_index.saturating_sub(1);
                    }
                    KeyCode::Right => {
                        state.cursor_index = (state.cursor_index + 1).min(char_len(&state.input));
                    }
                    KeyCode::Home => {
                        state.cursor_index = 0;
                    }
                    KeyCode::End => {
                        state.cursor_index = char_len(&state.input);
                    }
                    KeyCode::Up => {
                        state.selected_index = state.selected_index.saturating_sub(1);
                    }
                    KeyCode::Down => {
                        if visible_len > 0 {
                            state.selected_index = (state.selected_index + 1).min(visible_len - 1);
                        }
                    }
                    KeyCode::Tab => {
                        // Autocomplete with top match
                        if let Some((matched, _)) = filtered_items.get(state.selected_index) {
                            // Extract branch name (remove markers like " (main)")
                            let branch = extract_worktree_name(matched).to_string();
                            state.input = branch;
                            state.cursor_index = char_len(&state.input);
                        }
                    }
                    KeyCode::Enter => {
                        // Select top fuzzy match
                        if let Some((matched, _)) = filtered_items.get(state.selected_index) {
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

    Ok(result)
}

fn extract_worktree_name(item: &str) -> &str {
    let item = item.split(" #").next().unwrap_or(item);
    item.split(" (").next().unwrap_or(item)
}

#[cfg(test)]
mod terminal_tests {
    use super::TerminalCleanup;

    #[test]
    fn terminal_cleanup_drop_is_safe() {
        let cleanup = TerminalCleanup;
        drop(cleanup);
    }
}
