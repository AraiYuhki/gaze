use std::path::{Path, PathBuf};

use crate::cli::{
    generate_patch, parse_branch_list, parse_diff_hunks, parse_log, parse_stash_list, parse_status,
    GitCli,
};
use crate::domain::{
    build_status_map, BranchEntry, FileDiff, FileStatus, GraphLine, NodeKind, StashEntry,
    StatusKind, TreeNode,
};
use crate::error::Result;
use crate::filter::DisplayFilter;
use crate::ui::tree_view;

/// 現在表示している View の種類
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Status,
    Tree,
    Log,
    Stash,
    Branch,
}

/// 確認ダイアログの状態
#[derive(Debug, Clone)]
pub enum ConfirmDialog {
    /// ダイアログなし
    None,
    /// 変更破棄の確認
    DiscardChanges { file_index: usize },
    /// チェックアウトの確認
    Checkout { commit_hash: String },
    /// Amend 確認
    Amend,
    /// Stash 削除の確認
    DropStash { stash_index: usize },
    /// ブランチ切り替えの確認
    CheckoutBranch { branch_name: String },
    /// Push の確認
    Push,
    /// Pull の確認
    Pull,
}

/// コミットモードの種類
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitMode {
    /// コミットモードではない
    None,
    /// 通常のコミット
    Normal,
    /// Amend コミット
    Amend,
}

/// Stash メッセージ入力モード
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StashInputMode {
    /// 入力モードではない
    None,
    /// Stash push 用のメッセージ入力
    Push,
}

/// Branch View 入力モード
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchInputMode {
    /// 入力モードではない
    None,
    /// 検索入力中
    Search,
}

/// Log View で取得するコミット数
const LOG_LIMIT: usize = 200;

/// アプリケーション全体の状態
pub struct AppState {
    /// Git CLI 実行インスタンス
    git: GitCli,
    /// リポジトリルートパス
    #[allow(dead_code)] // 将来のリファクタリングや拡張で使用予定
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
    /// フラット化されたツリーのキャッシュ（パスと深さ）
    /// ツリー構造が変わった時に tree_flat_dirty を true にして再構築する
    tree_flat_cache: Vec<(PathBuf, usize)>,
    /// フラットキャッシュが無効化されているか
    tree_flat_dirty: bool,
    /// Log View のキャッシュ
    pub log_cache: Vec<GraphLine>,
    /// Log View での選択インデックス
    pub log_selected_index: usize,
    /// ヘルプ画面表示フラグ
    pub show_help: bool,
    /// Tree View 検索モード
    pub tree_search_mode: bool,
    /// Tree View 検索文字列
    pub tree_search_query: String,
    /// Tree View 検索マッチパスリスト
    pub tree_search_matches: Vec<std::path::PathBuf>,
    /// Tree View 現在のマッチインデックス
    pub tree_search_current_match: usize,
    /// コミットモード
    pub commit_mode: CommitMode,
    /// コミットメッセージ（複数行）
    pub commit_message: Vec<String>,
    /// コミットメッセージのカーソル位置（X座標: 行内の文字位置）
    pub commit_cursor_x: usize,
    /// コミットメッセージのカーソル位置（Y座標: 行番号）
    pub commit_cursor_y: usize,
    /// コミット画面でのステージファイル選択インデックス
    pub commit_file_index: usize,
    /// コミット画面でファイル一覧にフォーカスしているか
    pub commit_focus_files: bool,
    /// Stash 一覧のキャッシュ
    pub stash_cache: Vec<StashEntry>,
    /// Stash View での選択インデックス
    pub stash_selected_index: usize,
    /// Stash メッセージ入力モード
    pub stash_input_mode: StashInputMode,
    /// Stash メッセージ
    pub stash_message: String,
    /// Branch 一覧のキャッシュ
    pub branch_cache: Vec<BranchEntry>,
    /// Branch View での選択インデックス
    pub branch_selected_index: usize,
    /// Branch 検索クエリ
    pub branch_search_query: String,
    /// Branch View 入力モード
    pub branch_input_mode: BranchInputMode,
    /// フィルタ済み Branch インデックスのキャッシュ
    /// branch_cache と branch_search_query が変わった時に再構築する
    filtered_branch_indices: Vec<usize>,
    /// Hunk モードかどうか
    pub hunk_mode: bool,
    /// Hunk モードのファイル diff 一覧
    pub hunk_file_diffs: Vec<FileDiff>,
    /// Hunk モードでの選択インデックス（フラット化されたリスト上のインデックス）
    pub hunk_selected_index: usize,
    /// Hunk モードの対象ファイルパス
    pub hunk_target_path: Option<PathBuf>,
    /// Hunk モードが --cached（ステージ済み）の diff を対象にしているか
    hunk_is_cached: bool,
    /// ファイルログ表示中のファイルパス（None の場合は通常の Log View）
    pub file_log_path: Option<std::path::PathBuf>,
    /// ファイルログのキャッシュ
    pub file_log_cache: Vec<GraphLine>,
    /// ファイルログでの選択インデックス
    pub file_log_selected_index: usize,
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
            tree_flat_cache: Vec::new(),
            tree_flat_dirty: true,
            log_cache: Vec::new(),
            log_selected_index: 0,
            show_help: false,
            tree_search_mode: false,
            tree_search_query: String::new(),
            tree_search_matches: Vec::new(),
            tree_search_current_match: 0,
            commit_mode: CommitMode::None,
            commit_message: vec![String::new()],
            commit_cursor_x: 0,
            commit_cursor_y: 0,
            commit_file_index: 0,
            commit_focus_files: false,
            stash_cache: Vec::new(),
            stash_selected_index: 0,
            stash_input_mode: StashInputMode::None,
            stash_message: String::new(),
            branch_cache: Vec::new(),
            branch_selected_index: 0,
            branch_search_query: String::new(),
            branch_input_mode: BranchInputMode::None,
            filtered_branch_indices: Vec::new(),
            hunk_mode: false,
            hunk_file_diffs: Vec::new(),
            hunk_selected_index: 0,
            hunk_target_path: None,
            hunk_is_cached: false,
            file_log_path: None,
            file_log_cache: Vec::new(),
            file_log_selected_index: 0,
        };
        // 起動時に1回だけ status を取得
        state.refresh_status()?;
        // Tree のステータスを適用
        let status_map = build_status_map(&state.status_cache);
        state.tree_root.apply_status_map(&status_map);
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
            match view {
                View::Tree => {
                    // Tree View 切り替え時に status を更新してツリーに適用
                    let _ = self.refresh_status();
                    self.apply_status_to_tree();
                }
                View::Log => {
                    // Log View 切り替え時にファイルログモードをクリアしてログを取得
                    self.clear_file_log();
                    let _ = self.refresh_log();
                }
                View::Status => {
                    let _ = self.refresh_status();
                }
                View::Stash => {
                    // Stash View 切り替え時に stash 一覧を取得
                    let _ = self.refresh_stash();
                }
                View::Branch => {
                    // Branch View 切り替え時に branch 一覧と status を取得
                    // status はチェックアウト時の uncommitted changes 検出に必要
                    let _ = self.refresh_status();
                    let _ = self.refresh_branches();
                }
            }
        }
    }

    /// Git log を再取得してキャッシュを更新する
    pub fn refresh_log(&mut self) -> Result<()> {
        let output = self.git.execute(&[
            "log",
            "--oneline",
            "--graph",
            "--all",
            "-n",
            &LOG_LIMIT.to_string(),
        ])?;
        self.log_cache = parse_log(&output);
        // 選択インデックスが範囲外になった場合は調整
        if !self.log_cache.is_empty() && self.log_selected_index >= self.log_cache.len() {
            self.log_selected_index = self.log_cache.len() - 1;
        }
        Ok(())
    }

    /// 特定ファイルの Git log を取得してキャッシュを更新する
    ///
    /// # Arguments
    /// * `path` - ログを取得するファイルのパス（リポジトリルートからの相対パス）
    pub fn refresh_file_log(&mut self, path: &Path) -> Result<()> {
        let path_str = path.to_string_lossy();
        let output = self.git.execute(&[
            "log",
            "--oneline",
            "--follow",
            "-n",
            &LOG_LIMIT.to_string(),
            "--",
            &path_str,
        ])?;
        self.file_log_cache = parse_log(&output);
        self.file_log_selected_index = 0;
        self.file_log_path = Some(path.to_path_buf());
        Ok(())
    }

    /// ファイルログモードをクリアして通常の Log View に戻る
    pub fn clear_file_log(&mut self) {
        self.file_log_path = None;
        self.file_log_cache.clear();
        self.file_log_selected_index = 0;
    }

    /// ファイルログモードかどうかを返す
    pub fn is_file_log_mode(&self) -> bool {
        self.file_log_path.is_some()
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
                let max = self.get_tree_flat_len();
                if max > 0 {
                    self.tree_selected_index = (self.tree_selected_index + 1).min(max - 1);
                }
            }
            View::Log => {
                if self.is_file_log_mode() {
                    if !self.file_log_cache.is_empty() {
                        self.file_log_selected_index =
                            (self.file_log_selected_index + 1).min(self.file_log_cache.len() - 1);
                    }
                } else if !self.log_cache.is_empty() {
                    self.log_selected_index =
                        (self.log_selected_index + 1).min(self.log_cache.len() - 1);
                }
            }
            View::Stash => {
                if !self.stash_cache.is_empty() {
                    self.stash_selected_index =
                        (self.stash_selected_index + 1).min(self.stash_cache.len() - 1);
                }
            }
            View::Branch => {
                let filtered = self.filtered_branches();
                if !filtered.is_empty() {
                    self.branch_selected_index =
                        (self.branch_selected_index + 1).min(filtered.len() - 1);
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
            View::Log => {
                if self.is_file_log_mode() {
                    self.file_log_selected_index = self.file_log_selected_index.saturating_sub(1);
                } else {
                    self.log_selected_index = self.log_selected_index.saturating_sub(1);
                }
            }
            View::Stash => {
                self.stash_selected_index = self.stash_selected_index.saturating_sub(1);
            }
            View::Branch => {
                self.branch_selected_index = self.branch_selected_index.saturating_sub(1);
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
            View::Log => {
                if self.is_file_log_mode() {
                    self.file_log_selected_index = 0;
                } else {
                    self.log_selected_index = 0;
                }
            }
            View::Stash => {
                self.stash_selected_index = 0;
            }
            View::Branch => {
                self.branch_selected_index = 0;
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
                let max = self.get_tree_flat_len();
                if max > 0 {
                    self.tree_selected_index = max - 1;
                }
            }
            View::Log => {
                if self.is_file_log_mode() {
                    if !self.file_log_cache.is_empty() {
                        self.file_log_selected_index = self.file_log_cache.len() - 1;
                    }
                } else if !self.log_cache.is_empty() {
                    self.log_selected_index = self.log_cache.len() - 1;
                }
            }
            View::Stash => {
                if !self.stash_cache.is_empty() {
                    self.stash_selected_index = self.stash_cache.len() - 1;
                }
            }
            View::Branch => {
                let filtered = self.filtered_branches();
                if !filtered.is_empty() {
                    self.branch_selected_index = filtered.len() - 1;
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

    /// 選択されているファイルの差分を取得する（色付き）
    ///
    /// # Errors
    /// - Git コマンドの実行に失敗した場合
    pub fn get_diff(&self) -> Result<String> {
        if let Some(file) = self.status_cache.get(self.selected_index) {
            let path_str = file.path.to_string_lossy();
            // ステージされている場合は --cached を付ける
            if file.index != StatusKind::Unmodified && file.index != StatusKind::Untracked {
                self.git
                    .execute(&["diff", "--cached", "--color=always", path_str.as_ref()])
            } else {
                self.git
                    .execute(&["diff", "--color=always", path_str.as_ref()])
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
            if let Some(file) = self.status_cache.get(file_index).cloned() {
                let path_str = file.path.to_string_lossy();

                // ステージ済みの変更を破棄
                if file.index != StatusKind::Unmodified && file.index != StatusKind::Untracked {
                    self.git
                        .execute(&["restore", "--staged", path_str.as_ref()])?;
                }

                // ワークツリーの変更を破棄
                if file.worktree != StatusKind::Unmodified {
                    self.git.execute(&["restore", path_str.as_ref()])?;
                }

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

    // --- Tree View フラットキャッシュ管理 ---

    /// フラットキャッシュを無効化する
    ///
    /// ツリー構造に影響する操作（展開/折りたたみ/フィルタ変更等）の後に呼び出す
    fn invalidate_tree_flat_cache(&mut self) {
        self.tree_flat_dirty = true;
    }

    /// フラットキャッシュを必要に応じて再構築する
    fn ensure_tree_flat_cache(&mut self) {
        if !self.tree_flat_dirty {
            return;
        }
        let flat = tree_view::flatten_tree(&self.tree_root, &self.display_filter, 0);
        self.tree_flat_cache = flat
            .into_iter()
            .map(|(node, depth)| (node.path.clone(), depth))
            .collect();
        self.tree_flat_dirty = false;
    }

    /// キャッシュ済みのフラットツリーの長さを取得する
    pub fn get_tree_flat_len(&mut self) -> usize {
        self.ensure_tree_flat_cache();
        self.tree_flat_cache.len()
    }

    /// キャッシュ済みのフラットツリーから指定インデックスのパスを取得する
    pub fn get_tree_flat_path(&mut self, index: usize) -> Option<PathBuf> {
        self.ensure_tree_flat_cache();
        self.tree_flat_cache.get(index).map(|(p, _)| p.clone())
    }

    // --- Tree View 用メソッド ---

    /// 選択されているツリーノードを展開/折りたたみする
    #[allow(dead_code)] // TODO(Phase 2): toggle 機能は Enter キーでのみ使用予定
    pub fn toggle_tree_node(&mut self) {
        // borrow checker 対策: status_map を先に構築
        let status_map = build_status_map(&self.status_cache);

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
                        node.apply_status_map(&status_map);
                    }
                    node.expanded = true;
                }
                self.invalidate_tree_flat_cache();
            }
        }
    }

    /// 選択されているツリーノードを展開する
    pub fn expand_tree_node(&mut self) {
        // borrow checker 対策: status_map を先に構築
        let status_map = build_status_map(&self.status_cache);

        if let Some(node) = self.get_selected_tree_node_mut() {
            if node.kind == NodeKind::Directory && !node.expanded {
                if node.children.is_none() {
                    // 遅延ロード
                    let _ = node.load_children();
                    node.apply_status_map(&status_map);
                }
                node.expanded = true;
                self.invalidate_tree_flat_cache();
            }
        }
    }

    /// 選択されているツリーノードを折りたたむ
    pub fn collapse_tree_node(&mut self) {
        if let Some(node) = self.get_selected_tree_node_mut() {
            if node.kind == NodeKind::Directory && node.expanded {
                node.expanded = false;
                self.invalidate_tree_flat_cache();
            }
        }
    }

    /// 表示フィルタを切り替える
    pub fn toggle_display_filter(&mut self) {
        self.display_filter.toggle();
        self.invalidate_tree_flat_cache();
        // 選択インデックスを調整
        let max = self.get_tree_flat_len();
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

    /// 選択されている Tree ノードがファイルかどうかを返す
    pub fn is_selected_tree_node_file(&mut self) -> bool {
        let path = self.get_tree_flat_path(self.tree_selected_index);
        path.and_then(|p| {
            // パスでノードを探してファイルかどうかを判定
            find_node_by_path(&self.tree_root, &p)
        })
        .is_some_and(|node| node.kind == NodeKind::File)
    }

    /// 選択されている Tree ノードのパスを取得する
    pub fn get_selected_tree_node_path(&mut self) -> Option<PathBuf> {
        self.get_tree_flat_path(self.tree_selected_index)
    }

    /// Tree View で選択されているファイルのログを開く
    ///
    /// # Errors
    /// - ファイルが選択されていない場合
    /// - Git コマンドの実行に失敗した場合
    pub fn open_file_log(&mut self) -> Result<()> {
        // ファイルでない場合は何もしない
        if !self.is_selected_tree_node_file() {
            return Ok(());
        }

        // パスを取得
        let path = self.get_selected_tree_node_path();
        if let Some(path) = path {
            // リポジトリルートからの相対パスを計算
            let repo_root = self.git.repo_root().to_path_buf();
            let relative_path = path.strip_prefix(&repo_root).unwrap_or(&path);
            self.refresh_file_log(relative_path)?;
            self.current_view = View::Log;
        }
        Ok(())
    }

    // --- Log View 用メソッド ---

    /// 選択されているコミットのハッシュを取得する
    pub fn selected_commit_hash(&self) -> Option<&str> {
        if self.is_file_log_mode() {
            self.file_log_cache
                .get(self.file_log_selected_index)
                .and_then(|line| line.hash.as_deref())
        } else {
            self.log_cache
                .get(self.log_selected_index)
                .and_then(|line| line.hash.as_deref())
        }
    }

    /// 選択されているコミットの詳細を取得する（色付き）
    ///
    /// # Errors
    /// - Git コマンドの実行に失敗した場合
    pub fn get_commit_details(&self) -> Result<String> {
        if let Some(hash) = self.selected_commit_hash() {
            // ファイルログモードの場合はそのファイルの変更のみ表示
            if let Some(ref path) = self.file_log_path {
                let path_str = path.to_string_lossy();
                self.git
                    .execute(&["show", "--color=always", hash, "--", &path_str])
            } else {
                self.git.execute(&["show", "--color=always", hash])
            }
        } else {
            Ok(String::new())
        }
    }

    /// チェックアウトの確認ダイアログを表示する
    pub fn show_checkout_confirm(&mut self) {
        if let Some(hash) = self.selected_commit_hash() {
            self.confirm_dialog = ConfirmDialog::Checkout {
                commit_hash: hash.to_string(),
            };
        }
    }

    /// チェックアウトを実行する
    ///
    /// # Errors
    /// - Git コマンドの実行に失敗した場合
    pub fn checkout_commit(&mut self) -> Result<()> {
        if let ConfirmDialog::Checkout { ref commit_hash } = self.confirm_dialog {
            let hash = commit_hash.clone();
            self.git.execute(&["checkout", &hash])?;
            self.confirm_dialog = ConfirmDialog::None;
            // チェックアウト後にログとステータスを更新
            self.refresh_log()?;
            self.refresh_status()?;
        }
        Ok(())
    }

    // --- Tree View 検索用メソッド ---

    /// 検索モードを開始する
    pub fn start_tree_search(&mut self) {
        self.tree_search_mode = true;
        self.tree_search_query.clear();
        self.tree_search_matches.clear();
        self.tree_search_current_match = 0;
    }

    /// 検索をキャンセルする
    pub fn cancel_tree_search(&mut self) {
        self.tree_search_mode = false;
        self.tree_search_query.clear();
        self.tree_search_matches.clear();
        self.tree_search_current_match = 0;
    }

    /// 検索を確定し、最初のマッチへジャンプする
    pub fn confirm_tree_search(&mut self) {
        self.tree_search_mode = false;
        self.jump_to_current_match();
    }

    /// 検索文字列に文字を追加する
    pub fn add_tree_search_char(&mut self, c: char) {
        self.tree_search_query.push(c);
        self.update_tree_search_matches();
    }

    /// 検索文字列の末尾を削除する
    pub fn remove_tree_search_char(&mut self) {
        self.tree_search_query.pop();
        self.update_tree_search_matches();
    }

    /// 次のマッチへ移動する
    pub fn next_tree_search_match(&mut self) {
        if !self.tree_search_matches.is_empty() {
            self.tree_search_current_match =
                (self.tree_search_current_match + 1) % self.tree_search_matches.len();
            self.jump_to_current_match();
        }
    }

    /// 前のマッチへ移動する
    pub fn prev_tree_search_match(&mut self) {
        if !self.tree_search_matches.is_empty() {
            if self.tree_search_current_match == 0 {
                self.tree_search_current_match = self.tree_search_matches.len() - 1;
            } else {
                self.tree_search_current_match -= 1;
            }
            self.jump_to_current_match();
        }
    }

    /// 現在のマッチへジャンプする（親を展開してからインデックスを設定）
    fn jump_to_current_match(&mut self) {
        if self.tree_search_matches.is_empty() {
            return;
        }

        let target_path = self.tree_search_matches[self.tree_search_current_match].clone();

        // 親ディレクトリを展開する（キャッシュは expand 内で無効化される）
        self.expand_parents_for_path(&target_path);

        // キャッシュを再構築してインデックスを取得
        self.ensure_tree_flat_cache();
        for (index, (path, _)) in self.tree_flat_cache.iter().enumerate() {
            if *path == target_path {
                self.tree_selected_index = index;
                break;
            }
        }
    }

    /// 指定されたパスの親ディレクトリを全て展開する
    fn expand_parents_for_path(&mut self, target_path: &std::path::Path) {
        // ルートからターゲットまでのパスを収集
        let mut ancestors: Vec<std::path::PathBuf> = Vec::new();
        let mut current = target_path.parent();
        while let Some(parent) = current {
            // ルートパス自体は除外し、ルートパスの子孫のみを追加
            if parent != self.tree_root.path && parent.starts_with(&self.tree_root.path) {
                ancestors.push(parent.to_path_buf());
            }
            current = parent.parent();
        }

        // ルートに近い順に展開
        ancestors.reverse();

        // HashMap を1回だけ構築（borrow checker 対策も兼ねる）
        let status_map = build_status_map(&self.status_cache);

        let mut changed = false;
        for ancestor_path in ancestors {
            if let Some(node) = find_node_by_path_mut(&mut self.tree_root, &ancestor_path) {
                if node.kind == NodeKind::Directory && !node.expanded {
                    if node.children.is_none() {
                        let _ = node.load_children();
                        node.apply_status_map(&status_map);
                    }
                    node.expanded = true;
                    changed = true;
                }
            }
        }
        if changed {
            self.invalidate_tree_flat_cache();
        }
    }

    /// 検索マッチリストを更新する（ロード済みノードのみ検索）
    fn update_tree_search_matches(&mut self) {
        self.tree_search_matches.clear();
        self.tree_search_current_match = 0;

        if self.tree_search_query.is_empty() {
            return;
        }

        let query_lower = self.tree_search_query.to_lowercase();

        // ロード済みのノードのみ検索（未ロードディレクトリは自動ロードしない）
        self.search_tree_recursive(&query_lower);

        // 最初のマッチへジャンプ（検索モード中）
        if !self.tree_search_matches.is_empty() {
            self.jump_to_current_match();
        }
    }

    /// ツリーを再帰的に検索してマッチを収集する
    ///
    /// ロード済みのノードのみ検索する。未ロードのディレクトリは自動ロードしない。
    /// 大規模リポジトリでの検索性能を確保するため。
    fn search_tree_recursive(&mut self, query: &str) {
        fn collect_matches(
            node: &TreeNode,
            query: &str,
            filter: &DisplayFilter,
            matches: &mut Vec<std::path::PathBuf>,
        ) {
            if let Some(children) = &node.children {
                for child in children {
                    // フィルタで非表示のものはスキップ
                    if filter.should_hide(&child.path) {
                        continue;
                    }

                    // 名前がクエリにマッチするか確認
                    let name_lower = child.name.to_lowercase();
                    if name_lower.contains(query) {
                        matches.push(child.path.clone());
                    }

                    // ディレクトリの場合はロード済みの子のみ再帰
                    if child.kind == NodeKind::Directory && child.children.is_some() {
                        collect_matches(child, query, filter, matches);
                    }
                }
            }
        }

        let filter = self.display_filter.clone();
        let mut matches = Vec::new();
        collect_matches(&self.tree_root, query, &filter, &mut matches);
        self.tree_search_matches = matches;
    }

    /// Status キャッシュをツリーに適用する
    ///
    /// HashMap を1回だけ構築し、全ノードに対して再利用する
    fn apply_status_to_tree(&mut self) {
        let status_map = build_status_map(&self.status_cache);

        // ルートノードにステータスを適用
        self.tree_root.apply_status_map(&status_map);

        // 展開されている子ノードにも再帰的に適用
        fn apply_recursive(node: &mut TreeNode, status_map: &crate::domain::StatusMap) {
            if let Some(children) = &mut node.children {
                for child in children {
                    child.apply_status_map(status_map);
                    if child.expanded {
                        apply_recursive(child, status_map);
                    }
                }
            }
        }
        apply_recursive(&mut self.tree_root, &status_map);
    }

    /// 選択されているツリーノードへの可変参照を取得する
    fn get_selected_tree_node_mut(&mut self) -> Option<&mut TreeNode> {
        self.ensure_tree_flat_cache();
        let target_path = self
            .tree_flat_cache
            .get(self.tree_selected_index)
            .map(|(p, _)| p.clone());

        if let Some(path) = target_path {
            find_node_by_path_mut(&mut self.tree_root, &path)
        } else {
            None
        }
    }

    // ==================== コミット関連メソッド ====================

    /// ステージされたファイルの一覧を取得
    pub fn get_staged_files(&self) -> Vec<&FileStatus> {
        self.status_cache
            .iter()
            .filter(|f| f.index != StatusKind::Unmodified && f.index != StatusKind::Untracked)
            .collect()
    }

    /// コミットモードを開始（通常コミット）
    pub fn start_commit_mode(&mut self) {
        let staged = self.get_staged_files();
        if staged.is_empty() {
            self.set_status_message("No staged changes to commit");
            return;
        }
        self.commit_mode = CommitMode::Normal;
        self.commit_message = vec![String::new()];
        self.commit_cursor_x = 0;
        self.commit_cursor_y = 0;
        self.commit_file_index = 0;
        self.commit_focus_files = false;
    }

    /// コミットモードを開始（Amend）
    pub fn start_amend_mode(&mut self) -> Result<()> {
        // 直前のコミットメッセージを取得
        let output = self.git.execute(&["log", "-1", "--pretty=%B"])?;
        let lines: Vec<String> = output.lines().map(|s| s.to_string()).collect();

        self.commit_mode = CommitMode::Amend;
        self.commit_message = if lines.is_empty() {
            vec![String::new()]
        } else {
            // 末尾の空行を削除
            let mut trimmed = lines;
            while trimmed.last().is_some_and(|s| s.is_empty()) {
                trimmed.pop();
            }
            if trimmed.is_empty() {
                vec![String::new()]
            } else {
                trimmed
            }
        };
        // カーソルを末尾に配置
        self.commit_cursor_y = self.commit_message.len().saturating_sub(1);
        self.commit_cursor_x = self.commit_message.last().map_or(0, |s| s.chars().count());
        self.commit_file_index = 0;
        self.commit_focus_files = false;
        Ok(())
    }

    /// コミットモードをキャンセル
    pub fn cancel_commit_mode(&mut self) {
        self.commit_mode = CommitMode::None;
        self.commit_message = vec![String::new()];
        self.commit_cursor_x = 0;
        self.commit_cursor_y = 0;
    }

    /// コミットを実行
    pub fn execute_commit(&mut self) -> Result<()> {
        let message = self.commit_message.join("\n");
        if message.trim().is_empty() {
            self.set_status_message("Commit message cannot be empty");
            return Ok(());
        }

        let result = match self.commit_mode {
            CommitMode::Normal => self.git.execute(&["commit", "-m", &message]),
            CommitMode::Amend => self.git.execute(&["commit", "--amend", "-m", &message]),
            CommitMode::None => return Ok(()),
        };

        match result {
            Ok(_) => {
                let mode_str = if self.commit_mode == CommitMode::Amend {
                    "Amended"
                } else {
                    "Committed"
                };
                self.set_status_message(&format!("{} successfully", mode_str));
                self.cancel_commit_mode();
                self.refresh_status()?;
            }
            Err(e) => {
                self.set_status_message(&format!("Commit failed: {}", e));
            }
        }
        Ok(())
    }

    /// コミットメッセージに文字を追加
    pub fn commit_insert_char(&mut self, c: char) {
        if self.commit_cursor_y < self.commit_message.len() {
            let line = &mut self.commit_message[self.commit_cursor_y];
            let char_indices: Vec<(usize, char)> = line.char_indices().collect();
            let byte_pos = if self.commit_cursor_x >= char_indices.len() {
                line.len()
            } else {
                char_indices[self.commit_cursor_x].0
            };
            line.insert(byte_pos, c);
            self.commit_cursor_x += 1;
        }
    }

    /// コミットメッセージから文字を削除（Backspace）
    pub fn commit_delete_char(&mut self) {
        if self.commit_cursor_x > 0 {
            let line = &mut self.commit_message[self.commit_cursor_y];
            let char_indices: Vec<(usize, char)> = line.char_indices().collect();
            if self.commit_cursor_x <= char_indices.len() {
                let byte_pos = char_indices[self.commit_cursor_x - 1].0;
                line.remove(byte_pos);
                self.commit_cursor_x -= 1;
            }
        } else if self.commit_cursor_y > 0 {
            // 行頭で Backspace: 前の行と結合
            let current_line = self.commit_message.remove(self.commit_cursor_y);
            self.commit_cursor_y -= 1;
            self.commit_cursor_x = self.commit_message[self.commit_cursor_y].chars().count();
            self.commit_message[self.commit_cursor_y].push_str(&current_line);
        }
    }

    /// コミットメッセージで改行
    pub fn commit_new_line(&mut self) {
        let line = &mut self.commit_message[self.commit_cursor_y];
        let char_indices: Vec<(usize, char)> = line.char_indices().collect();
        let byte_pos = if self.commit_cursor_x >= char_indices.len() {
            line.len()
        } else {
            char_indices[self.commit_cursor_x].0
        };
        let rest = line[byte_pos..].to_string();
        line.truncate(byte_pos);
        self.commit_cursor_y += 1;
        self.commit_message.insert(self.commit_cursor_y, rest);
        self.commit_cursor_x = 0;
    }

    /// コミットカーソルを左に移動
    pub fn commit_cursor_left(&mut self) {
        if self.commit_cursor_x > 0 {
            self.commit_cursor_x -= 1;
        } else if self.commit_cursor_y > 0 {
            self.commit_cursor_y -= 1;
            self.commit_cursor_x = self.commit_message[self.commit_cursor_y].chars().count();
        }
    }

    /// コミットカーソルを右に移動
    pub fn commit_cursor_right(&mut self) {
        let line_len = self.commit_message[self.commit_cursor_y].chars().count();
        if self.commit_cursor_x < line_len {
            self.commit_cursor_x += 1;
        } else if self.commit_cursor_y < self.commit_message.len() - 1 {
            self.commit_cursor_y += 1;
            self.commit_cursor_x = 0;
        }
    }

    /// コミットカーソルを上に移動
    pub fn commit_cursor_up(&mut self) {
        if self.commit_cursor_y > 0 {
            self.commit_cursor_y -= 1;
            let line_len = self.commit_message[self.commit_cursor_y].chars().count();
            self.commit_cursor_x = self.commit_cursor_x.min(line_len);
        }
    }

    /// コミットカーソルを下に移動
    pub fn commit_cursor_down(&mut self) {
        if self.commit_cursor_y < self.commit_message.len() - 1 {
            self.commit_cursor_y += 1;
            let line_len = self.commit_message[self.commit_cursor_y].chars().count();
            self.commit_cursor_x = self.commit_cursor_x.min(line_len);
        }
    }

    /// コミット画面でのファイル選択を下に移動
    pub fn commit_file_next(&mut self) {
        let staged_count = self.get_staged_files().len();
        if staged_count > 0 {
            self.commit_file_index = (self.commit_file_index + 1).min(staged_count - 1);
        }
    }

    /// コミット画面でのファイル選択を上に移動
    pub fn commit_file_prev(&mut self) {
        if self.commit_file_index > 0 {
            self.commit_file_index -= 1;
        }
    }

    /// コミット画面でフォーカスを切り替え
    pub fn commit_toggle_focus(&mut self) {
        self.commit_focus_files = !self.commit_focus_files;
    }

    /// コミット画面で選択されたファイルの staged diff を取得（色付き）
    pub fn get_commit_staged_diff(&self) -> Result<String> {
        let staged = self.get_staged_files();
        if let Some(file) = staged.get(self.commit_file_index) {
            let path_str = file.path.to_string_lossy();
            self.git
                .execute(&["diff", "--staged", "--color=always", &path_str])
        } else {
            Ok(String::new())
        }
    }

    // --- Stash 関連メソッド ---

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

    // --- Branch 関連メソッド ---

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
        if let Some(branch) = self.selected_branch() {
            // 現在のブランチの場合は何もしない
            if branch.is_current {
                self.set_status_message("Already on this branch");
                return;
            }
            self.confirm_dialog = ConfirmDialog::CheckoutBranch {
                branch_name: branch.name.clone(),
            };
        }
    }

    /// ブランチをチェックアウトする
    ///
    /// # Errors
    /// - Git コマンドの実行に失敗した場合
    pub fn checkout_branch(&mut self) -> Result<()> {
        if let ConfirmDialog::CheckoutBranch { ref branch_name } = self.confirm_dialog {
            let name = branch_name.clone();

            // 未コミットの変更があるかチェック
            if !self.status_cache.is_empty() {
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
        }
        Ok(())
    }

    // --- Hunk モード関連メソッド ---

    /// Hunk モードを開始する（選択中のファイルの diff をパースして表示）
    ///
    /// # Errors
    /// - Git コマンドの実行に失敗した場合
    pub fn start_hunk_mode(&mut self) -> Result<()> {
        if let Some(file) = self.status_cache.get(self.selected_index).cloned() {
            let path_str = file.path.to_string_lossy().to_string();

            // ステージ済みか否かで diff の取得モードを決定
            let is_cached =
                file.index != StatusKind::Unmodified && file.index != StatusKind::Untracked;

            // 色なしの diff を取得（パース用）
            let diff_output = if is_cached {
                self.git.execute(&["diff", "--cached", &path_str])?
            } else {
                self.git.execute(&["diff", &path_str])?
            };

            if diff_output.is_empty() {
                self.set_status_message("No diff available for this file");
                return Ok(());
            }

            let file_diffs = parse_diff_hunks(&diff_output);
            if file_diffs.is_empty() {
                self.set_status_message("No hunks found in diff");
                return Ok(());
            }

            self.hunk_file_diffs = file_diffs;
            // parse_diff_hunks は hunk が空の FileDiff を生成しないため、
            // インデックス 1 は必ず最初の hunk ヘッダ（0 はファイルヘッダ）
            self.hunk_selected_index = 1;
            self.hunk_target_path = Some(file.path.clone());
            self.hunk_is_cached = is_cached;
            self.hunk_mode = true;
        }
        Ok(())
    }

    /// Hunk モードを終了する
    pub fn cancel_hunk_mode(&mut self) {
        self.hunk_mode = false;
        self.hunk_file_diffs.clear();
        self.hunk_selected_index = 0;
        self.hunk_target_path = None;
        self.hunk_is_cached = false;
    }

    /// Hunk モードでの選択を下に移動（hunk ヘッダのみに移動）
    pub fn hunk_select_next(&mut self) {
        let hunk_indices = self.get_hunk_header_indices();
        if let Some(current_pos) = hunk_indices
            .iter()
            .position(|&i| i == self.hunk_selected_index)
        {
            if current_pos + 1 < hunk_indices.len() {
                self.hunk_selected_index = hunk_indices[current_pos + 1];
            }
        } else {
            // 現在位置が hunk ヘッダでない場合、次の hunk ヘッダに移動
            if let Some(&next) = hunk_indices.iter().find(|&&i| i > self.hunk_selected_index) {
                self.hunk_selected_index = next;
            }
        }
    }

    /// Hunk モードでの選択を上に移動（hunk ヘッダのみに移動）
    pub fn hunk_select_previous(&mut self) {
        let hunk_indices = self.get_hunk_header_indices();
        if let Some(current_pos) = hunk_indices
            .iter()
            .position(|&i| i == self.hunk_selected_index)
        {
            if current_pos > 0 {
                self.hunk_selected_index = hunk_indices[current_pos - 1];
            }
        } else {
            // 現在位置が hunk ヘッダでない場合、前の hunk ヘッダに移動
            if let Some(&prev) = hunk_indices
                .iter()
                .rev()
                .find(|&&i| i < self.hunk_selected_index)
            {
                self.hunk_selected_index = prev;
            }
        }
    }

    /// 選択されている hunk をステージする
    ///
    /// `git apply --cached` を使用してパッチを適用する。
    ///
    /// # Errors
    /// - Git コマンドの実行に失敗した場合
    pub fn stage_selected_hunk(&mut self) -> Result<()> {
        // 選択位置から対応する hunk を特定
        if let Some((file_path, hunk)) = self.get_selected_hunk() {
            let patch = generate_patch(&file_path, &hunk);

            // git apply --cached で hunk をステージ
            match self.git.execute_with_stdin(&["apply", "--cached"], &patch) {
                Ok(_) => {
                    self.set_status_message("Hunk staged successfully");
                    // ステータスを更新して hunk を再読み込み
                    self.refresh_status()?;
                    // diff を再取得して表示を更新（元の diff モードを維持）
                    let path = self.hunk_target_path.clone();
                    if let Some(ref path) = path {
                        let path_str = path.to_string_lossy().to_string();
                        let diff_output = if self.hunk_is_cached {
                            self.git.execute(&["diff", "--cached", &path_str])?
                        } else {
                            self.git.execute(&["diff", &path_str])?
                        };
                        if diff_output.is_empty() {
                            // もう diff がない場合は hunk モードを終了
                            self.cancel_hunk_mode();
                            return Ok(());
                        }
                        self.hunk_file_diffs = parse_diff_hunks(&diff_output);
                        if self.hunk_file_diffs.is_empty() {
                            self.cancel_hunk_mode();
                            return Ok(());
                        }
                        // 選択インデックスを調整
                        let hunk_indices = self.get_hunk_header_indices();
                        if !hunk_indices.is_empty() {
                            self.hunk_selected_index = hunk_indices[0.min(hunk_indices.len() - 1)];
                        }
                    }
                }
                Err(e) => {
                    self.set_status_message(&format!("Failed to stage hunk: {}", e));
                }
            }
        }
        Ok(())
    }

    /// フラット化されたリストの中で hunk ヘッダ行のインデックスを返す
    fn get_hunk_header_indices(&self) -> Vec<usize> {
        let mut indices = Vec::new();
        let mut flat_index = 0;

        for file_diff in &self.hunk_file_diffs {
            // ファイルヘッダ行
            flat_index += 1;

            for hunk in &file_diff.hunks {
                // hunk ヘッダ行
                indices.push(flat_index);
                flat_index += 1;

                // hunk の中身行
                flat_index += hunk.lines.len();
            }
        }

        indices
    }

    /// 選択されている hunk を取得する（ファイルパスとクローン）
    fn get_selected_hunk(&self) -> Option<(String, crate::domain::Hunk)> {
        let mut flat_index = 0;

        for file_diff in &self.hunk_file_diffs {
            flat_index += 1; // ファイルヘッダ

            for hunk in &file_diff.hunks {
                if flat_index == self.hunk_selected_index {
                    return Some((file_diff.file_path.clone(), hunk.clone()));
                }
                flat_index += 1; // hunk ヘッダ
                flat_index += hunk.lines.len(); // hunk 行数
            }
        }

        None
    }

    // --- リモート操作メソッド ---

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

/// パスでノードを探して不変参照を返す
///
/// パスの前方一致を使って探索を枝刈りし、ターゲットが含まれ得ないサブツリーをスキップする。
fn find_node_by_path<'a>(node: &'a TreeNode, path: &std::path::Path) -> Option<&'a TreeNode> {
    if node.path == path {
        return Some(node);
    }

    if let Some(children) = &node.children {
        for child in children {
            if child.path == path {
                return Some(child);
            }
            if child.kind == NodeKind::Directory && path.starts_with(&child.path) {
                if let Some(found) = find_node_by_path(child, path) {
                    return Some(found);
                }
            }
        }
    }

    None
}

/// パスでノードを探して可変参照を返す
///
/// パスの前方一致を使って探索を枝刈りし、ターゲットが含まれ得ないサブツリーをスキップする。
fn find_node_by_path_mut<'a>(
    node: &'a mut TreeNode,
    path: &std::path::Path,
) -> Option<&'a mut TreeNode> {
    if node.path == path {
        return Some(node);
    }

    if let Some(children) = &mut node.children {
        for child in children {
            // ターゲットパスが子のパスで始まるか、子のパスがターゲットと一致する場合のみ探索
            // ファイルノードは path が一致する場合のみ、ディレクトリノードは path が
            // ターゲットの祖先である場合のみ再帰する
            if child.path == path {
                return Some(child);
            }
            if child.kind == NodeKind::Directory && path.starts_with(&child.path) {
                if let Some(found) = find_node_by_path_mut(child, path) {
                    return Some(found);
                }
            }
        }
    }

    None
}
