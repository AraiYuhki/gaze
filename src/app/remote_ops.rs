use crate::domain::StatusKind;
use crate::error::Result;

use super::{AppState, ConfirmDialog};

impl AppState {
    /// Push の確認ダイアログを表示する
    pub fn show_push_confirm(&mut self) {
        self.confirm_dialog = ConfirmDialog::Push;
    }

    /// Pull の確認ダイアログを表示する
    pub fn show_pull_confirm(&mut self) {
        // 未コミットの変更があるかチェック
        let has_changes = self
            .status_cache
            .iter()
            .any(|f| f.worktree != StatusKind::Unmodified || f.index != StatusKind::Unmodified);
        if has_changes {
            self.set_status_message(
                "Cannot pull: uncommitted changes exist. Stash or commit first.",
            );
            return;
        }
        self.confirm_dialog = ConfirmDialog::Pull;
    }

    /// Push を実行する
    ///
    /// # Errors
    /// - Git コマンドの実行に失敗した場合
    pub fn execute_push(&mut self) -> Result<()> {
        if !matches!(self.confirm_dialog, ConfirmDialog::Push) {
            return Ok(());
        }
        self.confirm_dialog = ConfirmDialog::None;

        match self.git.execute(&["push"]) {
            Ok(_) => {
                self.set_status_message("Pushed successfully");
            }
            Err(e) => {
                self.set_status_message(&format!("Push failed: {}", e));
            }
        }
        Ok(())
    }

    /// Pull を実行する
    ///
    /// # Errors
    /// - Git コマンドの実行に失敗した場合
    pub fn execute_pull(&mut self) -> Result<()> {
        if !matches!(self.confirm_dialog, ConfirmDialog::Pull) {
            return Ok(());
        }
        self.confirm_dialog = ConfirmDialog::None;

        match self.git.execute(&["pull"]) {
            Ok(_) => {
                self.set_status_message("Pulled successfully");
                // Pull 後にステータスとログを更新
                self.refresh_status()?;
            }
            Err(e) => {
                self.set_status_message(&format!("Pull failed: {}", e));
            }
        }
        Ok(())
    }
}
