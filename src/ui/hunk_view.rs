use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::AppState;
use crate::domain::HunkLine;

/// Hunk View を描画する
pub fn render(f: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // メインコンテンツ
            Constraint::Length(1), // ステータスバー
        ])
        .split(f.area());

    render_hunk_list(f, state, chunks[0]);
    render_status_bar(f, state, chunks[1]);
}

/// Hunk リストを描画する
fn render_hunk_list(f: &mut Frame, state: &AppState, area: Rect) {
    let file_diffs = &state.hunk_file_diffs;

    if file_diffs.is_empty() {
        let message = Paragraph::new("No diff available")
            .style(Style::default().fg(Color::DarkGray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Hunk Staging "),
            );
        f.render_widget(message, area);
        return;
    }

    // すべての hunk を行としてフラット化して表示
    let mut items: Vec<ListItem> = Vec::new();
    let mut flat_index = 0;

    for file_diff in file_diffs {
        // ファイルヘッダ
        items.push(ListItem::new(Line::from(Span::styled(
            format!("  {}", file_diff.file_path),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))));
        flat_index += 1;

        for (hunk_idx, hunk) in file_diff.hunks.iter().enumerate() {
            let is_selected = state.hunk_selected_index == flat_index;

            // hunk ヘッダ行
            let hunk_label = format!("  Hunk #{}: {}", hunk_idx + 1, hunk.header);
            let header_style = if is_selected {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Magenta)
            };

            let header_item = ListItem::new(Line::from(Span::styled(hunk_label, header_style)));
            let header_item = if is_selected {
                header_item.style(Style::default().bg(Color::Rgb(60, 60, 80)))
            } else {
                header_item
            };
            items.push(header_item);
            flat_index += 1;

            // hunk の各行（コンテキスト表示）
            for line in &hunk.lines {
                let (prefix, content, color) = match line {
                    HunkLine::Context(s) => (" ", s.as_str(), Color::White),
                    HunkLine::Added(s) => ("+", s.as_str(), Color::Green),
                    HunkLine::Removed(s) => ("-", s.as_str(), Color::Red),
                };

                items.push(ListItem::new(Line::from(Span::styled(
                    format!("    {}{}", prefix, content),
                    Style::default().fg(color),
                ))));
                flat_index += 1;
            }
        }
    }

    // ファイル名を取得（複数ファイルの場合は最初のファイル名）
    let file_info = if file_diffs.len() == 1 {
        format!(" Hunk Staging: {} ", file_diffs[0].file_path)
    } else {
        format!(" Hunk Staging ({} files) ", file_diffs.len())
    };

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(file_info));

    // ListState で選択位置にスクロールする
    let mut list_state = ListState::default().with_selected(Some(state.hunk_selected_index));
    f.render_stateful_widget(list, area, &mut list_state);
}

/// ステータスバーを描画する
fn render_status_bar(f: &mut Frame, state: &AppState, area: Rect) {
    let message = state
        .status_message
        .as_deref()
        .unwrap_or("j/k:move s:stage hunk Esc:back q:quit");

    let status_bar = Paragraph::new(message).style(Style::default().fg(Color::Cyan));

    f.render_widget(status_bar, area);
}
