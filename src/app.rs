use std::path::Path;

use crate::cli::{parse_log, parse_status, GitCli};
use crate::domain::{FileStatus, GraphLine, NodeKind, StatusKind, TreeNode};
use crate::error::Result;
use crate::filter::DisplayFilter;
use crate::ui::tree_view;

/// 現在表示している View の種類
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Status,
    Tree,
    Log,
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
            match view {
                View::Tree => {
                    // Tree View 切り替え時に status を更新してツリーに適用
                    let _ = self.refresh_status();
                    self.apply_status_to_tree();
                }
                View::Log => {
                    // Log View 切り替え時にログを取得
                    let _ = self.refresh_log();
                }
                View::Status => {
                    let _ = self.refresh_status();
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
            View::Log => {
                if !self.log_cache.is_empty() {
                    self.log_selected_index =
                        (self.log_selected_index + 1).min(self.log_cache.len() - 1);
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
                self.log_selected_index = self.log_selected_index.saturating_sub(1);
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
                self.log_selected_index = 0;
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
            View::Log => {
                if !self.log_cache.is_empty() {
                    self.log_selected_index = self.log_cache.len() - 1;
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

    // --- Log View 用メソッド ---

    /// 選択されているコミットのハッシュを取得する
    pub fn selected_commit_hash(&self) -> Option<&str> {
        self.log_cache
            .get(self.log_selected_index)
            .and_then(|line| line.hash.as_deref())
    }

    /// 選択されているコミットの詳細を取得する
    ///
    /// # Errors
    /// - Git コマンドの実行に失敗した場合
    pub fn get_commit_details(&self) -> Result<String> {
        if let Some(hash) = self.selected_commit_hash() {
            self.git.execute(&["show", hash])
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

        // 親ディレクトリを展開する
        self.expand_parents_for_path(&target_path);

        // 展開後にインデックスを取得
        let flat = tree_view::flatten_tree(&self.tree_root, &self.display_filter, 0);
        for (index, (node, _)) in flat.iter().enumerate() {
            if node.path == target_path {
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

        // status_cache を先にクローン（borrow checker 対策）
        let cache = self.status_cache.clone();

        for ancestor_path in ancestors {
            if let Some(node) = find_node_by_path_mut(&mut self.tree_root, &ancestor_path) {
                if node.kind == NodeKind::Directory && !node.expanded {
                    if node.children.is_none() {
                        let _ = node.load_children();
                        node.apply_status_cache(&cache);
                    }
                    node.expanded = true;
                }
            }
        }
    }

    /// 検索マッチリストを更新する（全ノードを検索、折りたたみ状態も含む）
    fn update_tree_search_matches(&mut self) {
        self.tree_search_matches.clear();
        self.tree_search_current_match = 0;

        if self.tree_search_query.is_empty() {
            return;
        }

        let query_lower = self.tree_search_query.to_lowercase();

        // 全ノードを再帰的に検索（折りたたまれたノードも含む）
        // まず未ロードのディレクトリもロードする必要があるため、
        // ツリーを走査しながらマッチを収集
        let cache = self.status_cache.clone();
        self.search_tree_recursive(&query_lower, &cache);

        // 最初のマッチへジャンプ（検索モード中）
        if !self.tree_search_matches.is_empty() {
            self.jump_to_current_match();
        }
    }

    /// ツリーを再帰的に検索してマッチを収集する
    fn search_tree_recursive(&mut self, query: &str, cache: &[FileStatus]) {
        // 検索のためにツリー全体を走査
        fn collect_matches(
            node: &mut TreeNode,
            query: &str,
            filter: &DisplayFilter,
            cache: &[FileStatus],
            matches: &mut Vec<std::path::PathBuf>,
        ) {
            // 未ロードの場合はロード
            if node.kind == NodeKind::Directory && node.children.is_none() {
                let _ = node.load_children();
                node.apply_status_cache(cache);
            }

            if let Some(children) = &mut node.children {
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

                    // ディレクトリの場合は再帰
                    if child.kind == NodeKind::Directory {
                        collect_matches(child, query, filter, cache, matches);
                    }
                }
            }
        }

        let filter = self.display_filter.clone();
        let mut matches = Vec::new();
        collect_matches(&mut self.tree_root, query, &filter, cache, &mut matches);
        self.tree_search_matches = matches;
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
        self.commit_cursor_x = self
            .commit_message
            .last()
            .map_or(0, |s| s.chars().count());
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
        self.commit_message
            .insert(self.commit_cursor_y, rest);
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

    /// コミット画面で選択されたファイルの staged diff を取得
    pub fn get_commit_staged_diff(&self) -> Result<String> {
        let staged = self.get_staged_files();
        if let Some(file) = staged.get(self.commit_file_index) {
            let path_str = file.path.to_string_lossy();
            self.git.execute(&["diff", "--staged", &path_str])
        } else {
            Ok(String::new())
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
