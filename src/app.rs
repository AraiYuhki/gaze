use std::path::Path;

use crate::cli::{parse_status, GitCli};
use crate::domain::{FileStatus, StatusKind};
use crate::error::Result;

/// 現在表示している View の種類
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Status,
    // TODO(Phase 2): Tree を追加
    // TODO(Phase 3): Log を追加
}

/// 確認ダイアログの状態
#[derive(Debug, Clone)]
pub enum ConfirmDialog {
    /// ダイアログなし
    None,
    /// 変更破棄の確認
    DiscardChanges { file_index: usize },
}

/// アプリケーション全体の状態
pub struct AppState {
    /// Git CLI 実行インスタンス
    git: GitCli,
    /// 現在の View
    // TODO(Phase 2): View 切り替え機能で使用予定
    #[allow(dead_code)]
    pub current_view: View,
    /// Status View での選択インデックス
    pub selected_index: usize,
    /// Git status のキャッシュ
    pub status_cache: Vec<FileStatus>,
    /// 確認ダイアログの状態
    pub confirm_dialog: ConfirmDialog,
    /// ステータスバーに表示するメッセージ
    pub status_message: Option<String>,
    /// アプリケーション終了フラグ
    pub should_quit: bool,
}

impl AppState {
    /// 新しい AppState を作成する
    ///
    /// # Errors
    /// - 指定されたパスが Git リポジトリ内でない場合
    pub fn new(path: &Path) -> Result<Self> {
        let git = GitCli::new(path)?;
        let mut state = Self {
            git,
            current_view: View::Status,
            selected_index: 0,
            status_cache: Vec::new(),
            confirm_dialog: ConfirmDialog::None,
            status_message: None,
            should_quit: false,
        };
        // 起動時に1回だけ status を取得
        state.refresh_status()?;
        Ok(state)
    }

    /// Git status を再取得してキャッシュを更新する
    ///
    /// このメソッドは以下のタイミングでのみ呼び出すこと:
    /// - アプリ起動時
    /// - View 切り替え時
    /// - 手動リフレッシュ時（R キー）
    /// - 変更操作の直後（s, r 等の操作完了後）
    pub fn refresh_status(&mut self) -> Result<()> {
        let output = self.git.execute(&["status", "--porcelain=v1"])?;
        self.status_cache = parse_status(&output)?;
        // 選択インデックスが範囲外になった場合は調整
        if !self.status_cache.is_empty() && self.selected_index >= self.status_cache.len() {
            self.selected_index = self.status_cache.len() - 1;
        }
        Ok(())
    }

    /// 選択を1つ下に移動
    pub fn select_next(&mut self) {
        if !self.status_cache.is_empty() {
            self.selected_index = (self.selected_index + 1).min(self.status_cache.len() - 1);
        }
    }

    /// 選択を1つ上に移動
    pub fn select_previous(&mut self) {
        self.selected_index = self.selected_index.saturating_sub(1);
    }

    /// 選択を先頭に移動
    pub fn select_first(&mut self) {
        self.selected_index = 0;
    }

    /// 選択を末尾に移動
    pub fn select_last(&mut self) {
        if !self.status_cache.is_empty() {
            self.selected_index = self.status_cache.len() - 1;
        }
    }

    /// 現在選択されているファイルを取得
    pub fn selected_file(&self) -> Option<&FileStatus> {
        self.status_cache.get(self.selected_index)
    }

    /// 選択されているファイルをステージ/アンステージする
    ///
    /// # Errors
    /// - Git コマンドの実行に失敗した場合
    pub fn toggle_stage(&mut self) -> Result<()> {
        if let Some(file) = self.status_cache.get(self.selected_index) {
            let path_str = file.path.to_string_lossy();
            // インデックスにステージされていれば unstage、そうでなければ stage
            if file.index != StatusKind::Unmodified && file.index != StatusKind::Untracked {
                // Unstage: git restore --staged <file>
                self.git
                    .execute(&["restore", "--staged", path_str.as_ref()])?;
            } else {
                // Stage: git add <file>
                self.git.execute(&["add", path_str.as_ref()])?;
            }
            // 操作完了後に1回だけ status を再取得
            self.refresh_status()?;
        }
        Ok(())
    }

    /// 選択されているファイルの差分を取得する
    ///
    /// # Errors
    /// - Git コマンドの実行に失敗した場合
    pub fn get_diff(&self) -> Result<String> {
        if let Some(file) = self.status_cache.get(self.selected_index) {
            let path_str = file.path.to_string_lossy();
            // ステージされている場合は --cached を付ける
            if file.index != StatusKind::Unmodified && file.index != StatusKind::Untracked {
                self.git.execute(&["diff", "--cached", path_str.as_ref()])
            } else {
                self.git.execute(&["diff", path_str.as_ref()])
            }
        } else {
            Ok(String::new())
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

    /// 確認ダイアログをキャンセルする
    pub fn cancel_confirm(&mut self) {
        self.confirm_dialog = ConfirmDialog::None;
    }

    /// 変更破棄を実行する
    ///
    /// # Errors
    /// - Git コマンドの実行に失敗した場合
    pub fn discard_changes(&mut self) -> Result<()> {
        if let ConfirmDialog::DiscardChanges { file_index } = self.confirm_dialog {
            if let Some(file) = self.status_cache.get(file_index) {
                let path_str = file.path.to_string_lossy();
                // git restore <file> で変更を破棄
                self.git.execute(&["restore", path_str.as_ref()])?;
                self.confirm_dialog = ConfirmDialog::None;
                // 操作完了後に1回だけ status を再取得
                self.refresh_status()?;
            }
        }
        Ok(())
    }

    /// ステータスメッセージを設定する
    pub fn set_status_message(&mut self, message: &str) {
        self.status_message = Some(message.to_string());
    }

    /// ステータスメッセージをクリアする
    pub fn clear_status_message(&mut self) {
        self.status_message = None;
    }
}
