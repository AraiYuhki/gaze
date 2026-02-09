use crossterm::event::KeyCode;

use crate::app::AppState;

/// Tree View のキー処理
pub fn handle_tree_view_keys(state: &mut AppState, code: KeyCode) {
    // 検索モード中の処理
    if state.tree_search_mode {
        match code {
            KeyCode::Esc => {
                state.cancel_tree_search();
            }
            KeyCode::Enter => {
                state.confirm_tree_search();
                let match_count = state.tree_search_matches.len();
                if match_count > 0 {
                    state.set_status_message(&format!("Found {} match(es)", match_count));
                } else {
                    state.set_status_message("No matches found");
                }
            }
            KeyCode::Backspace => {
                state.remove_tree_search_char();
            }
            KeyCode::Char(c) => {
                state.add_tree_search_char(c);
            }
            _ => {}
        }
        return;
    }

    // 通常モードの処理
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
        // 展開（Enter）
        KeyCode::Enter => {
            state.expand_tree_node();
        }
        // 展開 or ファイルログ（l）
        KeyCode::Char('l') => {
            if state.is_selected_tree_node_file() {
                // ファイルの場合はファイルログを表示
                if let Err(e) = state.open_file_log() {
                    state.set_status_message(&format!("Error: {}", e));
                }
            } else {
                // ディレクトリの場合は展開
                state.expand_tree_node();
            }
        }
        KeyCode::Char('h') => {
            state.collapse_tree_node();
        }
        // フィルタ切替
        KeyCode::Char('H') => {
            state.toggle_display_filter();
            let status = if state.display_filter.is_enabled() {
                "Filter ON"
            } else {
                "Filter OFF"
            };
            state.set_status_message(status);
        }
        // 検索
        KeyCode::Char('/') => {
            state.start_tree_search();
        }
        // 次のマッチへ
        KeyCode::Char('n') => {
            if !state.tree_search_matches.is_empty() {
                state.next_tree_search_match();
                state.set_status_message(&format!(
                    "Match {}/{}",
                    state.tree_search_current_match + 1,
                    state.tree_search_matches.len()
                ));
            }
        }
        // 前のマッチへ
        KeyCode::Char('N') => {
            if !state.tree_search_matches.is_empty() {
                state.prev_tree_search_match();
                state.set_status_message(&format!(
                    "Match {}/{}",
                    state.tree_search_current_match + 1,
                    state.tree_search_matches.len()
                ));
            }
        }
        // 手動リフレッシュ
        KeyCode::Char('R') => {
            if let Err(e) = state.refresh_status() {
                state.set_status_message(&format!("Error: {}", e));
            } else {
                // Tree View ではリフレッシュ後にステータスを再適用
                state.refresh_tree_status();
                state.set_status_message("Refreshed");
            }
        }
        _ => {}
    }
}
