use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// ヘルプ画面を描画する
pub fn render(f: &mut Frame) {
    let area = f.area();

    let help_text = vec![
        Line::from(vec![Span::styled(
            "Gaze - Git TUI Help",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "=== Global ===",
            Style::default().fg(Color::Yellow),
        )]),
        Line::from("  1          Status View"),
        Line::from("  2          Tree View"),
        Line::from("  3          Log View"),
        Line::from("  4          Stash View"),
        Line::from("  5          Branch View"),
        Line::from("  Tab        Next View"),
        Line::from("  ?          Help"),
        Line::from("  q          Quit"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "=== Status View ===",
            Style::default().fg(Color::Yellow),
        )]),
        Line::from("  j/k, ↓/↑   Navigate"),
        Line::from("  g/G        First/Last"),
        Line::from("  s          Stage/Unstage"),
        Line::from("  d          Show diff"),
        Line::from("  r          Discard changes"),
        Line::from("  c          Commit"),
        Line::from("  C          Amend commit"),
        Line::from("  R          Refresh"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "=== Tree View ===",
            Style::default().fg(Color::Yellow),
        )]),
        Line::from("  j/k, ↓/↑   Navigate"),
        Line::from("  Enter      Expand directory"),
        Line::from("  l          Expand / File log"),
        Line::from("  h          Collapse"),
        Line::from("  H          Toggle filter"),
        Line::from("  /          Search"),
        Line::from("  n/N        Next/Prev match"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "=== Log View ===",
            Style::default().fg(Color::Yellow),
        )]),
        Line::from("  j/k, ↓/↑   Navigate"),
        Line::from("  Enter      Show commit"),
        Line::from("  c          Checkout"),
        Line::from("  Esc        Exit file log"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "=== Stash View ===",
            Style::default().fg(Color::Yellow),
        )]),
        Line::from("  j/k, ↓/↑   Navigate"),
        Line::from("  s          Stash push"),
        Line::from("  p          Stash pop"),
        Line::from("  a          Stash apply"),
        Line::from("  d          Stash drop"),
        Line::from("  Enter      Show stash"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "=== Branch View ===",
            Style::default().fg(Color::Yellow),
        )]),
        Line::from("  j/k, ↓/↑   Navigate"),
        Line::from("  g/G        First/Last"),
        Line::from("  Enter      Checkout branch"),
        Line::from("  /          Search filter"),
        Line::from("  Esc        Clear filter"),
        Line::from("  R          Refresh"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Press any key to close",
            Style::default().fg(Color::DarkGray),
        )]),
    ];

    // ヘルプテキストの行数に基づいて高さを計算（ボーダー2行分を追加）
    let content_height = help_text.len() as u16 + 2;
    let popup_width = 60.min(area.width.saturating_sub(4));
    let popup_height = content_height.min(area.height.saturating_sub(2));

    let popup_area = centered_rect(popup_width, popup_height, area);

    // 背景をクリア
    f.render_widget(Clear, popup_area);

    let paragraph = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Help "),
        )
        .alignment(Alignment::Left);

    f.render_widget(paragraph, popup_area);
}

/// 中央に配置された矩形を計算する
fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((r.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((r.width.saturating_sub(width)) / 2),
            Constraint::Length(width),
            Constraint::Min(0),
        ])
        .split(vertical[1])[1]
}
