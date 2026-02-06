use std::path::PathBuf;

use crate::domain::{
    BranchEntry, FileDiff, FileStatus, GraphLine, Hunk, HunkLine, StashEntry, StatusKind,
};
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

// --- Stash パーサー ---

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

// --- Branch パーサー ---

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

// ==================== Hunk パーサー ====================

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

    // --- Stash パーサーのテスト ---

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

    // --- Branch パーサーのテスト ---

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

    // ==================== Hunk パーサーテスト ====================

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
