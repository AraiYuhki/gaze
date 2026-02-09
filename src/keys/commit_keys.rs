use std::io;

use anyhow::Result;
use crossterm::event::{self, KeyCode, KeyModifiers};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::app::AppState;
use crate::pager::Pager;

use super::display_in_pager;

/// コミットモードのキー処理
pub fn handle_commit_mode_keys(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut AppState,
    key: &event::KeyEvent,
    pager: &Pager,
) -> Result<()> {
    // Ctrl+Enter または Ctrl+D でコミット実行
    // 注: 多くのターミナルでは Ctrl+Enter が正しく認識されないため Ctrl+D を推奨
    let is_commit_key = (key.code == KeyCode::Enter
        && key.modifiers.contains(KeyModifiers::CONTROL))
        || (key.code == KeyCode::Char('d') && key.modifiers.contains(KeyModifiers::CONTROL));

    if is_commit_key {
        if let Err(e) = state.execute_commit() {
            state.set_status_message(&format!("Error: {}", e));
        }
        return Ok(());
    }

    // Esc でキャンセル
    if key.code == KeyCode::Esc {
        state.cancel_commit_mode();
        state.set_status_message("Commit cancelled");
        return Ok(());
    }

    // Tab でフォーカス切り替え
    if key.code == KeyCode::Tab {
        state.commit_toggle_focus();
        return Ok(());
    }

    // ファイル一覧にフォーカスしている場合
    if state.commit_focus_files {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                state.commit_file_next();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                state.commit_file_prev();
            }
            KeyCode::Char('d') => {
                // staged diff を表示
                match state.get_commit_staged_diff() {
                    Ok(diff) => {
                        if diff.is_empty() {
                            state.set_status_message("No diff available");
                        } else {
                            display_in_pager(terminal, pager, &diff, state)?;
                        }
                    }
                    Err(e) => {
                        state.set_status_message(&format!("Error: {}", e));
                    }
                }
            }
            _ => {}
        }
    } else {
        // メッセージ入力にフォーカスしている場合
        match key.code {
            KeyCode::Enter => {
                state.commit_new_line();
            }
            KeyCode::Backspace => {
                state.commit_delete_char();
            }
            KeyCode::Left => {
                state.commit_cursor_left();
            }
            KeyCode::Right => {
                state.commit_cursor_right();
            }
            KeyCode::Up => {
                state.commit_cursor_up();
            }
            KeyCode::Down => {
                state.commit_cursor_down();
            }
            KeyCode::Char(c) => {
                state.commit_insert_char(c);
            }
            _ => {}
        }
    }

    Ok(())
}
