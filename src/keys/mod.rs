mod branch_keys;
mod commit_keys;
mod hunk_keys;
mod log_keys;
mod stash_keys;
mod status_keys;
mod tree_keys;

pub use branch_keys::{handle_branch_input_keys, handle_branch_view_keys};
pub use commit_keys::handle_commit_mode_keys;
pub use hunk_keys::handle_hunk_mode_keys;
pub use log_keys::handle_log_view_keys;
pub use stash_keys::{handle_stash_input_keys, handle_stash_view_keys};
pub use status_keys::handle_status_view_keys;
pub use tree_keys::handle_tree_view_keys;

use std::io;

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::app::AppState;
use crate::pager::Pager;

/// ページャでコンテンツを表示する共通ヘルパー
///
/// ターミナルの raw モードを一時的に解除し、ページャを起動して、
/// 終了後にターミナルを元の状態に復帰させる。
fn display_in_pager(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    pager: &Pager,
    content: &str,
    state: &mut AppState,
) -> Result<()> {
    if content.is_empty() {
        return Ok(());
    }
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    if let Err(e) = pager.display(content) {
        enable_raw_mode()?;
        execute!(terminal.backend_mut(), EnterAlternateScreen)?;
        state.set_status_message(&format!("Pager error: {}", e));
    } else {
        enable_raw_mode()?;
        execute!(terminal.backend_mut(), EnterAlternateScreen)?;
        terminal.clear()?;
    }
    Ok(())
}
