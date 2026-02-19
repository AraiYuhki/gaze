mod branch_ops;
mod commit_ops;
mod hunk_ops;
mod navigation;
mod remote_ops;
mod stash_ops;
mod status_ops;
mod tree_ops;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::sync::mpsc;

use crate::cli::{parse_log, parse_status, GitCli};
use crate::domain::{
    build_status_map, BranchEntry, FileDiff, FileStatus, GraphLine, StashEntry, TreeNode,
};
use crate::error::Result;
use crate::filter::DisplayFilter;

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
    pub(super) git: GitCli,
    /// リポジトリルートパス
    #[allow(dead_code)] // 将来のリファクタリングや拡張で使用予定
    pub(super) repo_root: PathBuf,
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
    pub(super) tree_flat_cache: Vec<(PathBuf, usize)>,
    /// フラットキャッシュが無効化されているか
    pub(super) tree_flat_dirty: bool,
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
    pub tree_search_matches: Vec<PathBuf>,
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
    pub(super) filtered_branch_indices: Vec<usize>,
    /// Hunk モードかどうか
    pub hunk_mode: bool,
    /// Hunk モードのファイル diff 一覧
    pub hunk_file_diffs: Vec<FileDiff>,
    /// Hunk モードでの選択インデックス（フラット化されたリスト上のインデックス）
    pub hunk_selected_index: usize,
    /// Hunk モードの対象ファイルパス
    pub hunk_target_path: Option<PathBuf>,
    /// Hunk モードが --cached（ステージ済み）の diff を対象にしているか
    pub(super) hunk_is_cached: bool,
    /// Hunk Visual モードかどうか
    pub hunk_visual_mode: bool,
    /// Hunk Visual モードのアンカー位置（選択開始位置）
    pub hunk_visual_anchor: usize,
    /// ファイルログ表示中のファイルパス（None の場合は通常の Log View）
    pub file_log_path: Option<PathBuf>,
    /// ファイルログのキャッシュ
    pub file_log_cache: Vec<GraphLine>,
    /// ファイルログでの選択インデックス
    pub file_log_selected_index: usize,
    /// バックグラウンド status 取得の結果受信チャネル（起動時の二段階読み込み用）
    pub(super) bg_status_receiver: Option<mpsc::Receiver<Vec<FileStatus>>>,
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
            hunk_visual_mode: false,
            hunk_visual_anchor: 0,
            file_log_path: None,
            file_log_cache: Vec::new(),
            file_log_selected_index: 0,
            bg_status_receiver: None,
        };
        // 起動時は tracked ファイルのみ高速に取得（untracked 除外）
        state.refresh_status_tracked_only()?;
        // バックグラウンドで完全な status を取得開始
        state.spawn_background_full_status();
        // Tree のステータスを適用
        let status_map = build_status_map(&state.status_cache);
        state.tree_root.apply_status_map(&status_map);
        Ok(state)
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
        let output =
            self.git
                .execute(&["log", "--oneline", "--graph", "-n", &LOG_LIMIT.to_string()])?;
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
        let hash = match self.selected_commit_hash() {
            Some(h) => h,
            None => return Ok(String::new()),
        };
        // ファイルログモードの場合はそのファイルの変更のみ表示
        if let Some(ref path) = self.file_log_path {
            let path_str = path.to_string_lossy();
            self.git
                .execute(&["show", "--color=always", hash, "--", &path_str])
        } else {
            self.git.execute(&["show", "--color=always", hash])
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

    /// 確認ダイアログをキャンセルする
    pub fn cancel_confirm(&mut self) {
        self.confirm_dialog = ConfirmDialog::None;
    }

    /// ステータスメッセージを設定する
    pub fn set_status_message(&mut self, message: &str) {
        self.status_message = Some(message.to_string());
    }

    /// ステータスメッセージをクリアする
    pub fn clear_status_message(&mut self) {
        self.status_message = None;
    }

    /// バックグラウンドスレッドで完全な `git status` を実行する
    ///
    /// 結果は `mpsc::Receiver` 経由で受信する。
    /// `check_background_status()` でポーリングして結果を取得する。
    fn spawn_background_full_status(&mut self) {
        let repo_root = self.git.repo_root().to_path_buf();
        let (sender, receiver) = mpsc::channel();
        self.bg_status_receiver = Some(receiver);

        std::thread::spawn(move || {
            let output = std::process::Command::new("git")
                .args(["status", "--porcelain=v1", "-uall"])
                .current_dir(&repo_root)
                .output();

            if let Ok(output) = output {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    if let Ok(statuses) = parse_status(&stdout) {
                        // 送信失敗は無視（receiver が破棄済みの場合）
                        let _ = sender.send(statuses);
                    }
                }
            }
        });
    }

    /// バックグラウンド status の結果をノンブロッキングでチェックする
    ///
    /// 結果が届いていれば status_cache を差し替え、Tree のステータスも更新する。
    /// 1回の受信で完了し、receiver を破棄する。
    pub fn check_background_status(&mut self) -> bool {
        use std::sync::mpsc::TryRecvError;

        let recv_result = self.bg_status_receiver.as_ref().map(|rx| rx.try_recv());

        match recv_result {
            Some(Ok(statuses)) => {
                self.status_cache = statuses;
                // 選択インデックスの範囲外チェック
                if self.status_cache.is_empty() {
                    self.selected_index = 0;
                } else if self.selected_index >= self.status_cache.len() {
                    self.selected_index = self.status_cache.len() - 1;
                }
                // Tree にステータスを再適用
                self.apply_status_to_tree();
                self.tree_flat_dirty = true;
                // 1回限りで receiver を破棄
                self.bg_status_receiver = None;
                true
            }
            Some(Err(TryRecvError::Disconnected)) => {
                // 送信側スレッドが異常終了した場合、receiver を破棄して無駄なポーリングを防ぐ
                self.bg_status_receiver = None;
                false
            }
            _ => false,
        }
    }
}
