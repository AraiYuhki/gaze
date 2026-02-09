use crate::domain::{FileDiff, Hunk, HunkLine};

/// git diff の出力（色なし）をパースして FileDiff を返す
///
/// ANSI カラーコードは含まれていないことを前提とする。
/// パースに失敗した場合でも、できる限りの情報を返す。
pub fn parse_diff_hunks(output: &str) -> Vec<FileDiff> {
    let mut results = Vec::new();
    let mut current_file: Option<String> = None;
    let mut current_hunks: Vec<Hunk> = Vec::new();
    let mut current_hunk: Option<Hunk> = None;

    for line in output.lines() {
        // 新しいファイルの diff ヘッダ
        if line.starts_with("diff --git ") {
            // 現在の hunk を保存
            if let Some(hunk) = current_hunk.take() {
                current_hunks.push(hunk);
            }
            // 現在のファイルを保存
            if let Some(file_path) = current_file.take() {
                if !current_hunks.is_empty() {
                    results.push(FileDiff {
                        file_path,
                        hunks: std::mem::take(&mut current_hunks),
                    });
                }
            }
            // "diff --git a/path b/path" からパスを抽出
            current_file = extract_diff_file_path(line);
            continue;
        }

        // --- a/... や +++ b/... は読み飛ばす
        if line.starts_with("--- ") || line.starts_with("+++ ") {
            continue;
        }

        // index 行やモード行は読み飛ばす
        if line.starts_with("index ")
            || line.starts_with("old mode")
            || line.starts_with("new mode")
            || line.starts_with("new file")
            || line.starts_with("deleted file")
            || line.starts_with("similarity")
            || line.starts_with("rename ")
            || line.starts_with("copy ")
        {
            continue;
        }

        // hunk ヘッダ
        if line.starts_with("@@ ") {
            // 現在の hunk を保存
            if let Some(hunk) = current_hunk.take() {
                current_hunks.push(hunk);
            }
            current_hunk = parse_hunk_header(line);
            continue;
        }

        // hunk の中身
        if let Some(ref mut hunk) = current_hunk {
            if let Some(stripped) = line.strip_prefix('+') {
                hunk.lines.push(HunkLine::Added(stripped.to_string()));
            } else if let Some(stripped) = line.strip_prefix('-') {
                hunk.lines.push(HunkLine::Removed(stripped.to_string()));
            } else if let Some(stripped) = line.strip_prefix(' ') {
                hunk.lines.push(HunkLine::Context(stripped.to_string()));
            } else if line == "\\ No newline at end of file" {
                // git の "No newline at end of file" メッセージは無視
            } else {
                // 先頭にプレフィックスがない行はコンテキスト行として扱う
                hunk.lines.push(HunkLine::Context(line.to_string()));
            }
        }
    }

    // 最後の hunk とファイルを保存
    if let Some(hunk) = current_hunk {
        current_hunks.push(hunk);
    }
    if let Some(file_path) = current_file {
        if !current_hunks.is_empty() {
            results.push(FileDiff {
                file_path,
                hunks: current_hunks,
            });
        }
    }

    results
}

/// "diff --git a/path b/path" からファイルパスを抽出する
fn extract_diff_file_path(line: &str) -> Option<String> {
    // "diff --git a/path b/path" の形式
    let rest = line.strip_prefix("diff --git ")?;
    // 末尾から " b/" を探すことで、パス内に " b/" が含まれるケースにも対応
    if let Some(pos) = rest.rfind(" b/") {
        Some(rest[pos + 3..].to_string())
    } else {
        // a/path のみの場合
        rest.strip_prefix("a/").map(|s| s.to_string())
    }
}

/// hunk ヘッダ行をパースして Hunk を生成する
///
/// 形式: "@@ -old_start,old_count +new_start,new_count @@ optional context"
fn parse_hunk_header(line: &str) -> Option<Hunk> {
    let header = line.to_string();

    // "@@ -" の後から解析
    let rest = line.strip_prefix("@@ -")?;

    // 次の " @@" を探す
    let end_marker = rest.find(" @@")?;
    let range_part = &rest[..end_marker];

    // "-old_start,old_count +new_start,new_count" をパース
    let parts: Vec<&str> = range_part.split(' ').collect();
    if parts.len() < 2 {
        return None;
    }

    let (old_start, old_count) = parse_range(parts[0])?;
    let new_part = parts[1].strip_prefix('+')?;
    let (new_start, new_count) = parse_range(new_part)?;

    Some(Hunk {
        header,
        old_start,
        old_count,
        new_start,
        new_count,
        lines: Vec::new(),
    })
}

/// "start,count" または "start" をパースする
fn parse_range(s: &str) -> Option<(usize, usize)> {
    if let Some((start_str, count_str)) = s.split_once(',') {
        let start = start_str.parse().ok()?;
        let count = count_str.parse().ok()?;
        Some((start, count))
    } else {
        let start = s.parse().ok()?;
        Some((start, 1))
    }
}

/// 選択した hunk をパッチファイル形式で生成する
///
/// `git apply --cached` に渡すためのパッチ文字列を生成する。
pub fn generate_patch(file_path: &str, hunk: &Hunk) -> String {
    let mut patch = String::new();
    patch.push_str(&format!("--- a/{}\n", file_path));
    patch.push_str(&format!("+++ b/{}\n", file_path));
    patch.push_str(&hunk.header);
    patch.push('\n');
    for line in &hunk.lines {
        match line {
            HunkLine::Context(s) => {
                patch.push(' ');
                patch.push_str(s);
                patch.push('\n');
            }
            HunkLine::Added(s) => {
                patch.push('+');
                patch.push_str(s);
                patch.push('\n');
            }
            HunkLine::Removed(s) => {
                patch.push('-');
                patch.push_str(s);
                patch.push('\n');
            }
        }
    }
    patch
}

/// 選択した行のみを含む部分パッチを生成する
///
/// hunk 内の指定行インデックスのみをステージするパッチを作る。
/// - Context 行: 常に含む
/// - 選択された Added 行: `+` として含む
/// - 非選択の Added 行: パッチから除外
/// - 選択された Removed 行: `-` として含む
/// - 非選択の Removed 行: Context 行に変換（行はまだ存在するため）
/// - hunk ヘッダの行数を再計算する
pub fn generate_partial_patch(
    file_path: &str,
    hunk: &Hunk,
    selected_line_indices: &[usize],
) -> String {
    let mut patch_lines: Vec<String> = Vec::new();
    let mut old_count: usize = 0;
    let mut new_count: usize = 0;

    for (i, line) in hunk.lines.iter().enumerate() {
        let is_selected = selected_line_indices.contains(&i);
        match line {
            HunkLine::Context(s) => {
                patch_lines.push(format!(" {}", s));
                old_count += 1;
                new_count += 1;
            }
            HunkLine::Added(s) => {
                if is_selected {
                    patch_lines.push(format!("+{}", s));
                    new_count += 1;
                }
                // 非選択の Added 行はパッチから除外
            }
            HunkLine::Removed(s) => {
                if is_selected {
                    patch_lines.push(format!("-{}", s));
                    old_count += 1;
                } else {
                    // 非選択の Removed 行は Context に変換（行はまだ存在する）
                    patch_lines.push(format!(" {}", s));
                    old_count += 1;
                    new_count += 1;
                }
            }
        }
    }

    let mut patch = String::new();
    patch.push_str(&format!("--- a/{}\n", file_path));
    patch.push_str(&format!("+++ b/{}\n", file_path));
    patch.push_str(&format!(
        "@@ -{},{} +{},{} @@\n",
        hunk.old_start, old_count, hunk.new_start, new_count
    ));
    for pl in &patch_lines {
        patch.push_str(pl);
        patch.push('\n');
    }
    patch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_diff_hunks_single_file_single_hunk() {
        // Arrange
        let input = "diff --git a/src/main.rs b/src/main.rs\n\
                      index abc1234..def5678 100644\n\
                      --- a/src/main.rs\n\
                      +++ b/src/main.rs\n\
                      @@ -1,3 +1,4 @@\n\
                       line1\n\
                      -line2\n\
                      +line2_modified\n\
                      +line2b\n\
                       line3\n";

        // Act
        let result = parse_diff_hunks(input);

        // Assert
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].file_path, "src/main.rs");
        assert_eq!(result[0].hunks.len(), 1);

        let hunk = &result[0].hunks[0];
        assert_eq!(hunk.old_start, 1);
        assert_eq!(hunk.old_count, 3);
        assert_eq!(hunk.new_start, 1);
        assert_eq!(hunk.new_count, 4);
        assert_eq!(hunk.lines.len(), 5);
        assert_eq!(hunk.lines[0], HunkLine::Context("line1".to_string()));
        assert_eq!(hunk.lines[1], HunkLine::Removed("line2".to_string()));
        assert_eq!(hunk.lines[2], HunkLine::Added("line2_modified".to_string()));
        assert_eq!(hunk.lines[3], HunkLine::Added("line2b".to_string()));
        assert_eq!(hunk.lines[4], HunkLine::Context("line3".to_string()));
    }

    #[test]
    fn test_parse_diff_hunks_multiple_hunks() {
        // Arrange
        let input = "diff --git a/file.rs b/file.rs\n\
                      --- a/file.rs\n\
                      +++ b/file.rs\n\
                      @@ -1,2 +1,3 @@\n\
                       line1\n\
                      +added\n\
                       line2\n\
                      @@ -10,2 +11,2 @@\n\
                       line10\n\
                      -line11\n\
                      +line11_mod\n";

        // Act
        let result = parse_diff_hunks(input);

        // Assert
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].hunks.len(), 2);

        let hunk1 = &result[0].hunks[0];
        assert_eq!(hunk1.old_start, 1);
        assert_eq!(hunk1.lines.len(), 3);

        let hunk2 = &result[0].hunks[1];
        assert_eq!(hunk2.old_start, 10);
        assert_eq!(hunk2.lines.len(), 3);
    }

    #[test]
    fn test_parse_diff_hunks_empty_input_returns_empty() {
        // Arrange & Act
        let result = parse_diff_hunks("");

        // Assert
        assert!(result.is_empty());
    }

    #[test]
    fn test_generate_patch_produces_valid_patch() {
        // Arrange
        let hunk = Hunk {
            header: "@@ -1,3 +1,4 @@".to_string(),
            old_start: 1,
            old_count: 3,
            new_start: 1,
            new_count: 4,
            lines: vec![
                HunkLine::Context("line1".to_string()),
                HunkLine::Removed("line2".to_string()),
                HunkLine::Added("line2_mod".to_string()),
                HunkLine::Added("line2b".to_string()),
                HunkLine::Context("line3".to_string()),
            ],
        };

        // Act
        let patch = generate_patch("src/main.rs", &hunk);

        // Assert
        assert!(patch.starts_with("--- a/src/main.rs\n"));
        assert!(patch.contains("+++ b/src/main.rs\n"));
        assert!(patch.contains("@@ -1,3 +1,4 @@\n"));
        assert!(patch.contains(" line1\n"));
        assert!(patch.contains("-line2\n"));
        assert!(patch.contains("+line2_mod\n"));
        assert!(patch.contains("+line2b\n"));
        assert!(patch.contains(" line3\n"));
    }

    #[test]
    fn test_parse_hunk_header_without_count() {
        // Arrange: count が省略されている場合は 1 とみなす
        let input = "@@ -5 +5 @@ fn test()";

        // Act
        let result = parse_hunk_header(input);

        // Assert
        assert!(result.is_some());
        let hunk = result.unwrap();
        assert_eq!(hunk.old_start, 5);
        assert_eq!(hunk.old_count, 1);
        assert_eq!(hunk.new_start, 5);
        assert_eq!(hunk.new_count, 1);
    }

    // --- generate_partial_patch テスト ---

    #[test]
    fn test_generate_partial_patch_selected_added_lines_only() {
        // Arrange: Added 行のみ選択
        let hunk = Hunk {
            header: "@@ -1,3 +1,5 @@".to_string(),
            old_start: 1,
            old_count: 3,
            new_start: 1,
            new_count: 5,
            lines: vec![
                HunkLine::Context("line1".to_string()),
                HunkLine::Added("new1".to_string()),
                HunkLine::Added("new2".to_string()),
                HunkLine::Context("line2".to_string()),
                HunkLine::Context("line3".to_string()),
            ],
        };

        // Act: index 1 のみ選択（"new1"）
        let patch = generate_partial_patch("file.rs", &hunk, &[1]);

        // Assert: 選択した Added 行のみ含む、非選択 Added は除外
        assert!(patch.contains("@@ -1,3 +1,4 @@\n"));
        assert!(patch.contains("+new1\n"));
        assert!(!patch.contains("+new2\n"));
        assert!(patch.contains(" line1\n"));
        assert!(patch.contains(" line2\n"));
        assert!(patch.contains(" line3\n"));
    }

    #[test]
    fn test_generate_partial_patch_selected_removed_lines_only() {
        // Arrange: Removed 行のみ選択
        let hunk = Hunk {
            header: "@@ -1,4 +1,2 @@".to_string(),
            old_start: 1,
            old_count: 4,
            new_start: 1,
            new_count: 2,
            lines: vec![
                HunkLine::Context("line1".to_string()),
                HunkLine::Removed("old1".to_string()),
                HunkLine::Removed("old2".to_string()),
                HunkLine::Context("line2".to_string()),
            ],
        };

        // Act: index 1 のみ選択（"old1" を削除としてステージ）
        let patch = generate_partial_patch("file.rs", &hunk, &[1]);

        // Assert: 選択 Removed は - のまま、非選択 Removed は Context に変換
        assert!(patch.contains("@@ -1,4 +1,3 @@\n"));
        assert!(patch.contains("-old1\n"));
        assert!(patch.contains(" old2\n")); // 非選択 Removed → Context
        assert!(patch.contains(" line1\n"));
        assert!(patch.contains(" line2\n"));
    }

    #[test]
    fn test_generate_partial_patch_mixed_selection() {
        // Arrange: Added と Removed が混在
        let hunk = Hunk {
            header: "@@ -1,3 +1,4 @@".to_string(),
            old_start: 1,
            old_count: 3,
            new_start: 1,
            new_count: 4,
            lines: vec![
                HunkLine::Context("line1".to_string()),
                HunkLine::Removed("old_line".to_string()),
                HunkLine::Added("new_line1".to_string()),
                HunkLine::Added("new_line2".to_string()),
                HunkLine::Context("line3".to_string()),
            ],
        };

        // Act: Removed(1) と Added(2) のみ選択
        let patch = generate_partial_patch("file.rs", &hunk, &[1, 2]);

        // Assert: old=3(context1 + removed + context3), new=3(context1 + added + context3)
        assert!(patch.contains("@@ -1,3 +1,3 @@\n"));
        assert!(patch.contains("-old_line\n"));
        assert!(patch.contains("+new_line1\n"));
        assert!(!patch.contains("+new_line2\n")); // 非選択 Added は除外
    }

    #[test]
    fn test_generate_partial_patch_empty_selection_converts_removed_to_context() {
        // Arrange: 何も選択しない場合
        let hunk = Hunk {
            header: "@@ -1,3 +1,4 @@".to_string(),
            old_start: 1,
            old_count: 3,
            new_start: 1,
            new_count: 4,
            lines: vec![
                HunkLine::Context("line1".to_string()),
                HunkLine::Removed("old_line".to_string()),
                HunkLine::Added("new_line".to_string()),
                HunkLine::Context("line3".to_string()),
            ],
        };

        // Act: 空の選択
        let patch = generate_partial_patch("file.rs", &hunk, &[]);

        // Assert: Removed は Context に、Added は除外。old=new=3
        assert!(patch.contains("@@ -1,3 +1,3 @@\n"));
        assert!(!patch.contains("-old_line\n"));
        assert!(!patch.contains("+new_line\n"));
        assert!(patch.contains(" old_line\n")); // Removed → Context
    }
}
