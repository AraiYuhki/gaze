/// Stash エントリを表す構造体
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StashEntry {
    /// stash@{N} の N
    pub index: usize,
    /// ブランチ名
    pub branch: String,
    /// 短縮コミットハッシュ
    pub commit_hash: String,
    /// stash メッセージ
    pub message: String,
    /// 生の行（パース失敗時のフォールバック用）
    pub raw_line: String,
}

impl StashEntry {
    /// 生の行から StashEntry を作成（パース失敗時用）
    pub fn from_raw(raw_line: String) -> Self {
        Self {
            index: 0,
            branch: String::new(),
            commit_hash: String::new(),
            message: raw_line.clone(),
            raw_line,
        }
    }
}
