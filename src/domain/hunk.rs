/// Hunk 内の各行の種別
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HunkLine {
    /// 変更なしの行（スペース始まり）
    Context(String),
    /// 追加行（+ 始まり）
    Added(String),
    /// 削除行（- 始まり）
    Removed(String),
}

/// diff の1つの hunk（変更ブロック）
#[derive(Debug, Clone)]
#[allow(dead_code)] // パース結果の保持用。パッチ生成では header 文字列を使用するため直接参照されない
pub struct Hunk {
    /// ヘッダ行（例: "@@ -1,3 +1,4 @@ fn main()"）
    pub header: String,
    /// 変更前の開始行番号
    pub old_start: usize,
    /// 変更前の行数
    pub old_count: usize,
    /// 変更後の開始行番号
    pub new_start: usize,
    /// 変更後の行数
    pub new_count: usize,
    /// hunk 内の各行
    pub lines: Vec<HunkLine>,
}

/// diff 出力から hunk をパースした結果
#[derive(Debug, Clone)]
pub struct FileDiff {
    /// ファイルパス（"a/..." の部分）
    pub file_path: String,
    /// パースされた hunk のリスト
    pub hunks: Vec<Hunk>,
}
