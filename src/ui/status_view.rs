use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
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
    if let ConfirmDialog::DiscardChanges { file_index } = &state.confirm_dialog {
        render_confirm_dialog(f, state, *file_index);
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
                item.style(Style::default().bg(Color::DarkGray))
            } else {
                item
            }
        })
        .collect();

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(" Status (1) "));

    f.render_widget(list, area);
}

/// ステータスバーを描画する
fn render_status_bar(f: &mut Frame, state: &AppState, area: Rect) {
    let message = state
        .status_message
        .as_deref()
        .unwrap_or("j/k:move s:stage d:diff r:discard R:refresh q:quit");

    let status_bar = Paragraph::new(message).style(Style::default().fg(Color::Cyan));

    f.render_widget(status_bar, area);
}

/// 確認ダイアログを描画する
fn render_confirm_dialog(f: &mut Frame, state: &AppState, file_index: usize) {
    let area = centered_rect(60, 20, f.area());

    let file_path = state
        .status_cache
        .get(file_index)
        .map(|f| f.path.display().to_string())
        .unwrap_or_default();

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Discard changes?",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(format!("File: {}", file_path)),
        Line::from(""),
        Line::from("Press 'y' to confirm, 'n' to cancel"),
    ];

    let dialog = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Confirm ")
                .style(Style::default().bg(Color::Black)),
        )
        .style(Style::default().bg(Color::Black));

    // 背景を塗りつぶしてからダイアログを描画
    f.render_widget(ratatui::widgets::Clear, area);
    f.render_widget(dialog, area);
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

/// 中央揃えの矩形を計算する
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
