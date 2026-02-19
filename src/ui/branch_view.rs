use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{AppState, BranchInputMode, ConfirmDialog};
use crate::domain::BranchEntry;

/// Branch View を描画する
pub fn render(f: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // メインコンテンツ
            Constraint::Length(1), // ステータスバー
        ])
        .split(f.area());

    render_branch_list(f, state, chunks[0]);
    render_status_bar(f, state, chunks[1]);

    // 確認ダイアログがあれば描画
    if let ConfirmDialog::CheckoutBranch { branch_name } = &state.confirm_dialog {
        render_checkout_confirm_dialog(f, state, branch_name);
    }

    // 検索入力モードの場合
    if state.branch_input_mode == BranchInputMode::Search {
        render_search_input(f, state);
    }
}

/// Branch 一覧を描画する
fn render_branch_list(f: &mut Frame, state: &AppState, area: Rect) {
    let filtered_branches = state.filtered_branches();

    if filtered_branches.is_empty() {
        let message = if state.branch_search_query.is_empty() {
            "No branches"
        } else {
            "No matching branches"
        };
        let empty_message = Paragraph::new(message)
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).title(" Branch (5) "));
        f.render_widget(empty_message, area);
        return;
    }

    let items: Vec<ListItem> = filtered_branches
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let is_selected = index == state.branch_selected_index;
            create_branch_list_item(entry, is_selected)
        })
        .collect();

    let title = if state.branch_search_query.is_empty() {
        " Branch (5) ".to_string()
    } else {
        format!(
            " Branch (5) - Filter: {} ({} matches) ",
            state.branch_search_query,
            filtered_branches.len()
        )
    };

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(title));

    f.render_widget(list, area);
}

/// Branch エントリのリストアイテムを作成
fn create_branch_list_item(entry: &BranchEntry, is_selected: bool) -> ListItem<'static> {
    let mut spans = Vec::new();

    // 現在のブランチマーカー
    if entry.is_current {
        spans.push(Span::styled(
            "* ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ));
    } else {
        spans.push(Span::raw("  "));
    }

    // リモートブランチのインジケータ
    if entry.is_remote {
        spans.push(Span::styled(
            "[remote] ",
            Style::default().fg(Color::Magenta),
        ));
    }

    // ブランチ名
    let name_style = if entry.is_current {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else if entry.is_remote {
        Style::default().fg(Color::Cyan)
    } else if is_selected {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    spans.push(Span::styled(entry.name.clone(), name_style));

    let line = Line::from(spans);

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
        .unwrap_or("j/k:move Enter:checkout /:search R:refresh q:quit");

    let status_bar = Paragraph::new(message).style(Style::default().fg(Color::Cyan));

    f.render_widget(status_bar, area);
}

/// 検索入力を描画
fn render_search_input(f: &mut Frame, state: &AppState) {
    let area = search_input_rect(f.area());

    let input_line = Line::from(vec![
        Span::styled("/", Style::default().fg(Color::Yellow)),
        Span::raw(&state.branch_search_query),
        Span::styled("_", Style::default().bg(Color::White).fg(Color::Black)),
    ]);

    let search_bar = Paragraph::new(input_line)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Search ")
                .style(Style::default().bg(Color::Black)),
        )
        .style(Style::default().bg(Color::Black));

    f.render_widget(ratatui::widgets::Clear, area);
    f.render_widget(search_bar, area);
}

/// ブランチチェックアウト確認ダイアログを描画
fn render_checkout_confirm_dialog(f: &mut Frame, state: &AppState, branch_name: &str) {
    let area = super::centered_rect(50, 25, f.area());
    super::render_confirm_dialog(
        f,
        "Switch branch?",
        &format!("Branch: {}", branch_name),
        state.confirm_selected_yes,
        area,
    );
}

/// 検索入力用の矩形を計算（画面下部）
fn search_input_rect(area: Rect) -> Rect {
    let height = 3;
    let width = area.width.min(60);
    let x = (area.width.saturating_sub(width)) / 2;
    let y = area.height.saturating_sub(height + 2);

    Rect::new(x, y, width, height)
}
