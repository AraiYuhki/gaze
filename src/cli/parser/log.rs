use crate::domain::GraphLine;

/// git log --oneline --graph の出力をパースする
///
/// # 重要
/// - グラフ構造（親子関係）の解析は禁止
/// - パース失敗時は raw_line をそのまま返す
pub fn parse_log(output: &str) -> Vec<GraphLine> {
    output.lines().map(parse_log_line).collect()
}

/// 1行の log 出力をパースする
///
/// フォーマット例:
/// - `* abc1234 (HEAD -> main, origin/main) commit message`
/// - `| * def5678 another commit`
/// - `|/`
pub fn parse_log_line(line: &str) -> GraphLine {
    let raw_line = line.to_string();

    // グラフ文字を抽出（行頭から英数字またはスペースが出現するまで）
    let graph_end = find_graph_end(line);
    let graph_chars = line[..graph_end].to_string();
    let rest = line[graph_end..].trim_start();

    // 残りが空なら graph のみの行
    if rest.is_empty() {
        return GraphLine {
            raw_line,
            graph_chars,
            hash: None,
            refs: Vec::new(),
            message: None,
        };
    }

    // ハッシュを抽出（7-40文字の16進数）
    let (hash, rest) = extract_hash(rest);

    // refs を抽出（括弧で囲まれた部分）
    let (refs, rest) = extract_refs(rest);

    // 残りはメッセージ
    let message = if rest.is_empty() {
        None
    } else {
        Some(rest.to_string())
    };

    GraphLine {
        raw_line,
        graph_chars,
        hash,
        refs,
        message,
    }
}

/// グラフ文字の終端位置を探す
fn find_graph_end(line: &str) -> usize {
    let graph_chars = ['*', '|', '/', '\\', '_', ' ', '-'];
    let mut end = 0;

    for c in line.chars() {
        if graph_chars.contains(&c) {
            end += c.len_utf8();
        } else {
            break;
        }
    }
    end
}

/// ハッシュを抽出する
fn extract_hash(s: &str) -> (Option<String>, &str) {
    // 7-40文字の16進数を探す
    let chars: Vec<char> = s.chars().collect();
    let mut hash_len = 0;

    for c in &chars {
        if c.is_ascii_hexdigit() {
            hash_len += 1;
        } else {
            break;
        }
    }

    if (7..=40).contains(&hash_len) {
        let hash = chars[..hash_len].iter().collect();
        let rest = &s[chars[..hash_len].iter().collect::<String>().len()..].trim_start();
        (Some(hash), rest)
    } else {
        (None, s)
    }
}

/// refs を抽出する（括弧で囲まれた部分）
fn extract_refs(s: &str) -> (Vec<String>, &str) {
    if !s.starts_with('(') {
        return (Vec::new(), s);
    }

    if let Some(end) = s.find(')') {
        let refs_str = &s[1..end];
        let refs: Vec<String> = refs_str
            .split(',')
            .map(|r| r.trim().to_string())
            .filter(|r| !r.is_empty())
            .collect();
        let rest = s[end + 1..].trim_start();
        (refs, rest)
    } else {
        (Vec::new(), s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_log_line_with_hash_refs_and_message() {
        // Arrange
        let input = "* abc1234 (HEAD -> main, origin/main) Initial commit";

        // Act
        let result = parse_log_line(input);

        // Assert
        assert_eq!(result.graph_chars, "* ");
        assert_eq!(result.hash, Some("abc1234".to_string()));
        assert_eq!(result.refs, vec!["HEAD -> main", "origin/main"]);
        assert_eq!(result.message, Some("Initial commit".to_string()));
    }

    #[test]
    fn test_parse_log_line_without_refs() {
        // Arrange
        let input = "* def5678 Second commit";

        // Act
        let result = parse_log_line(input);

        // Assert
        assert_eq!(result.graph_chars, "* ");
        assert_eq!(result.hash, Some("def5678".to_string()));
        assert!(result.refs.is_empty());
        assert_eq!(result.message, Some("Second commit".to_string()));
    }

    #[test]
    fn test_parse_log_line_graph_only() {
        // Arrange
        let input = "|/";

        // Act
        let result = parse_log_line(input);

        // Assert
        assert_eq!(result.graph_chars, "|/");
        assert!(result.hash.is_none());
        assert!(result.refs.is_empty());
        assert!(result.message.is_none());
    }

    #[test]
    fn test_parse_log_line_with_branch_graph() {
        // Arrange
        let input = "| * 1234567 Feature commit";

        // Act
        let result = parse_log_line(input);

        // Assert
        assert_eq!(result.graph_chars, "| * ");
        assert_eq!(result.hash, Some("1234567".to_string()));
        assert_eq!(result.message, Some("Feature commit".to_string()));
    }

    #[test]
    fn test_parse_log_empty_returns_empty_vec() {
        // Arrange
        let input = "";

        // Act
        let result = parse_log(input);

        // Assert
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_log_multiple_lines() {
        // Arrange
        let input = "* abc1234 (HEAD) First\n* def5678 Second";

        // Act
        let result = parse_log(input);

        // Assert
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].hash, Some("abc1234".to_string()));
        assert_eq!(result[1].hash, Some("def5678".to_string()));
    }
}
