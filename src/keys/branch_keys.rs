use anyhow::Result;
use crossterm::event::{self, KeyCode};

use crate::app::AppState;

/// Branch View のキー処理
pub fn handle_branch_view_keys(state: &mut AppState, code: KeyCode) -> Result<()> {
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
        // ブランチ切り替え
        KeyCode::Enter => {
            state.show_branch_checkout_confirm();
        }
        // 検索モード開始
        KeyCode::Char('/') => {
            state.start_branch_search();
        }
        // 検索クリア
        KeyCode::Esc => {
            if !state.branch_search_query.is_empty() {
                state.clear_branch_search();
            }
        }
        // 手動リフレッシュ
        KeyCode::Char('R') => {
            if let Err(e) = state.refresh_branches() {
                state.set_status_message(&format!("Error: {}", e));
            } else {
                state.set_status_message("Refreshed");
            }
        }
        _ => {}
    }
    Ok(())
}

/// Branch 入力モードのキー処理
pub fn handle_branch_input_keys(state: &mut AppState, key: &event::KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Enter => {
            // 検索を確定
            state.confirm_branch_search();
        }
        KeyCode::Esc => {
            state.cancel_branch_search();
        }
        KeyCode::Backspace => {
            state.branch_search_pop();
        }
        KeyCode::Char(c) => {
            state.branch_search_push(c);
        }
        _ => {}
    }
    Ok(())
}
