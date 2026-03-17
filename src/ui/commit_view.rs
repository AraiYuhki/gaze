use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};
use unicode_width::UnicodeWidthChar;

use crate::app::{AppState, CommitMode};
use crate::domain::StatusKind;

/// コミット画面を描画する
pub fn render(f: &mut Frame, state: &AppState) {
    let area = f.area();

    // 背景をクリア
    f.render_widget(Clear, area);

    // レイアウト: 上部にタイトル、左にファイル一覧、右にメッセージ入力、下部にステータスバー
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // タイトル
            Constraint::Min(10),   // メイン領域
            Constraint::Length(3), // ステータスバー
        ])
        .split(area);

    render_title(f, state, main_chunks[0]);

    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // ファイル一覧
            Constraint::Percentage(60), // メッセージ入力
        ])
        .split(main_chunks[1]);

    render_staged_files(f, state, content_chunks[0]);
    render_message_input(f, state, content_chunks[1]);
    render_status_bar(f, state, main_chunks[2]);
}

/// タイトルを描画
fn render_title(f: &mut Frame, state: &AppState, area: Rect) {
    let title = match state.commit_mode {
        CommitMode::Normal => "Commit",
        CommitMode::Amend => "Amend Commit",
        CommitMode::None => "Commit",
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let help_text = "Tab: switch focus | Ctrl+D: commit | Esc: cancel";
    let paragraph = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .block(block);

    f.render_widget(paragraph, area);
}

/// ステージされたファイル一覧を描画
fn render_staged_files(f: &mut Frame, state: &AppState, area: Rect) {
    let staged_files = state.get_staged_files();

    let border_color = if state.commit_focus_files {
        Color::Yellow
    } else {
        Color::White
    };

    let block = Block::default()
        .title(format!("Staged Files ({})", staged_files.len()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let items: Vec<ListItem> = staged_files
        .iter()
        .enumerate()
        .map(|(index, file)| {
            let is_selected = index == state.commit_file_index;

            let (status_char, status_color) = match file.index {
                StatusKind::Modified => ('M', Color::Yellow),
                StatusKind::Added => ('A', Color::Green),
                StatusKind::Deleted => ('D', Color::Red),
                StatusKind::Renamed => ('R', Color::Blue),
                StatusKind::Copied => ('C', Color::Cyan),
                _ => (' ', Color::White),
            };

            let status_span = Span::styled(
                format!("[{}] ", status_char),
                Style::default().fg(status_color),
            );

            let path_style = if is_selected && state.commit_focus_files {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let path_span = Span::styled(file.path.display().to_string(), path_style);

            let line = Line::from(vec![status_span, path_span]);
            let item = ListItem::new(line);

            if is_selected && state.commit_focus_files {
                item.style(Style::default().bg(Color::Rgb(60, 60, 80)))
            } else {
                item
            }
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

/// メッセージ入力領域を描画
fn render_message_input(f: &mut Frame, state: &AppState, area: Rect) {
    let border_color = if !state.commit_focus_files {
        Color::Yellow
    } else {
        Color::White
    };

    let block = Block::default()
        .title("Commit Message")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    // メッセージを行ごとに表示
    // Wrap は使わない — 折り返しアルゴリズムの不一致によりカーソル位置がずれるため
    let lines: Vec<Line> = state
        .commit_message
        .iter()
        .map(|line_content| Line::from(line_content.as_str()))
        .collect();

    // カーソル行が見える位置までスクロール
    let inner_height = area.height.saturating_sub(2) as usize;
    let scroll_y = state
        .commit_cursor_y
        .saturating_sub(inner_height.saturating_sub(1));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .scroll((scroll_y as u16, 0));
    f.render_widget(paragraph, area);

    // メッセージ入力にフォーカスしている場合、ターミナルカーソルを設定
    // これによりIMEのプリエディット（変換中文字列）が正しい位置に表示される
    if !state.commit_focus_files {
        let visual_x = calculate_cursor_visual_x(
            &state.commit_message[state.commit_cursor_y],
            state.commit_cursor_x,
        );
        let cursor_x = area.x + 1 + visual_x;
        let cursor_y = area.y + 1 + (state.commit_cursor_y - scroll_y) as u16;
        f.set_cursor_position((cursor_x, cursor_y));
    }
}

/// ステータスバーを描画
fn render_status_bar(f: &mut Frame, state: &AppState, area: Rect) {
    let mode_str = match state.commit_mode {
        CommitMode::Normal => "COMMIT",
        CommitMode::Amend => "AMEND",
        CommitMode::None => "",
    };

    let focus_str = if state.commit_focus_files {
        "Files"
    } else {
        "Message"
    };

    let help = if state.commit_focus_files {
        "j/k: select | d: diff | Tab: switch to message"
    } else {
        "Type message | Enter: new line | Tab: switch to files"
    };

    let status_text = format!("[{}] Focus: {} | {}", mode_str, focus_str, help);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(status_text)
        .style(Style::default().fg(Color::White))
        .block(block);

    f.render_widget(paragraph, area);
}

/// カーソルのビジュアル X 座標を計算する（文字幅を考慮）
///
/// 各文字の表示幅（半角=1, 全角=2）を合算してカーソル位置を求める。
fn calculate_cursor_visual_x(line: &str, cursor_x: usize) -> u16 {
    line.chars()
        .take(cursor_x)
        .map(|c| UnicodeWidthChar::width(c).unwrap_or(1))
        .sum::<usize>() as u16
}
