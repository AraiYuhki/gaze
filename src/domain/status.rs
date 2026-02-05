use std::path::PathBuf;

/// ファイルの Git ステータス種別
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Unmodified,
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    Untracked,
    Ignored,
}

/// ファイルの Git ステータス情報
#[derive(Debug, Clone)]
pub struct FileStatus {
    /// インデックス（ステージング領域）のステータス
    pub index: StatusKind,
    /// ワークツリーのステータス
    pub worktree: StatusKind,
    /// ファイルパス
    pub path: PathBuf,
    /// リネーム元のパス（リネーム時のみ）
    // TODO(Phase 1): リネーム表示で使用予定だが、現時点では未使用
    #[allow(dead_code)]
    pub original_path: Option<PathBuf>,
}
