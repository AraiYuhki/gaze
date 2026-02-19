use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{AppState, ConfirmDialog, StashInputMode};
use crate::domain::StashEntry;

/// Stash View を描画する
pub fn render(f: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // メインコンテンツ
            Constraint::Length(1), // ステータスバー
        ])
        .split(f.area());

    render_stash_list(f, state, chunks[0]);
    render_status_bar(f, state, chunks[1]);

    // 確認ダイアログがあれば描画
    if let ConfirmDialog::DropStash { stash_index } = &state.confirm_dialog {
        render_drop_confirm_dialog(f, state, *stash_index);
    }

    // stash メッセージ入力ダイアログがあれば描画
    if state.stash_input_mode != StashInputMode::None {
        render_stash_input_dialog(f, state);
    }
}

/// Stash 一覧を描画する
fn render_stash_list(f: &mut Frame, state: &AppState, area: Rect) {
    if state.stash_cache.is_empty() {
        let empty_message = Paragraph::new("No stashes")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).title(" Stash (4) "));
        f.render_widget(empty_message, area);
        return;
    }

    let items: Vec<ListItem> = state
        .stash_cache
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let is_selected = index == state.stash_selected_index;
            create_stash_list_item(entry, is_selected)
        })
        .collect();

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(" Stash (4) "));

    f.render_widget(list, area);
}

/// Stash エントリのリストアイテムを作成
fn create_stash_list_item(entry: &StashEntry, is_selected: bool) -> ListItem<'static> {
    // stash@{N}
    let index_span = Span::styled(
        format!("stash@{{{}}}", entry.index),
        Style::default().fg(Color::Yellow),
    );

    // ブランチ名
    let branch_span = if entry.branch.is_empty() {
        Span::raw("")
    } else {
        Span::styled(
            format!(" on {}", entry.branch),
            Style::default().fg(Color::Cyan),
        )
    };

    // メッセージ
    let message_style = if is_selected {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    let message_span = Span::styled(format!(": {}", entry.message), message_style);

    let line = Line::from(vec![index_span, branch_span, message_span]);

    let item = ListItem::new(line);
    if is_selected {
        item.style(Style::default().bg(Color::Rgb(60, 60, 80)))
    } else {
        item
    }
}

/// ステータスバーを描画する
fn render_status_bar(f: &mut Frame, state: &AppState, area: Rect) {
    let message = state
        .status_message
        .as_deref()
        .unwrap_or("j/k:move s:stash p:pop a:apply d:drop Enter:show R:refresh q:quit");

    let status_bar = Paragraph::new(message).style(Style::default().fg(Color::Cyan));

    f.render_widget(status_bar, area);
}

/// Stash 削除確認ダイアログを描画
fn render_drop_confirm_dialog(f: &mut Frame, state: &AppState, stash_index: usize) {
    let stash_name = state
        .stash_cache
        .get(stash_index)
        .map(|s| format!("stash@{{{}}}", s.index))
        .unwrap_or_default();

    let area = super::centered_rect(50, 25, f.area());
    super::render_confirm_dialog(
        f,
        "Drop stash?",
        &format!("Stash: {}", stash_name),
        state.confirm_selected_yes,
        area,
    );
}

/// Stash メッセージ入力ダイアログを描画
fn render_stash_input_dialog(f: &mut Frame, state: &AppState) {
    let area = super::centered_rect(60, 30, f.area());

    let title = match state.stash_input_mode {
        StashInputMode::Push => " Stash Message (optional) ",
        StashInputMode::None => "",
    };

    let input_line = Line::from(vec![
        Span::raw(&state.stash_message),
        Span::styled("_", Style::default().bg(Color::White).fg(Color::Black)),
    ]);

    let text = vec![
        Line::from(""),
        Line::from("Enter a message for this stash (leave empty for default):"),
        Line::from(""),
        input_line,
        Line::from(""),
        Line::from(Span::styled(
            "Enter: confirm, Esc: cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let dialog = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .style(Style::default().bg(Color::Black)),
        )
        .style(Style::default().bg(Color::Black));

    f.render_widget(ratatui::widgets::Clear, area);
    f.render_widget(dialog, area);
}
