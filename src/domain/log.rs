/// Git log の1行を表す構造体
///
/// グラフ構造（親子関係）の解析は禁止。
/// パース失敗時は raw_line をそのまま表示に使用する。
#[derive(Debug, Clone)]
pub struct GraphLine {
    /// 元の行（フォールバック用）
    pub raw_line: String,
    /// グラフ部分（|, *, /, \ など）
    pub graph_chars: String,
    /// コミットハッシュ（7文字）
    pub hash: Option<String>,
    /// refs（ブランチ名、タグ）
    pub refs: Vec<String>,
    /// コミットメッセージ
    pub message: Option<String>,
}

impl GraphLine {
    /// 新しい GraphLine を作成する（パース失敗時用）
    #[allow(dead_code)] // パース失敗時のフォールバック用
    pub fn from_raw(line: &str) -> Self {
        Self {
            raw_line: line.to_string(),
            graph_chars: String::new(),
            hash: None,
            refs: Vec::new(),
            message: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_raw_creates_graphline_with_raw_line() {
        // Arrange
        let line = "* abc1234 (HEAD -> main) Initial commit";

        // Act
        let result = GraphLine::from_raw(line);

        // Assert
        assert_eq!(result.raw_line, line);
        assert!(result.hash.is_none());
        assert!(result.refs.is_empty());
        assert!(result.message.is_none());
    }
}
