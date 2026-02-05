/// ブランチエントリを表す構造体
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchEntry {
    /// ブランチ名
    pub name: String,
    /// 現在のブランチかどうか
    pub is_current: bool,
    /// リモートブランチかどうか
    pub is_remote: bool,
    /// 生の行（パース失敗時のフォールバック用）
    pub raw_line: String,
}

impl BranchEntry {
    /// 生の行から BranchEntry を作成（パース失敗時用）
    #[allow(dead_code)] // TODO(Phase 8): パース失敗時のフォールバックとして将来使用予定
    pub fn from_raw(raw_line: String) -> Self {
        Self {
            name: raw_line.clone(),
            is_current: false,
            is_remote: false,
            raw_line,
        }
    }
}
