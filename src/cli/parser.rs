use std::path::PathBuf;

use crate::domain::{FileStatus, GraphLine, StatusKind};
use crate::error::{AppError, Result};

/// git status --porcelain=v1 の出力をパースする
///
/// # Errors
/// - 行のフォーマットが不正な場合
pub fn parse_status(output: &str) -> Result<Vec<FileStatus>> {
    output
        .lines()
        .filter(|line| !line.is_empty())
        .map(parse_status_line)
        .collect()
}

/// 1行の status 出力をパースする
fn parse_status_line(line: &str) -> Result<FileStatus> {
    // porcelain=v1 フォーマット: XY PATH または XY ORIG -> PATH
    // 最低でも "XY " + 1文字のパスが必要
    if line.len() < 4 {
        return Err(AppError::Parse(format!("Invalid status line: {}", line)));
    }

    let chars: Vec<char> = line.chars().collect();
    let index = parse_status_char(chars[0]);
    let worktree = parse_status_char(chars[1]);

    let path_part = &line[3..];
    let (path, original_path) = parse_path_part(path_part);

    Ok(FileStatus {
        index,
        worktree,
        path,
        original_path,
    })
}

/// パス部分をパースする（リネーム対応、引用符対応）
fn parse_path_part(path_part: &str) -> (PathBuf, Option<PathBuf>) {
    // リネームの場合: "old" -> "new" または old -> new
    // " -> " で分割するが、引用符内の " -> " は無視する必要がある
    if let Some((orig, dest)) = split_rename_path(path_part) {
        (
            PathBuf::from(unquote_path(dest)),
            Some(PathBuf::from(unquote_path(orig))),
        )
    } else {
        (PathBuf::from(unquote_path(path_part)), None)
    }
}

/// リネームパスを分割する
/// 引用符で囲まれたパスを考慮して " -> " で分割
fn split_rename_path(path_part: &str) -> Option<(&str, &str)> {
    // 引用符で始まる場合、閉じ引用符の後の " -> " を探す
    if path_part.starts_with('"') {
        // 最初の閉じ引用符を探す（エスケープされた引用符を考慮）
        if let Some(end_quote) = find_closing_quote(path_part) {
            let after_quote = &path_part[end_quote + 1..];
            if let Some(arrow_pos) = after_quote.find(" -> ") {
                let orig = &path_part[..end_quote + 1];
                let dest = &after_quote[arrow_pos + 4..];
                return Some((orig, dest));
            }
        }
        None
    } else {
        // 引用符なしの場合、単純に " -> " で分割
        path_part.find(" -> ").map(|pos| {
            let orig = &path_part[..pos];
            let dest = &path_part[pos + 4..];
            (orig, dest)
        })
    }
}

/// 引用符で囲まれたパスの閉じ引用符の位置を探す
fn find_closing_quote(s: &str) -> Option<usize> {
    let chars: Vec<char> = s.chars().collect();
    let mut i = 1; // 最初の引用符をスキップ
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            // エスケープシーケンスをスキップ
            i += 2;
        } else if chars[i] == '"' {
            // バイト位置を計算
            return Some(chars[..=i].iter().collect::<String>().len() - 1);
        } else {
            i += 1;
        }
    }
    None
}

/// 引用符で囲まれたパスをアンクォートする
/// Git の C スタイルエスケープを処理
fn unquote_path(path: &str) -> String {
    let path = path.trim();
    if !path.starts_with('"') || !path.ends_with('"') {
        return path.to_string();
    }

    // 引用符を除去
    let inner = &path[1..path.len() - 1];

    // C スタイルエスケープを処理
    let mut result = String::new();
    let chars: Vec<char> = inner.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            match chars[i + 1] {
                'n' => result.push('\n'),
                't' => result.push('\t'),
                'r' => result.push('\r'),
                '\\' => result.push('\\'),
                '"' => result.push('"'),
                // 8進数エスケープ（\nnn）
                c if c.is_ascii_digit() => {
                    let mut octal = String::new();
                    let mut j = i + 1;
                    while j < chars.len() && j < i + 4 && chars[j].is_ascii_digit() {
                        octal.push(chars[j]);
                        j += 1;
                    }
                    if let Ok(code) = u8::from_str_radix(&octal, 8) {
                        result.push(code as char);
                    }
                    i = j - 1;
                }
                other => {
                    result.push('\\');
                    result.push(other);
                }
            }
            i += 2;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    result
}

/// ステータス文字を StatusKind に変換する
fn parse_status_char(c: char) -> StatusKind {
    match c {
        'M' => StatusKind::Modified,
        'A' => StatusKind::Added,
        'D' => StatusKind::Deleted,
        'R' => StatusKind::Renamed,
        'C' => StatusKind::Copied,
        '?' => StatusKind::Untracked,
        '!' => StatusKind::Ignored,
        _ => StatusKind::Unmodified,
    }
}

// --- Log パーサー ---

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
    fn test_parse_status_modified_in_worktree_returns_modified_kind() {
        // Arrange
        let input = " M src/main.rs";

        // Act
        let result = parse_status_line(input).unwrap();

        // Assert
        assert_eq!(result.index, StatusKind::Unmodified);
        assert_eq!(result.worktree, StatusKind::Modified);
        assert_eq!(result.path, PathBuf::from("src/main.rs"));
        assert!(result.original_path.is_none());
    }

    #[test]
    fn test_parse_status_added_in_index_returns_added_kind() {
        // Arrange
        let input = "A  new_file.rs";

        // Act
        let result = parse_status_line(input).unwrap();

        // Assert
        assert_eq!(result.index, StatusKind::Added);
        assert_eq!(result.worktree, StatusKind::Unmodified);
        assert_eq!(result.path, PathBuf::from("new_file.rs"));
    }

    #[test]
    fn test_parse_status_deleted_in_worktree_returns_deleted_kind() {
        // Arrange
        let input = " D deleted.rs";

        // Act
        let result = parse_status_line(input).unwrap();

        // Assert
        assert_eq!(result.index, StatusKind::Unmodified);
        assert_eq!(result.worktree, StatusKind::Deleted);
        assert_eq!(result.path, PathBuf::from("deleted.rs"));
    }

    #[test]
    fn test_parse_status_renamed_in_index_returns_renamed_kind_with_original_path() {
        // Arrange
        let input = "R  old.rs -> new.rs";

        // Act
        let result = parse_status_line(input).unwrap();

        // Assert
        assert_eq!(result.index, StatusKind::Renamed);
        assert_eq!(result.worktree, StatusKind::Unmodified);
        assert_eq!(result.path, PathBuf::from("new.rs"));
        assert_eq!(result.original_path, Some(PathBuf::from("old.rs")));
    }

    #[test]
    fn test_parse_status_untracked_returns_untracked_kind() {
        // Arrange
        let input = "?? untracked.txt";

        // Act
        let result = parse_status_line(input).unwrap();

        // Assert
        assert_eq!(result.index, StatusKind::Untracked);
        assert_eq!(result.worktree, StatusKind::Untracked);
        assert_eq!(result.path, PathBuf::from("untracked.txt"));
    }

    #[test]
    fn test_parse_status_multiple_lines_returns_all_statuses() {
        // Arrange
        let input = " M src/main.rs\nA  new.rs\n?? untracked.txt";

        // Act
        let result = parse_status(input).unwrap();

        // Assert
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].worktree, StatusKind::Modified);
        assert_eq!(result[1].index, StatusKind::Added);
        assert_eq!(result[2].index, StatusKind::Untracked);
    }

    #[test]
    fn test_parse_status_empty_input_returns_empty_vec() {
        // Arrange
        let input = "";

        // Act
        let result = parse_status(input).unwrap();

        // Assert
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_status_line_too_short_returns_error() {
        // Arrange
        let input = "M ";

        // Act
        let result = parse_status_line(input);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_status_quoted_path_with_space_returns_unquoted_path() {
        // Arrange: Git quotes paths with spaces
        let input = " M \"my file.txt\"";

        // Act
        let result = parse_status_line(input).unwrap();

        // Assert
        assert_eq!(result.worktree, StatusKind::Modified);
        assert_eq!(result.path, PathBuf::from("my file.txt"));
    }

    #[test]
    fn test_parse_status_quoted_rename_returns_unquoted_paths() {
        // Arrange: Renamed file with quoted paths
        let input = "R  \"old name.rs\" -> \"new name.rs\"";

        // Act
        let result = parse_status_line(input).unwrap();

        // Assert
        assert_eq!(result.index, StatusKind::Renamed);
        assert_eq!(result.path, PathBuf::from("new name.rs"));
        assert_eq!(result.original_path, Some(PathBuf::from("old name.rs")));
    }

    #[test]
    fn test_unquote_path_with_escaped_characters() {
        // Arrange: Path with escaped characters
        let input = "\"file\\twith\\ttabs.txt\"";

        // Act
        let result = unquote_path(input);

        // Assert
        assert_eq!(result, "file\twith\ttabs.txt");
    }

    #[test]
    fn test_unquote_path_without_quotes_returns_as_is() {
        // Arrange: Path without quotes
        let input = "simple.txt";

        // Act
        let result = unquote_path(input);

        // Assert
        assert_eq!(result, "simple.txt");
    }

    // --- Log パーサーのテスト ---

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
