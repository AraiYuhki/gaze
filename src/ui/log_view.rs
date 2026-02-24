use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{AppState, ConfirmDialog};
use crate::domain::GraphLine;

/// Log View を描画する
pub fn render(f: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // メインコンテンツ
            Constraint::Length(1), // ステータスバー
        ])
        .split(f.area());

    render_log(f, state, chunks[0]);
    render_status_bar(f, state, chunks[1]);

    // コミットチェックアウト確認ダイアログ
    if let ConfirmDialog::Checkout { commit_hash } = &state.confirm_dialog {
        let area = super::centered_rect(50, 25, f.area());
        super::render_confirm_dialog(
            f,
            "Checkout commit?",
            commit_hash,
            state.confirm_selected_yes,
            area,
        );
    }
}

/// ログ一覧を描画する
fn render_log(f: &mut Frame, state: &AppState, area: Rect) {
    // ファイルログモードかどうかで使用するキャッシュとインデックスを切り替え
    let (log_cache, selected_index, title) = if let Some(ref path) = state.file_log_path {
        let filename = path
            .file_name()
            .map_or("unknown".to_string(), |n| n.to_string_lossy().to_string());
        (
            &state.file_log_cache,
            state.file_log_selected_index,
            format!(
                " File Log: {} [{} commits] ",
                filename,
                state.file_log_cache.len()
            ),
        )
    } else {
        (
            &state.log_cache,
            state.log_selected_index,
            format!(" Log (3) [{} commits] ", state.log_cache.len()),
        )
    };

    // スクロール位置を計算
    let visible_height = area.height.saturating_sub(2) as usize; // ボーダー分を引く
    let offset = calculate_scroll_offset(selected_index, visible_height);

    // ListState を使わずに手動でオフセットを適用
    let items_to_show: Vec<ListItem> = log_cache
        .iter()
        .enumerate()
        .skip(offset)
        .take(visible_height)
        .map(|(index, line)| {
            let is_selected = index == selected_index;
            render_log_item(line, is_selected)
        })
        .collect();

    let list = List::new(items_to_show).block(Block::default().borders(Borders::ALL).title(title));

    f.render_widget(list, area);
}

/// スクロールオフセットを計算する
fn calculate_scroll_offset(selected: usize, visible_height: usize) -> usize {
    if visible_height == 0 {
        return 0;
    }
    // 選択位置が画面の中央付近に来るようにする
    selected.saturating_sub(visible_height / 2)
}

/// ログアイテムを描画する
fn render_log_item(line: &GraphLine, is_selected: bool) -> ListItem<'static> {
    let mut spans = Vec::new();

    // グラフ部分（色分け）
    if !line.graph_chars.is_empty() {
        for c in line.graph_chars.chars() {
            let color = match c {
                '*' => Color::Yellow,
                '|' => Color::Blue,
                '/' | '\\' => Color::Green,
                '-' | '_' => Color::Cyan,
                _ => Color::DarkGray,
            };
            spans.push(Span::styled(c.to_string(), Style::default().fg(color)));
        }
    }

    // ハッシュ
    if let Some(ref hash) = line.hash {
        spans.push(Span::styled(
            hash.clone(),
            Style::default().fg(Color::Yellow),
        ));
        spans.push(Span::raw(" "));
    }

    // refs（ブランチ名、タグ）
    if !line.refs.is_empty() {
        spans.push(Span::styled("(", Style::default().fg(Color::Yellow)));
        for (i, ref_name) in line.refs.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(", ", Style::default().fg(Color::Yellow)));
            }
            // HEAD, origin などを色分け
            let color = if ref_name.contains("HEAD") {
                Color::Cyan
            } else if ref_name.contains("origin") || ref_name.contains("->") {
                Color::Red
            } else {
                // ブランチ名やタグは緑
                Color::Green
            };
            spans.push(Span::styled(
                ref_name.clone(),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ));
        }
        spans.push(Span::styled(") ", Style::default().fg(Color::Yellow)));
    }

    // メッセージ
    if let Some(ref message) = line.message {
        spans.push(Span::styled(
            message.clone(),
            Style::default().fg(Color::White),
        ));
    }

    // spans が空の場合は raw_line を表示
    if spans.is_empty() {
        spans.push(Span::raw(line.raw_line.clone()));
    }

    let item = ListItem::new(Line::from(spans));

    if is_selected {
        item.style(Style::default().bg(Color::Rgb(60, 60, 80)))
    } else {
        item
    }
}

/// ステータスバーを描画する
fn render_status_bar(f: &mut Frame, state: &AppState, area: Rect) {
    let default_message = if state.is_file_log_mode() {
        "j/k:move Enter:show Esc:back q:quit"
    } else {
        "j/k:move Enter:show c:checkout 1:status 2:tree q:quit"
    };

    let message = state.status_message.as_deref().unwrap_or(default_message);

    let status_bar = Paragraph::new(message).style(Style::default().fg(Color::Cyan));

    f.render_widget(status_bar, area);
}
