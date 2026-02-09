use crate::domain::StashEntry;

/// git stash list の出力をパースする
///
/// # フォーマット例
/// - `stash@{0}: WIP on main: abc1234 commit message`
/// - `stash@{1}: On feature: def5678 another message`
pub fn parse_stash_list(output: &str) -> Vec<StashEntry> {
    output.lines().map(parse_stash_line).collect()
}

/// 1行の stash list 出力をパースする
fn parse_stash_line(line: &str) -> StashEntry {
    let raw_line = line.to_string();

    // stash@{N}: を探す
    let Some(colon_pos) = line.find(": ") else {
        return StashEntry::from_raw(raw_line);
    };

    let stash_ref = &line[..colon_pos];
    let rest = &line[colon_pos + 2..];

    // stash@{N} からインデックスを抽出
    let index = extract_stash_index(stash_ref).unwrap_or(0);

    // "WIP on <branch>:" または "On <branch>:" を探す
    let (branch, commit_info) = if let Some(after_wip) = rest.strip_prefix("WIP on ") {
        extract_branch_and_rest(after_wip)
    } else if let Some(after_on) = rest.strip_prefix("On ") {
        extract_branch_and_rest(after_on)
    } else {
        (String::new(), rest.to_string())
    };

    // コミットハッシュとメッセージを抽出
    let (commit_hash, message) = extract_commit_info(&commit_info);

    StashEntry {
        index,
        branch,
        commit_hash,
        message,
        raw_line,
    }
}

/// stash@{N} からインデックスを抽出
fn extract_stash_index(stash_ref: &str) -> Option<usize> {
    let start = stash_ref.find('{')?;
    let end = stash_ref.find('}')?;
    stash_ref[start + 1..end].parse().ok()
}

/// ブランチ名とその後の部分を分割
fn extract_branch_and_rest(s: &str) -> (String, String) {
    // ブランチ名の後の ": " を探す
    if let Some(colon_pos) = s.find(": ") {
        let branch = s[..colon_pos].to_string();
        let rest = s[colon_pos + 2..].to_string();
        (branch, rest)
    } else {
        (String::new(), s.to_string())
    }
}

/// コミット情報（ハッシュとメッセージ）を抽出
fn extract_commit_info(s: &str) -> (String, String) {
    // 最初の空白で分割（ハッシュ メッセージ）
    if let Some(space_pos) = s.find(' ') {
        let hash = s[..space_pos].to_string();
        let message = s[space_pos + 1..].to_string();
        (hash, message)
    } else {
        // スペースがない場合は全体をメッセージとして扱う
        (String::new(), s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_stash_line_wip_format() {
        // Arrange
        let input = "stash@{0}: WIP on main: abc1234 commit message";

        // Act
        let result = parse_stash_line(input);

        // Assert
        assert_eq!(result.index, 0);
        assert_eq!(result.branch, "main");
        assert_eq!(result.commit_hash, "abc1234");
        assert_eq!(result.message, "commit message");
    }

    #[test]
    fn test_parse_stash_line_on_format() {
        // Arrange
        let input = "stash@{1}: On feature/test: def5678 another message";

        // Act
        let result = parse_stash_line(input);

        // Assert
        assert_eq!(result.index, 1);
        assert_eq!(result.branch, "feature/test");
        assert_eq!(result.commit_hash, "def5678");
        assert_eq!(result.message, "another message");
    }

    #[test]
    fn test_parse_stash_list_multiple_entries() {
        // Arrange
        let input = "stash@{0}: WIP on main: abc1234 First\nstash@{1}: On dev: def5678 Second";

        // Act
        let result = parse_stash_list(input);

        // Assert
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].index, 0);
        assert_eq!(result[1].index, 1);
    }

    #[test]
    fn test_parse_stash_list_empty_returns_empty_vec() {
        // Arrange
        let input = "";

        // Act
        let result = parse_stash_list(input);

        // Assert
        assert!(result.is_empty());
    }
}
