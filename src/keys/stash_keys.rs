use std::io;

use anyhow::Result;
use crossterm::event::{self, KeyCode};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::app::AppState;
use crate::pager::Pager;

use super::display_in_pager;

/// Stash View のキー処理
pub fn handle_stash_view_keys(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut AppState,
    code: KeyCode,
    pager: &Pager,
) -> Result<()> {
    match code {
        // ナビゲーション
        KeyCode::Char('j') | KeyCode::Down => {
            state.select_next();
            state.clear_status_message();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.select_previous();
            state.clear_status_message();
        }
        KeyCode::Char('g') => {
            state.select_first();
            state.clear_status_message();
        }
        KeyCode::Char('G') => {
            state.select_last();
            state.clear_status_message();
        }
        // Stash push（メッセージ入力ダイアログを表示）
        KeyCode::Char('s') => {
            state.start_stash_push();
        }
        // Stash pop
        KeyCode::Char('p') => {
            if let Err(e) = state.stash_pop() {
                state.set_status_message(&format!("Error: {}", e));
            }
        }
        // Stash apply
        KeyCode::Char('a') => {
            if let Err(e) = state.stash_apply() {
                state.set_status_message(&format!("Error: {}", e));
            }
        }
        // Stash drop（確認ダイアログを表示）
        KeyCode::Char('d') => {
            state.show_stash_drop_confirm();
        }
        // Stash show（内容表示）
        KeyCode::Enter => match state.get_stash_show() {
            Ok(content) => {
                if content.is_empty() {
                    state.set_status_message("No stash selected");
                } else {
                    display_in_pager(terminal, pager, &content, state)?;
                }
            }
            Err(e) => {
                state.set_status_message(&format!("Error: {}", e));
            }
        },
        // 手動リフレッシュ
        KeyCode::Char('R') => {
            if let Err(e) = state.refresh_stash() {
                state.set_status_message(&format!("Error: {}", e));
            } else {
                state.set_status_message("Refreshed");
            }
        }
        _ => {}
    }
    Ok(())
}

/// Stash 入力モードのキー処理
pub fn handle_stash_input_keys(state: &mut AppState, key: &event::KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Enter => {
            // メッセージを確定して stash push を実行
            if let Err(e) = state.stash_push() {
                state.set_status_message(&format!("Error: {}", e));
            }
        }
        KeyCode::Esc => {
            state.cancel_stash_input();
        }
        KeyCode::Backspace => {
            state.stash_message_pop();
        }
        KeyCode::Char(c) => {
            state.stash_message_push(c);
        }
        _ => {}
    }
    Ok(())
}
