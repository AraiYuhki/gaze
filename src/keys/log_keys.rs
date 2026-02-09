use std::io;

use anyhow::Result;
use crossterm::event::KeyCode;
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::app::{AppState, View};
use crate::pager::Pager;

use super::display_in_pager;

/// Log View のキー処理
pub fn handle_log_view_keys(
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
        // コミット詳細表示
        KeyCode::Enter => match state.get_commit_details() {
            Ok(details) => {
                if details.is_empty() {
                    state.set_status_message("No commit selected");
                } else {
                    display_in_pager(terminal, pager, &details, state)?;
                }
            }
            Err(e) => {
                state.set_status_message(&format!("Error: {}", e));
            }
        },
        // チェックアウト（ファイルログモードでは無効）
        KeyCode::Char('c') => {
            if !state.is_file_log_mode() {
                state.show_checkout_confirm();
            }
        }
        // 手動リフレッシュ
        KeyCode::Char('R') => {
            let result = if state.is_file_log_mode() {
                // ファイルログモードの場合は現在のパスでリフレッシュ
                if let Some(path) = state.file_log_path.clone() {
                    state.refresh_file_log(&path)
                } else {
                    Ok(())
                }
            } else {
                state.refresh_log()
            };
            if let Err(e) = result {
                state.set_status_message(&format!("Error: {}", e));
            } else {
                state.set_status_message("Refreshed");
            }
        }
        // ファイルログモードを終了して Tree View に戻る
        KeyCode::Esc => {
            if state.is_file_log_mode() {
                state.clear_file_log();
                state.switch_view(View::Tree);
            }
        }
        _ => {}
    }
    Ok(())
}
