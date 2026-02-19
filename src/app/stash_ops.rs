use crate::cli::parse_stash_list;
use crate::domain::StashEntry;
use crate::error::Result;

use super::{AppState, ConfirmDialog, StashInputMode};

impl AppState {
    /// Stash 一覧を再取得してキャッシュを更新する
    pub fn refresh_stash(&mut self) -> Result<()> {
        let output = self.git.execute(&["stash", "list"])?;
        self.stash_cache = parse_stash_list(&output);
        // 選択インデックスが範囲外になった場合は調整
        if !self.stash_cache.is_empty() && self.stash_selected_index >= self.stash_cache.len() {
            self.stash_selected_index = self.stash_cache.len() - 1;
        }
        Ok(())
    }

    /// 現在選択されている Stash エントリを取得
    pub fn selected_stash(&self) -> Option<&StashEntry> {
        self.stash_cache.get(self.stash_selected_index)
    }

    /// Stash メッセージ入力モードを開始
    pub fn start_stash_push(&mut self) {
        self.stash_input_mode = StashInputMode::Push;
        self.stash_message.clear();
    }

    /// Stash メッセージ入力をキャンセル
    pub fn cancel_stash_input(&mut self) {
        self.stash_input_mode = StashInputMode::None;
        self.stash_message.clear();
    }

    /// Stash メッセージに文字を追加
    pub fn stash_message_push(&mut self, c: char) {
        self.stash_message.push(c);
    }

    /// Stash メッセージから文字を削除
    pub fn stash_message_pop(&mut self) {
        self.stash_message.pop();
    }

    /// 現在の変更を stash に保存
    ///
    /// # Errors
    /// - Git コマンドの実行に失敗した場合
    pub fn stash_push(&mut self) -> Result<()> {
        if self.stash_message.is_empty() {
            self.git.execute(&["stash", "push"])?;
        } else {
            self.git
                .execute(&["stash", "push", "-m", &self.stash_message])?;
        }
        self.stash_input_mode = StashInputMode::None;
        self.stash_message.clear();
        self.refresh_stash()?;
        self.refresh_status()?;
        self.set_status_message("Changes stashed");
        Ok(())
    }

    /// 選択した stash を適用して削除（pop）
    ///
    /// # Errors
    /// - Git コマンドの実行に失敗した場合
    pub fn stash_pop(&mut self) -> Result<()> {
        if let Some(entry) = self.stash_cache.get(self.stash_selected_index) {
            let stash_ref = format!("stash@{{{}}}", entry.index);
            self.git.execute(&["stash", "pop", &stash_ref])?;
            self.refresh_stash()?;
            self.refresh_status()?;
            self.set_status_message("Stash popped");
        }
        Ok(())
    }

    /// 選択した stash を適用（削除せず）
    ///
    /// # Errors
    /// - Git コマンドの実行に失敗した場合
    pub fn stash_apply(&mut self) -> Result<()> {
        if let Some(entry) = self.stash_cache.get(self.stash_selected_index) {
            let stash_ref = format!("stash@{{{}}}", entry.index);
            self.git.execute(&["stash", "apply", &stash_ref])?;
            self.refresh_status()?;
            self.set_status_message("Stash applied");
        }
        Ok(())
    }

    /// Stash 削除の確認ダイアログを表示
    pub fn show_stash_drop_confirm(&mut self) {
        if self.selected_stash().is_some() {
            self.confirm_selected_yes = false;
            self.confirm_dialog = ConfirmDialog::DropStash {
                stash_index: self.stash_selected_index,
            };
        }
    }

    /// 選択した stash を削除
    ///
    /// # Errors
    /// - Git コマンドの実行に失敗した場合
    pub fn stash_drop(&mut self) -> Result<()> {
        if let ConfirmDialog::DropStash { stash_index } = self.confirm_dialog {
            if let Some(entry) = self.stash_cache.get(stash_index) {
                let stash_ref = format!("stash@{{{}}}", entry.index);
                self.git.execute(&["stash", "drop", &stash_ref])?;
                self.confirm_dialog = ConfirmDialog::None;
                self.refresh_stash()?;
                self.set_status_message("Stash dropped");
            }
        }
        Ok(())
    }

    /// 選択した stash の内容を取得（色付き）
    ///
    /// # Errors
    /// - Git コマンドの実行に失敗した場合
    pub fn get_stash_show(&self) -> Result<String> {
        if let Some(entry) = self.stash_cache.get(self.stash_selected_index) {
            let stash_ref = format!("stash@{{{}}}", entry.index);
            self.git
                .execute(&["stash", "show", "-p", "--color=always", &stash_ref])
        } else {
            Ok(String::new())
        }
    }
}
