use crate::domain::BranchEntry;

/// git branch -a の出力をパースする
///
/// # フォーマット例
/// - `* main` (現在のブランチ)
/// - `  feature/foo`
/// - `  remotes/origin/main`
/// - `  remotes/origin/HEAD -> origin/main`
pub fn parse_branch_list(output: &str) -> Vec<BranchEntry> {
    output
        .lines()
        .filter(|line| !line.is_empty())
        .filter(|line| !line.contains(" -> ")) // HEAD -> origin/main のような行をスキップ
        .map(parse_branch_line)
        .collect()
}

/// 1行の branch list 出力をパースする
fn parse_branch_line(line: &str) -> BranchEntry {
    let raw_line = line.to_string();

    // 先頭2文字をチェック（"* " または "  "）
    let is_current = line.starts_with("* ");

    // 先頭2文字をスキップしてブランチ名を取得
    let name_part = if line.len() > 2 { &line[2..] } else { line };
    let name = name_part.trim().to_string();

    // リモートブランチかどうかを判定
    let is_remote = name.starts_with("remotes/");

    // リモートブランチの場合は "remotes/" プレフィックスを除去して表示用の名前にする
    let display_name = if is_remote {
        name.strip_prefix("remotes/").unwrap_or(&name).to_string()
    } else {
        name
    };

    BranchEntry {
        name: display_name,
        is_current,
        is_remote,
        raw_line,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_branch_line_current_branch() {
        // Arrange
        let input = "* main";

        // Act
        let result = parse_branch_line(input);

        // Assert
        assert_eq!(result.name, "main");
        assert!(result.is_current);
        assert!(!result.is_remote);
    }

    #[test]
    fn test_parse_branch_line_local_branch() {
        // Arrange
        let input = "  feature/foo";

        // Act
        let result = parse_branch_line(input);

        // Assert
        assert_eq!(result.name, "feature/foo");
        assert!(!result.is_current);
        assert!(!result.is_remote);
    }

    #[test]
    fn test_parse_branch_line_remote_branch() {
        // Arrange
        let input = "  remotes/origin/main";

        // Act
        let result = parse_branch_line(input);

        // Assert
        assert_eq!(result.name, "origin/main");
        assert!(!result.is_current);
        assert!(result.is_remote);
    }

    #[test]
    fn test_parse_branch_list_multiple_branches() {
        // Arrange
        let input = "* main\n  feature/foo\n  remotes/origin/main";

        // Act
        let result = parse_branch_list(input);

        // Assert
        assert_eq!(result.len(), 3);
        assert!(result[0].is_current);
        assert_eq!(result[0].name, "main");
        assert!(!result[1].is_current);
        assert_eq!(result[1].name, "feature/foo");
        assert!(result[2].is_remote);
        assert_eq!(result[2].name, "origin/main");
    }

    #[test]
    fn test_parse_branch_list_filters_head_reference() {
        // Arrange: HEAD -> origin/main のような行はスキップされる
        let input = "* main\n  remotes/origin/HEAD -> origin/main\n  remotes/origin/main";

        // Act
        let result = parse_branch_list(input);

        // Assert
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "main");
        assert_eq!(result[1].name, "origin/main");
    }

    #[test]
    fn test_parse_branch_list_empty_returns_empty_vec() {
        // Arrange
        let input = "";

        // Act
        let result = parse_branch_list(input);

        // Assert
        assert!(result.is_empty());
    }
}
