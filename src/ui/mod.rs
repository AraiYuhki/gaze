pub mod branch_view;
pub mod commit_view;
pub mod help_view;
pub mod hunk_view;
pub mod log_view;
pub mod stash_view;
pub mod status_view;
pub mod tree_view;

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// 確認ダイアログの共通描画
///
/// Yes/No ボタンを表示し、選択状態をハイライトする。
pub fn render_confirm_dialog(
    f: &mut Frame,
    title: &str,
    detail: &str,
    selected_yes: bool,
    area: Rect,
) {
    let yes_style = if selected_yes {
        Style::default()
            .bg(Color::Green)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let no_style = if !selected_yes {
        Style::default()
            .bg(Color::Red)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            title,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    if !detail.is_empty() {
        lines.push(Line::from(detail));
        lines.push(Line::from(""));
    }
    lines.push(Line::from(vec![
        Span::raw("       "),
        Span::styled(" Yes ", yes_style),
        Span::raw("     "),
        Span::styled(" No ", no_style),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  ←/→: select  Enter/y: confirm  Esc/n: cancel",
        Style::default().fg(Color::DarkGray),
    )));

    let dialog = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Confirm ")
                .style(Style::default().bg(Color::Black)),
        )
        .style(Style::default().bg(Color::Black));

    f.render_widget(ratatui::widgets::Clear, area);
    f.render_widget(dialog, area);
}

/// 固定サイズの中央揃え矩形を計算する
///
/// ターミナルサイズが指定サイズより小さい場合はターミナルサイズに収まる。
pub fn fixed_centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w, h)
}
