use crate::cli::parse_status;
use crate::domain::{FileStatus, StatusKind};
use crate::error::Result;

use super::{AppState, ConfirmDialog};

impl AppState {
    /// Git status を再取得してキャッシュを更新する
    ///
    /// このメソッドは以下のタイミングでのみ呼び出すこと:
    /// - View 切り替え時
    /// - 手動リフレッシュ時（R キー）
    /// - 変更操作の直後（commit, stash, branch checkout 等）
    pub fn refresh_status(&mut self) -> Result<()> {
        let output = self.git.execute(&["status", "--porcelain=v1"])?;
        self.status_cache = parse_status(&output)?;
        // 選択インデックスが範囲外になった場合は調整
        if !self.status_cache.is_empty() && self.selected_index >= self.status_cache.len() {
            self.selected_index = self.status_cache.len() - 1;
        }
        // 手動リフレッシュ時にバックグラウンド結果を破棄
        self.bg_status_receiver = None;
        Ok(())
    }

    /// tracked ファイルのみの Git status を取得する（起動時の高速読み込み用）
    ///
    /// `-uno` オプションにより untracked ファイルを除外し、取得を高速化する。
    pub(super) fn refresh_status_tracked_only(&mut self) -> Result<()> {
        let output = self.git.execute(&["status", "--porcelain=v1", "-uno"])?;
        self.status_cache = parse_status(&output)?;
        Ok(())
    }

    /// 現在選択されているファイルを取得
    pub fn selected_file(&self) -> Option<&FileStatus> {
        self.status_cache.get(self.selected_index)
    }

    /// 選択されているファイルをステージする
    ///
    /// # Errors
    /// - Git コマンドの実行に失敗した場合
    pub fn stage(&mut self) -> Result<()> {
        if let Some(file) = self.status_cache.get(self.selected_index).cloned() {
            let path_str = file.path.to_string_lossy().to_string();
            self.git.execute(&["add", &path_str])?;
            self.optimistic_update_after_stage(&file);
        }
        Ok(())
    }

    /// 選択されているファイルをアンステージする
    ///
    /// # Errors
    /// - Git コマンドの実行に失敗した場合
    pub fn unstage(&mut self) -> Result<()> {
        if let Some(file) = self.status_cache.get(self.selected_index).cloned() {
            let path_str = file.path.to_string_lossy().to_string();
            self.git.execute(&["restore", "--staged", &path_str])?;
            self.optimistic_update_unstage(&file)?;
        }
        Ok(())
    }

    /// ステージ操作後の楽観的キャッシュ更新
    ///
    /// git add 成功後に status_cache をローカルで即時更新する。
    /// Renamed / Copied は楽観的更新が困難なため対応しない（呼び出し元でフォールバック）。
    pub(super) fn optimistic_update_after_stage(&mut self, file: &FileStatus) {
        if let Some(entry) = self.status_cache.iter_mut().find(|f| f.path == file.path) {
            match file.worktree {
                StatusKind::Untracked => {
                    entry.index = StatusKind::Added;
                    entry.worktree = StatusKind::Unmodified;
                }
                StatusKind::Modified => {
                    entry.index = StatusKind::Modified;
                    entry.worktree = StatusKind::Unmodified;
                }
                StatusKind::Deleted => {
                    entry.index = StatusKind::Deleted;
                    entry.worktree = StatusKind::Unmodified;
                }
                _ => {
                    entry.index = file.worktree;
                    entry.worktree = StatusKind::Unmodified;
                }
            }
            // 両方 Unmodified になったらエントリを削除
            if entry.index == StatusKind::Unmodified && entry.worktree == StatusKind::Unmodified {
                self.status_cache.retain(|f| f.path != file.path);
                if !self.status_cache.is_empty() && self.selected_index >= self.status_cache.len() {
                    self.selected_index = self.status_cache.len() - 1;
                }
            }
        }
        self.tree_flat_dirty = true;
        // バックグラウンド status 結果を破棄（楽観的更新と競合を避ける）
        self.bg_status_receiver = None;
    }

    /// アンステージ操作後の楽観的キャッシュ更新
    ///
    /// git restore --staged 成功後に status_cache をローカルで即時更新する。
    /// Renamed / Copied はパスの変更を伴う可能性があるため refresh_status にフォールバック。
    pub(super) fn optimistic_update_unstage(&mut self, file: &FileStatus) -> Result<()> {
        // Renamed / Copied は楽観的更新が困難なため refresh_status にフォールバック
        if file.index == StatusKind::Renamed || file.index == StatusKind::Copied {
            self.refresh_status()?;
            return Ok(());
        }

        if let Some(entry) = self.status_cache.iter_mut().find(|f| f.path == file.path) {
            match file.index {
                StatusKind::Added => {
                    // 新規追加ファイルの unstage → untracked に戻る
                    entry.index = StatusKind::Untracked;
                    entry.worktree = StatusKind::Untracked;
                }
                StatusKind::Modified => {
                    entry.worktree = StatusKind::Modified;
                    entry.index = StatusKind::Unmodified;
                }
                StatusKind::Deleted => {
                    entry.worktree = StatusKind::Deleted;
                    entry.index = StatusKind::Unmodified;
                }
                _ => {
                    entry.worktree = file.index;
                    entry.index = StatusKind::Unmodified;
                }
            }
            // 両方 Unmodified になったらエントリを削除
            if entry.index == StatusKind::Unmodified && entry.worktree == StatusKind::Unmodified {
                self.status_cache.retain(|f| f.path != file.path);
                if !self.status_cache.is_empty() && self.selected_index >= self.status_cache.len() {
                    self.selected_index = self.status_cache.len() - 1;
                }
            }
        }
        self.tree_flat_dirty = true;
        // バックグラウンド status 結果を破棄（楽観的更新と競合を避ける）
        self.bg_status_receiver = None;
        Ok(())
    }

    /// 選択されているファイルの差分を取得する（色付き）
    ///
    /// # Errors
    /// - Git コマンドの実行に失敗した場合
    pub fn get_diff(&self) -> Result<String> {
        let file = match self.status_cache.get(self.selected_index) {
            Some(f) => f,
            None => return Ok(String::new()),
        };
        let path_str = file.path.to_string_lossy();
        // ステージされている場合は --cached を付ける
        if file.index != StatusKind::Unmodified && file.index != StatusKind::Untracked {
            self.git
                .execute(&["diff", "--cached", "--color=always", path_str.as_ref()])
        } else {
            self.git
                .execute(&["diff", "--color=always", path_str.as_ref()])
        }
    }

    /// 変更破棄の確認ダイアログを表示する
    pub fn show_discard_confirm(&mut self) {
        if self.selected_file().is_some() {
            self.confirm_dialog = ConfirmDialog::DiscardChanges {
                file_index: self.selected_index,
            };
        }
    }

    /// 変更破棄を実行する
    ///
    /// # Errors
    /// - Git コマンドの実行に失敗した場合
    pub fn discard_changes(&mut self) -> Result<()> {
        let file_index = match self.confirm_dialog {
            ConfirmDialog::DiscardChanges { file_index } => file_index,
            _ => return Ok(()),
        };
        let file = match self.status_cache.get(file_index).cloned() {
            Some(f) => f,
            None => return Ok(()),
        };
        let path_str = file.path.to_string_lossy().to_string();

        // ステージ済みの変更を破棄
        if file.index != StatusKind::Unmodified && file.index != StatusKind::Untracked {
            self.git.execute(&["restore", "--staged", &path_str])?;
        }

        // ワークツリーの変更を破棄
        if file.worktree != StatusKind::Unmodified {
            self.git.execute(&["restore", &path_str])?;
        }

        self.confirm_dialog = ConfirmDialog::None;

        // 楽観的更新: エントリをキャッシュから削除
        self.status_cache.retain(|f| f.path != file.path);
        // 選択インデックスの範囲外チェック
        if !self.status_cache.is_empty() && self.selected_index >= self.status_cache.len() {
            self.selected_index = self.status_cache.len() - 1;
        } else if self.status_cache.is_empty() {
            self.selected_index = 0;
        }
        self.tree_flat_dirty = true;
        // バックグラウンド status 結果を破棄（楽観的更新と競合を避ける）
        self.bg_status_receiver = None;
        Ok(())
    }

    /// ステージされたファイルの一覧を取得
    pub fn get_staged_files(&self) -> Vec<&FileStatus> {
        self.status_cache
            .iter()
            .filter(|f| f.index != StatusKind::Unmodified && f.index != StatusKind::Untracked)
            .collect()
    }
}
