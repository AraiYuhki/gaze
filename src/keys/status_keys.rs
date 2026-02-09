use std::io;

use anyhow::Result;
use crossterm::event::KeyCode;
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::app::{AppState, ConfirmDialog};
use crate::pager::Pager;

use super::display_in_pager;

/// Status View のキー処理
pub fn handle_status_view_keys(
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
        // ステージング
        KeyCode::Char('s') => {
            if let Err(e) = state.stage() {
                state.set_status_message(&format!("Error: {}", e));
            } else {
                state.set_status_message("Staged");
            }
        }
        // アンステージング
        KeyCode::Char('u') => {
            if let Err(e) = state.unstage() {
                state.set_status_message(&format!("Error: {}", e));
            } else {
                state.set_status_message("Unstaged");
            }
        }
        // 差分表示
        KeyCode::Char('d') => match state.get_diff() {
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
        },
        // 変更破棄
        KeyCode::Char('r') => {
            state.show_discard_confirm();
        }
        // 手動リフレッシュ
        KeyCode::Char('R') => {
            if let Err(e) = state.refresh_status() {
                state.set_status_message(&format!("Error: {}", e));
            } else {
                state.set_status_message("Refreshed");
            }
        }
        // コミット
        KeyCode::Char('c') => {
            state.start_commit_mode();
        }
        // Amend（確認ダイアログを表示）
        KeyCode::Char('C') => {
            state.confirm_dialog = ConfirmDialog::Amend;
            state.set_status_message("Amend last commit? (y/n)");
        }
        // Push（確認ダイアログを表示）
        KeyCode::Char('P') => {
            state.show_push_confirm();
            state.set_status_message("Push to remote? (y/n)");
        }
        // Pull（確認ダイアログを表示）
        KeyCode::Char('U') => {
            state.show_pull_confirm();
            if matches!(state.confirm_dialog, ConfirmDialog::Pull) {
                state.set_status_message("Pull from remote? (y/n)");
            }
        }
        // Hunk ステージングモード
        KeyCode::Char('H') => {
            if let Err(e) = state.start_hunk_mode() {
                state.set_status_message(&format!("Error: {}", e));
            }
        }
        _ => {}
    }
    Ok(())
}
