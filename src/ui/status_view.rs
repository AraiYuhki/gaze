use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::{AppState, ConfirmDialog};
use crate::domain::StatusKind;

/// Status View を描画する
pub fn render(f: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // メインコンテンツ
            Constraint::Length(1), // ステータスバー
        ])
        .split(f.area());

    render_file_list(f, state, chunks[0]);
    render_status_bar(f, state, chunks[1]);

    // 確認ダイアログがあれば描画
    match &state.confirm_dialog {
        ConfirmDialog::DiscardChanges { file_index } => {
            render_discard_confirm_dialog(f, state, *file_index);
        }
        ConfirmDialog::Push => {
            render_push_pull_dialog(f, state, "Push to remote?");
        }
        ConfirmDialog::Pull => {
            render_push_pull_dialog(f, state, "Pull from remote?");
        }
        ConfirmDialog::Amend => {
            render_amend_dialog(f, state);
        }
        _ => {}
    }
}

/// ファイル一覧を描画する
fn render_file_list(f: &mut Frame, state: &AppState, area: Rect) {
    let items: Vec<ListItem> = state
        .status_cache
        .iter()
        .enumerate()
        .map(|(index, file)| {
            let is_selected = index == state.selected_index;

            // ステータス表示用の文字と色
            let (index_char, index_color) = status_to_char_and_color(file.index);
            let (worktree_char, worktree_color) = status_to_char_and_color(file.worktree);

            // ステータス文字（インデックス + ワークツリー）
            let status_span = Span::styled(
                format!("{}{}", index_char, worktree_char),
                Style::default().fg(if file.index != StatusKind::Unmodified {
                    index_color
                } else {
                    worktree_color
                }),
            );

            // ファイルパス
            let path_style = if is_selected {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let path_span = Span::styled(format!(" {}", file.path.display()), path_style);

            let line = Line::from(vec![status_span, path_span]);

            let item = ListItem::new(line);
            if is_selected {
                item.style(Style::default().bg(Color::Rgb(60, 60, 80)))
            } else {
                item
            }
        })
        .collect();

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(" Status (1) "));

    let mut list_state = ListState::default().with_selected(Some(state.selected_index));
    f.render_stateful_widget(list, area, &mut list_state);
}

/// ステータスバーを描画する
fn render_status_bar(f: &mut Frame, state: &AppState, area: Rect) {
    let message = state.status_message.as_deref().unwrap_or(
        "j/k:move s/S:stage u/W:unstage d:diff H:hunk r:discard c:commit P:push U:pull q:quit",
    );

    let status_bar = Paragraph::new(message).style(Style::default().fg(Color::Cyan));

    f.render_widget(status_bar, area);
}

/// 確認ダイアログを描画する
fn render_discard_confirm_dialog(f: &mut Frame, state: &AppState, file_index: usize) {
    let file_path = state
        .status_cache
        .get(file_index)
        .map(|f| f.path.display().to_string())
        .unwrap_or_default();

    let area = super::centered_rect(50, 20, f.area());
    super::render_confirm_dialog(
        f,
        "Discard changes?",
        &format!("File: {}", file_path),
        state.confirm_selected_yes,
        area,
    );
}

/// Push/Pull 確認ダイアログを描画する
fn render_push_pull_dialog(f: &mut Frame, state: &AppState, message: &str) {
    let area = super::centered_rect(50, 20, f.area());
    super::render_confirm_dialog(f, message, "", state.confirm_selected_yes, area);
}

/// Amend 確認ダイアログを描画する
fn render_amend_dialog(f: &mut Frame, state: &AppState) {
    let area = super::centered_rect(50, 20, f.area());
    super::render_confirm_dialog(
        f,
        "Amend last commit?",
        "",
        state.confirm_selected_yes,
        area,
    );
}

/// StatusKind を表示文字と色に変換する
fn status_to_char_and_color(kind: StatusKind) -> (char, Color) {
    match kind {
        StatusKind::Modified => ('M', Color::Yellow),
        StatusKind::Added => ('A', Color::Green),
        StatusKind::Deleted => ('D', Color::Red),
        StatusKind::Renamed => ('R', Color::Magenta),
        StatusKind::Copied => ('C', Color::Magenta),
        StatusKind::Untracked => ('?', Color::Gray),
        StatusKind::Ignored => ('!', Color::DarkGray),
        StatusKind::Unmodified => (' ', Color::White),
    }
}
