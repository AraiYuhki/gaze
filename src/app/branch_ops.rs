use crate::cli::parse_branch_list;
use crate::domain::{BranchEntry, StatusKind};
use crate::error::Result;

use super::{AppState, BranchInputMode, ConfirmDialog};

impl AppState {
    /// Branch 一覧を再取得してキャッシュを更新する
    pub fn refresh_branches(&mut self) -> Result<()> {
        let output = self.git.execute(&["branch", "-a"])?;
        self.branch_cache = parse_branch_list(&output);
        self.rebuild_filtered_branches();
        // 選択インデックスが範囲外になった場合は調整
        let filtered_len = self.filtered_branch_indices.len();
        if filtered_len > 0 && self.branch_selected_index >= filtered_len {
            self.branch_selected_index = filtered_len - 1;
        }
        Ok(())
    }

    /// フィルタリングされた Branch 一覧を取得（キャッシュ使用）
    pub fn filtered_branches(&self) -> Vec<&BranchEntry> {
        self.filtered_branch_indices
            .iter()
            .filter_map(|&i| self.branch_cache.get(i))
            .collect()
    }

    /// フィルタ済み Branch インデックスキャッシュを再構築する
    ///
    /// branch_cache または branch_search_query が変わった後に呼び出す
    fn rebuild_filtered_branches(&mut self) {
        if self.branch_search_query.is_empty() {
            self.filtered_branch_indices = (0..self.branch_cache.len()).collect();
        } else {
            let query_lower = self.branch_search_query.to_lowercase();
            self.filtered_branch_indices = self
                .branch_cache
                .iter()
                .enumerate()
                .filter(|(_, b)| b.name.to_lowercase().contains(&query_lower))
                .map(|(i, _)| i)
                .collect();
        }
    }

    /// 現在選択されている Branch エントリを取得
    pub fn selected_branch(&self) -> Option<&BranchEntry> {
        self.filtered_branches()
            .get(self.branch_selected_index)
            .copied()
    }

    /// Branch 検索モードを開始
    pub fn start_branch_search(&mut self) {
        self.branch_input_mode = BranchInputMode::Search;
        self.branch_search_query.clear();
        self.branch_selected_index = 0;
    }

    /// Branch 検索をキャンセル
    pub fn cancel_branch_search(&mut self) {
        self.branch_input_mode = BranchInputMode::None;
        self.branch_search_query.clear();
        self.rebuild_filtered_branches();
        self.branch_selected_index = 0;
    }

    /// Branch 検索を確定
    pub fn confirm_branch_search(&mut self) {
        self.branch_input_mode = BranchInputMode::None;
        // 検索クエリは保持したまま、選択状態を維持
    }

    /// Branch 検索クエリに文字を追加
    pub fn branch_search_push(&mut self, c: char) {
        self.branch_search_query.push(c);
        self.rebuild_filtered_branches();
        // 検索結果が変わる可能性があるので選択インデックスをリセット
        self.branch_selected_index = 0;
    }

    /// Branch 検索クエリから文字を削除
    pub fn branch_search_pop(&mut self) {
        self.branch_search_query.pop();
        self.rebuild_filtered_branches();
        // 検索結果が変わる可能性があるので選択インデックスをリセット
        self.branch_selected_index = 0;
    }

    /// Branch 検索をクリア
    pub fn clear_branch_search(&mut self) {
        self.branch_search_query.clear();
        self.rebuild_filtered_branches();
        self.branch_selected_index = 0;
    }

    /// ブランチチェックアウトの確認ダイアログを表示
    pub fn show_branch_checkout_confirm(&mut self) {
        let branch_info = self
            .selected_branch()
            .map(|b| (b.is_current, b.name.clone()));
        if let Some((is_current, name)) = branch_info {
            if is_current {
                self.set_status_message("Already on this branch");
                return;
            }
            self.confirm_selected_yes = false;
            self.confirm_dialog = ConfirmDialog::CheckoutBranch { branch_name: name };
        }
    }

    /// ブランチをチェックアウトする
    ///
    /// # Errors
    /// - Git コマンドの実行に失敗した場合
    pub fn checkout_branch(&mut self) -> Result<()> {
        let name = match self.confirm_dialog {
            ConfirmDialog::CheckoutBranch { ref branch_name } => branch_name.clone(),
            _ => return Ok(()),
        };

        // 未コミットの変更があるかチェック
        let has_changes = self.status_cache.iter().any(|f| {
            f.index != StatusKind::Unmodified
                || f.worktree != StatusKind::Unmodified
                || f.index == StatusKind::Untracked
        });
        if has_changes {
            self.confirm_dialog = ConfirmDialog::None;
            self.set_status_message(
                "Cannot switch: uncommitted changes exist. Stash or commit first.",
            );
            return Ok(());
        }

        // リモートブランチの場合はトラッキングブランチとしてチェックアウト
        let selected = self.selected_branch();
        let is_remote = selected.is_some_and(|b| b.is_remote);

        let result = if is_remote {
            // remote/branch-name から branch-name を抽出
            // 最初の `/` 以降をローカルブランチ名とする（origin/, upstream/ 等に対応）
            let local_name = name.find('/').map_or(&name[..], |pos| &name[pos + 1..]);
            self.git
                .execute(&["checkout", "-t", &name, "-b", local_name])
        } else {
            self.git.execute(&["checkout", &name])
        };

        self.confirm_dialog = ConfirmDialog::None;

        match result {
            Ok(_) => {
                self.set_status_message(&format!("Switched to branch '{}'", name));
                self.refresh_branches()?;
                self.refresh_status()?;
            }
            Err(e) => {
                self.set_status_message(&format!("Failed to switch branch: {}", e));
            }
        }
        Ok(())
    }
}
