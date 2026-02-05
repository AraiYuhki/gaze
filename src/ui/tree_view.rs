use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::AppState;
use crate::domain::{NodeKind, StatusKind, TreeNode};
use crate::filter::DisplayFilter;

/// Tree View を描画する
pub fn render(f: &mut Frame, state: &AppState) {
    // 検索モード中は検索バーを表示
    let constraints = if state.tree_search_mode {
        vec![
            Constraint::Min(3),    // メインコンテンツ
            Constraint::Length(1), // 検索バー
            Constraint::Length(1), // ステータスバー
        ]
    } else {
        vec![
            Constraint::Min(3),    // メインコンテンツ
            Constraint::Length(1), // ステータスバー
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(f.area());

    render_tree(f, state, chunks[0]);

    if state.tree_search_mode {
        render_search_bar(f, state, chunks[1]);
        render_status_bar(f, state, chunks[2]);
    } else {
        render_status_bar(f, state, chunks[1]);
    }
}

/// ツリーを描画する
fn render_tree(f: &mut Frame, state: &AppState, area: Rect) {
    let flat_nodes = flatten_tree(&state.tree_root, &state.display_filter, 0);

    let items: Vec<ListItem> = flat_nodes
        .iter()
        .enumerate()
        .map(|(index, (node, depth))| {
            let is_selected = index == state.tree_selected_index;
            let is_match = state.tree_search_matches.contains(&node.path);
            render_tree_item(
                node,
                *depth,
                is_selected,
                is_match,
                &state.tree_search_query,
            )
        })
        .collect();

    let filter_status = if state.display_filter.is_enabled() {
        " [Filter ON]"
    } else {
        ""
    };

    let search_status = if !state.tree_search_matches.is_empty() {
        format!(
            " [{}/{}]",
            state.tree_search_current_match + 1,
            state.tree_search_matches.len()
        )
    } else {
        String::new()
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Tree (2){}{} ", filter_status, search_status)),
    );

    f.render_widget(list, area);
}

/// ツリーアイテムを描画する
fn render_tree_item(
    node: &TreeNode,
    depth: usize,
    is_selected: bool,
    is_match: bool,
    search_query: &str,
) -> ListItem<'static> {
    // インデント
    let indent = "  ".repeat(depth);

    // 展開アイコン
    let icon = match &node.kind {
        NodeKind::Directory => {
            if node.expanded {
                "▼ "
            } else {
                "▶ "
            }
        }
        NodeKind::File => "  ",
    };

    // Git ステータスインジケータ
    let (status_indicator, status_color) = match node.git_status {
        Some(StatusKind::Modified) => ("[M]", Color::Yellow),
        Some(StatusKind::Added) => ("[A]", Color::Green),
        Some(StatusKind::Deleted) => ("[D]", Color::Red),
        Some(StatusKind::Renamed) => ("[R]", Color::Magenta),
        Some(StatusKind::Untracked) => ("[?]", Color::Gray),
        _ => ("", Color::White),
    };

    // 名前のスタイル
    let base_color = if node.kind == NodeKind::Directory {
        Color::Blue
    } else {
        Color::White
    };

    let name_style = if is_selected {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else if is_match {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(base_color)
    };

    // 検索クエリがある場合はマッチ部分をハイライト
    let name_spans = if is_match && !search_query.is_empty() && !is_selected {
        highlight_match(&node.name, search_query, name_style, base_color)
    } else {
        vec![Span::styled(node.name.clone(), name_style)]
    };

    let mut spans = vec![
        Span::raw(indent),
        Span::styled(icon, Style::default().fg(Color::Cyan)),
    ];
    spans.extend(name_spans);

    if !status_indicator.is_empty() {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            status_indicator.to_string(),
            Style::default().fg(status_color),
        ));
    }

    let line = Line::from(spans);
    let item = ListItem::new(line);

    if is_selected {
        item.style(Style::default().bg(Color::DarkGray))
    } else {
        item
    }
}

/// マッチ部分をハイライトした Span のリストを返す
fn highlight_match(
    text: &str,
    query: &str,
    match_style: Style,
    normal_color: Color,
) -> Vec<Span<'static>> {
    let text_lower = text.to_lowercase();
    let query_lower = query.to_lowercase();

    let mut spans = Vec::new();
    let mut last_end = 0;

    // 大文字小文字を無視してマッチ位置を検索
    for (start, _) in text_lower.match_indices(&query_lower) {
        // マッチ前のテキスト
        if start > last_end {
            spans.push(Span::styled(
                text[last_end..start].to_string(),
                Style::default().fg(normal_color),
            ));
        }
        // マッチ部分
        let end = start + query.len();
        spans.push(Span::styled(text[start..end].to_string(), match_style));
        last_end = end;
    }

    // マッチ後のテキスト
    if last_end < text.len() {
        spans.push(Span::styled(
            text[last_end..].to_string(),
            Style::default().fg(normal_color),
        ));
    }

    if spans.is_empty() {
        vec![Span::styled(
            text.to_string(),
            Style::default().fg(normal_color),
        )]
    } else {
        spans
    }
}

/// ステータスバーを描画する
fn render_status_bar(f: &mut Frame, state: &AppState, area: Rect) {
    let message = state
        .status_message
        .as_deref()
        .unwrap_or("j/k:move Enter/l:expand h:collapse /:search n/N:next/prev q:quit");

    let status_bar = Paragraph::new(message).style(Style::default().fg(Color::Cyan));

    f.render_widget(status_bar, area);
}

/// 検索バーを描画する
fn render_search_bar(f: &mut Frame, state: &AppState, area: Rect) {
    let match_info = if state.tree_search_matches.is_empty() {
        if state.tree_search_query.is_empty() {
            String::new()
        } else {
            " (no match)".to_string()
        }
    } else {
        format!(
            " ({}/{})",
            state.tree_search_current_match + 1,
            state.tree_search_matches.len()
        )
    };

    let search_text = format!("/{}{}", state.tree_search_query, match_info);

    let search_bar = Paragraph::new(search_text).style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    f.render_widget(search_bar, area);
}

/// ツリーをフラット化する（表示用）
///
/// 展開されたディレクトリの子を含め、表示順に並べたリストを返す
pub fn flatten_tree<'a>(
    root: &'a TreeNode,
    filter: &DisplayFilter,
    depth: usize,
) -> Vec<(&'a TreeNode, usize)> {
    let mut result = Vec::new();

    if let Some(children) = &root.children {
        for child in children {
            // フィルタで非表示のものはスキップ
            if filter.should_hide(&child.path) {
                continue;
            }

            result.push((child, depth));

            // 展開されたディレクトリの子を再帰的に追加
            if child.expanded && child.kind == NodeKind::Directory {
                result.extend(flatten_tree(child, filter, depth + 1));
            }
        }
    }

    result
}

/// フラット化されたツリーから指定インデックスのノードを取得
#[allow(dead_code)] // TODO(Phase 2): 将来の拡張用（ファイル操作など）
pub fn get_node_at_index<'a>(
    root: &'a TreeNode,
    filter: &DisplayFilter,
    index: usize,
) -> Option<&'a TreeNode> {
    let flat = flatten_tree(root, filter, 0);
    flat.get(index).map(|(node, _)| *node)
}

/// フラット化されたツリーの長さを取得
pub fn get_flat_tree_len(root: &TreeNode, filter: &DisplayFilter) -> usize {
    flatten_tree(root, filter, 0).len()
}
