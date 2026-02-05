use std::path::Path;

use crate::cli::{parse_status, GitCli};
use crate::domain::{FileStatus, NodeKind, StatusKind, TreeNode};
use crate::error::Result;
use crate::filter::DisplayFilter;
use crate::ui::tree_view;

/// 現在表示している View の種類
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Status,
    Tree,
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
    /// リポジトリルートパス
    #[allow(dead_code)] // TODO(Phase 3): Log View で使用予定
    repo_root: std::path::PathBuf,
    /// 現在の View
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
    /// Tree View のルートノード
    pub tree_root: TreeNode,
    /// Tree View での選択インデックス
    pub tree_selected_index: usize,
    /// 表示フィルタ
    pub display_filter: DisplayFilter,
}

impl AppState {
    /// 新しい AppState を作成する
    ///
    /// # Errors
    /// - 指定されたパスが Git リポジトリ内でない場合
    pub fn new(path: &Path) -> Result<Self> {
        let git = GitCli::new(path)?;
        let repo_root = git.repo_root().to_path_buf();

        // ルートノードを作成（子は未ロード）
        let mut tree_root = TreeNode::new_dir(
            repo_root
                .file_name()
                .map_or("root".to_string(), |n| n.to_string_lossy().to_string()),
            repo_root.clone(),
        );

        // ルートの子だけをロード（遅延ロードの例外：起動時にルート直下は表示）
        let _ = tree_root.load_children();

        let mut state = Self {
            git,
            repo_root,
            current_view: View::Status,
            selected_index: 0,
            status_cache: Vec::new(),
            confirm_dialog: ConfirmDialog::None,
            status_message: None,
            should_quit: false,
            tree_root,
            tree_selected_index: 0,
            display_filter: DisplayFilter::load(),
        };
        // 起動時に1回だけ status を取得
        state.refresh_status()?;
        // Tree のステータスを適用
        state.tree_root.apply_status_cache(&state.status_cache);
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

    /// View を切り替える
    pub fn switch_view(&mut self, view: View) {
        if self.current_view != view {
            self.current_view = view;
            self.clear_status_message();
            // View 切り替え時に status を更新してツリーに適用
            if view == View::Tree {
                let _ = self.refresh_status();
                self.apply_status_to_tree();
            }
        }
    }

    /// 選択を1つ下に移動
    pub fn select_next(&mut self) {
        match self.current_view {
            View::Status => {
                if !self.status_cache.is_empty() {
                    self.selected_index =
                        (self.selected_index + 1).min(self.status_cache.len() - 1);
                }
            }
            View::Tree => {
                let max = tree_view::get_flat_tree_len(&self.tree_root, &self.display_filter);
                if max > 0 {
                    self.tree_selected_index = (self.tree_selected_index + 1).min(max - 1);
                }
            }
        }
    }

    /// 選択を1つ上に移動
    pub fn select_previous(&mut self) {
        match self.current_view {
            View::Status => {
                self.selected_index = self.selected_index.saturating_sub(1);
            }
            View::Tree => {
                self.tree_selected_index = self.tree_selected_index.saturating_sub(1);
            }
        }
    }

    /// 選択を先頭に移動
    pub fn select_first(&mut self) {
        match self.current_view {
            View::Status => {
                self.selected_index = 0;
            }
            View::Tree => {
                self.tree_selected_index = 0;
            }
        }
    }

    /// 選択を末尾に移動
    pub fn select_last(&mut self) {
        match self.current_view {
            View::Status => {
                if !self.status_cache.is_empty() {
                    self.selected_index = self.status_cache.len() - 1;
                }
            }
            View::Tree => {
                let max = tree_view::get_flat_tree_len(&self.tree_root, &self.display_filter);
                if max > 0 {
                    self.tree_selected_index = max - 1;
                }
            }
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

    // --- Tree View 用メソッド ---

    /// 選択されているツリーノードを展開/折りたたみする
    #[allow(dead_code)] // TODO(Phase 2): toggle 機能は Enter キーでのみ使用予定
    pub fn toggle_tree_node(&mut self) {
        // borrow checker 対策: status_cache を先にクローン
        let cache = self.status_cache.clone();

        if let Some(node) = self.get_selected_tree_node_mut() {
            if node.kind == NodeKind::Directory {
                if node.expanded {
                    // 折りたたみ
                    node.expanded = false;
                } else {
                    // 展開
                    if node.children.is_none() {
                        // 遅延ロード
                        let _ = node.load_children();
                        node.apply_status_cache(&cache);
                    }
                    node.expanded = true;
                }
            }
        }
    }

    /// 選択されているツリーノードを展開する
    pub fn expand_tree_node(&mut self) {
        // borrow checker 対策: status_cache を先にクローン
        let cache = self.status_cache.clone();

        if let Some(node) = self.get_selected_tree_node_mut() {
            if node.kind == NodeKind::Directory && !node.expanded {
                if node.children.is_none() {
                    // 遅延ロード
                    let _ = node.load_children();
                    node.apply_status_cache(&cache);
                }
                node.expanded = true;
            }
        }
    }

    /// 選択されているツリーノードを折りたたむ
    pub fn collapse_tree_node(&mut self) {
        if let Some(node) = self.get_selected_tree_node_mut() {
            if node.kind == NodeKind::Directory && node.expanded {
                node.expanded = false;
            }
        }
    }

    /// 表示フィルタを切り替える
    pub fn toggle_display_filter(&mut self) {
        self.display_filter.toggle();
        // 選択インデックスを調整
        let max = tree_view::get_flat_tree_len(&self.tree_root, &self.display_filter);
        if max > 0 && self.tree_selected_index >= max {
            self.tree_selected_index = max - 1;
        }
    }

    /// Tree View のステータスをリフレッシュする（公開メソッド）
    ///
    /// Tree View で R キーを押した後に呼び出す
    pub fn refresh_tree_status(&mut self) {
        self.apply_status_to_tree();
    }

    /// Status キャッシュをツリーに適用する
    fn apply_status_to_tree(&mut self) {
        // ルートノードにステータスを適用
        self.tree_root.apply_status_cache(&self.status_cache);

        // 展開されている子ノードにも再帰的に適用
        fn apply_recursive(node: &mut TreeNode, cache: &[FileStatus]) {
            if let Some(children) = &mut node.children {
                for child in children {
                    child.apply_status_cache(cache);
                    if child.expanded {
                        apply_recursive(child, cache);
                    }
                }
            }
        }
        apply_recursive(&mut self.tree_root, &self.status_cache);
    }

    /// 選択されているツリーノードへの可変参照を取得する
    fn get_selected_tree_node_mut(&mut self) -> Option<&mut TreeNode> {
        let index = self.tree_selected_index;
        let filter = &self.display_filter;

        // フラット化してインデックスを取得し、対応するノードを探す
        let flat = tree_view::flatten_tree(&self.tree_root, filter, 0);
        if let Some((target_node, _)) = flat.get(index) {
            let target_path = target_node.path.clone();
            // パスでノードを探して可変参照を返す
            find_node_by_path_mut(&mut self.tree_root, &target_path)
        } else {
            None
        }
    }
}

/// パスでノードを探して可変参照を返す
fn find_node_by_path_mut<'a>(
    node: &'a mut TreeNode,
    path: &std::path::Path,
) -> Option<&'a mut TreeNode> {
    if node.path == path {
        return Some(node);
    }

    if let Some(children) = &mut node.children {
        for child in children {
            if let Some(found) = find_node_by_path_mut(child, path) {
                return Some(found);
            }
        }
    }

    None
}
