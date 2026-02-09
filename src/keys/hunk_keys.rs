use anyhow::Result;
use crossterm::event::KeyCode;

use crate::app::AppState;

/// Hunk モードのキー処理
pub fn handle_hunk_mode_keys(state: &mut AppState, code: KeyCode) -> Result<()> {
    match code {
        // ナビゲーション（1行ずつ移動、ファイルヘッダはスキップ）
        KeyCode::Char('j') | KeyCode::Down => {
            state.hunk_select_next();
            state.clear_status_message();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.hunk_select_previous();
            state.clear_status_message();
        }
        // 選択した hunk / 行 / Visual 選択範囲をステージ
        KeyCode::Char('s') => {
            if let Err(e) = state.stage_selected_hunk() {
                state.set_status_message(&format!("Error: {}", e));
            }
        }
        // Visual モードのトグル
        KeyCode::Char('v') => {
            if state.hunk_visual_mode {
                state.hunk_visual_mode = false;
            } else {
                state.hunk_visual_mode = true;
                state.hunk_visual_anchor = state.hunk_selected_index;
            }
            state.clear_status_message();
        }
        // Esc: Visual モード中は Visual 解除のみ、通常時は hunk モード終了
        KeyCode::Esc => {
            if state.hunk_visual_mode {
                state.hunk_visual_mode = false;
                state.clear_status_message();
            } else {
                state.cancel_hunk_mode();
            }
        }
        // q: 常に hunk モード終了
        KeyCode::Char('q') => {
            state.cancel_hunk_mode();
        }
        _ => {}
    }
    Ok(())
}
