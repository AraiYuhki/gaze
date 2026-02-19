pub mod branch_view;
pub mod commit_view;
pub mod help_view;
pub mod hunk_view;
pub mod log_view;
pub mod stash_view;
pub mod status_view;
pub mod tree_view;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
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

/// 中央揃えの矩形を計算する（パーセント指定）
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
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
