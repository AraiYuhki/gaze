use crate::cli::{generate_partial_patch, generate_patch, parse_diff_hunks};
use crate::domain::{HunkLine, StatusKind};
use crate::error::Result;

use super::AppState;

impl AppState {
    /// Hunk モードを開始する（選択中のファイルの diff をパースして表示）
    ///
    /// # Errors
    /// - Git コマンドの実行に失敗した場合
    pub fn start_hunk_mode(&mut self) -> Result<()> {
        if let Some(file) = self.status_cache.get(self.selected_index).cloned() {
            let path_str = file.path.to_string_lossy().to_string();

            // ステージ済みか否かで diff の取得モードを決定
            let is_cached =
                file.index != StatusKind::Unmodified && file.index != StatusKind::Untracked;

            // 色なしの diff を取得（パース用）
            let diff_output = if is_cached {
                self.git.execute(&["diff", "--cached", &path_str])?
            } else {
                self.git.execute(&["diff", &path_str])?
            };

            if diff_output.is_empty() {
                self.set_status_message("No diff available for this file");
                return Ok(());
            }

            let file_diffs = parse_diff_hunks(&diff_output);
            if file_diffs.is_empty() {
                self.set_status_message("No hunks found in diff");
                return Ok(());
            }

            self.hunk_file_diffs = file_diffs;
            // parse_diff_hunks は hunk が空の FileDiff を生成しないため、
            // インデックス 1 は必ず最初の hunk ヘッダ（0 はファイルヘッダ）
            self.hunk_selected_index = 1;
            self.hunk_target_path = Some(file.path.clone());
            self.hunk_is_cached = is_cached;
            self.hunk_mode = true;
        }
        Ok(())
    }

    /// Hunk モードを終了する
    pub fn cancel_hunk_mode(&mut self) {
        self.hunk_mode = false;
        self.hunk_file_diffs.clear();
        self.hunk_selected_index = 0;
        self.hunk_target_path = None;
        self.hunk_is_cached = false;
        self.hunk_visual_mode = false;
        self.hunk_visual_anchor = 0;
    }

    /// Hunk モードでの選択を下に1行移動（ファイルヘッダはスキップ）
    pub fn hunk_select_next(&mut self) {
        let total = self.get_hunk_total_lines();
        let mut next = self.hunk_selected_index + 1;
        // ファイルヘッダ行をスキップ
        while next < total && self.is_file_header_index(next) {
            next += 1;
        }
        if next < total {
            self.hunk_selected_index = next;
        }
    }

    /// Hunk モードでの選択を上に1行移動（ファイルヘッダはスキップ）
    pub fn hunk_select_previous(&mut self) {
        if self.hunk_selected_index == 0 {
            return;
        }
        let mut prev = self.hunk_selected_index - 1;
        // ファイルヘッダ行をスキップ
        while prev > 0 && self.is_file_header_index(prev) {
            prev -= 1;
        }
        // prev==0 がファイルヘッダの場合、移動しない
        if !self.is_file_header_index(prev) {
            self.hunk_selected_index = prev;
        }
    }

    /// パッチを適用し、diff を再取得して表示を更新する
    ///
    /// # Errors
    /// - Git コマンドの実行に失敗した場合
    fn apply_hunk_patch(&mut self, patch: &str, message: &str) -> Result<()> {
        match self.git.execute_with_stdin(&["apply", "--cached"], patch) {
            Ok(_) => {
                self.set_status_message(message);
                self.refresh_hunk_diff_after_patch()?;
            }
            Err(e) => {
                self.set_status_message(&format!("Failed to stage: {}", e));
            }
        }
        Ok(())
    }

    /// パッチ適用後に diff を再取得して hunk モードの表示を更新する
    fn refresh_hunk_diff_after_patch(&mut self) -> Result<()> {
        let path = match self.hunk_target_path.clone() {
            Some(p) => p,
            None => return Ok(()),
        };
        let path_str = path.to_string_lossy().to_string();

        let diff_output = if self.hunk_is_cached {
            self.git.execute(&["diff", "--cached", &path_str])?
        } else {
            self.git.execute(&["diff", &path_str])?
        };

        self.update_status_after_hunk_stage(&path_str, diff_output.is_empty());

        if diff_output.is_empty() {
            self.cancel_hunk_mode();
            return Ok(());
        }

        self.hunk_file_diffs = parse_diff_hunks(&diff_output);
        if self.hunk_file_diffs.is_empty() {
            self.cancel_hunk_mode();
            return Ok(());
        }

        // 選択インデックスを最初の hunk ヘッダに調整
        let hunk_indices = self.get_hunk_header_indices();
        if !hunk_indices.is_empty() {
            self.hunk_selected_index = hunk_indices[0];
        }
        Ok(())
    }

    /// hunk ステージ後にステータスキャッシュを更新する
    fn update_status_after_hunk_stage(&mut self, path_str: &str, diff_empty: bool) {
        if let Some(entry) = self
            .status_cache
            .iter_mut()
            .find(|f| f.path.to_string_lossy() == path_str)
        {
            entry.index = StatusKind::Modified;
            if diff_empty {
                entry.worktree = StatusKind::Unmodified;
            }
        }
        self.tree_flat_dirty = true;
        self.bg_status_receiver = None;
    }

    /// 選択されている hunk / 行をステージする
    ///
    /// カーソル位置に応じて動作が変わる:
    /// - hunk ヘッダ上: hunk 全体をステージ
    /// - コンテンツ行上: その1行だけをステージ
    ///
    /// # Errors
    /// - Git コマンドの実行に失敗した場合
    pub fn stage_selected_hunk(&mut self) -> Result<()> {
        // Visual モードの場合は別メソッドで処理
        if self.hunk_visual_mode {
            return self.stage_visual_selection();
        }

        let resolved = self.resolve_flat_index(self.hunk_selected_index);
        match resolved {
            // hunk ヘッダ上 → hunk 全体をステージ
            Some((file_idx, hunk_idx, None)) => {
                let file_path = self.hunk_file_diffs[file_idx].file_path.clone();
                let hunk = self.hunk_file_diffs[file_idx].hunks[hunk_idx].clone();
                let patch = generate_patch(&file_path, &hunk);
                self.apply_hunk_patch(&patch, "Hunk staged successfully")?;
            }
            // コンテンツ行上 → 1行だけをステージ
            Some((file_idx, hunk_idx, Some(line_idx))) => {
                let line = &self.hunk_file_diffs[file_idx].hunks[hunk_idx].lines[line_idx];
                // Context 行はステージ不可
                if matches!(line, HunkLine::Context(_)) {
                    self.set_status_message("Context line cannot be staged");
                    return Ok(());
                }
                let file_path = self.hunk_file_diffs[file_idx].file_path.clone();
                let hunk = self.hunk_file_diffs[file_idx].hunks[hunk_idx].clone();
                let patch = generate_partial_patch(&file_path, &hunk, &[line_idx]);
                self.apply_hunk_patch(&patch, "Line staged successfully")?;
            }
            // ファイルヘッダ上 → 何もしない
            None => {}
        }
        Ok(())
    }

    /// Visual モードで選択された範囲をステージする
    ///
    /// # Errors
    /// - Git コマンドの実行に失敗した場合
    fn stage_visual_selection(&mut self) -> Result<()> {
        let (range_min, range_max) = match self.hunk_visual_range() {
            Some(range) => range,
            None => return Ok(()),
        };

        // 範囲内のコンテンツ行を解決
        let mut target_file_idx: Option<usize> = None;
        let mut target_hunk_idx: Option<usize> = None;
        let mut selected_line_indices: Vec<usize> = Vec::new();
        let mut has_change_lines = false;

        for flat_idx in range_min..=range_max {
            if let Some((file_idx, hunk_idx, Some(line_idx))) = self.resolve_flat_index(flat_idx) {
                // 最初のコンテンツ行から file/hunk を特定
                if target_file_idx.is_none() {
                    target_file_idx = Some(file_idx);
                    target_hunk_idx = Some(hunk_idx);
                }
                // 別 hunk に跨がるかチェック
                if target_file_idx != Some(file_idx) || target_hunk_idx != Some(hunk_idx) {
                    self.hunk_visual_mode = false;
                    self.set_status_message("Cannot stage across different hunks");
                    return Ok(());
                }
                let line = &self.hunk_file_diffs[file_idx].hunks[hunk_idx].lines[line_idx];
                if !matches!(line, HunkLine::Context(_)) {
                    has_change_lines = true;
                }
                selected_line_indices.push(line_idx);
            }
            // hunk ヘッダやファイルヘッダはスキップ
        }

        if !has_change_lines {
            self.hunk_visual_mode = false;
            self.set_status_message("No changes in selection (context lines only)");
            return Ok(());
        }

        if let (Some(fi), Some(hi)) = (target_file_idx, target_hunk_idx) {
            let file_path = self.hunk_file_diffs[fi].file_path.clone();
            let hunk = self.hunk_file_diffs[fi].hunks[hi].clone();
            // Context 行はインデックスに含まれていても generate_partial_patch が正しく処理する
            let patch = generate_partial_patch(&file_path, &hunk, &selected_line_indices);
            self.hunk_visual_mode = false;
            self.apply_hunk_patch(&patch, "Selection staged successfully")?;
        }
        Ok(())
    }

    /// フラット化されたリストの中で hunk ヘッダ行のインデックスを返す
    fn get_hunk_header_indices(&self) -> Vec<usize> {
        let mut indices = Vec::new();
        let mut flat_index = 0;

        for file_diff in &self.hunk_file_diffs {
            // ファイルヘッダ行
            flat_index += 1;

            for hunk in &file_diff.hunks {
                // hunk ヘッダ行
                indices.push(flat_index);
                flat_index += 1;

                // hunk の中身行
                flat_index += hunk.lines.len();
            }
        }

        indices
    }

    /// フラットリストの総行数を返す
    fn get_hunk_total_lines(&self) -> usize {
        let mut total = 0;
        for file_diff in &self.hunk_file_diffs {
            total += 1; // ファイルヘッダ
            for hunk in &file_diff.hunks {
                total += 1; // hunk ヘッダ
                total += hunk.lines.len();
            }
        }
        total
    }

    /// 指定インデックスがファイルヘッダ行かどうかを判定する
    fn is_file_header_index(&self, index: usize) -> bool {
        let mut flat_index = 0;
        for file_diff in &self.hunk_file_diffs {
            if flat_index == index {
                return true;
            }
            flat_index += 1; // ファイルヘッダ
            for hunk in &file_diff.hunks {
                flat_index += 1; // hunk ヘッダ
                flat_index += hunk.lines.len();
            }
        }
        false
    }

    /// フラットインデックスを構造的位置に解決する
    ///
    /// 戻り値: `Some((file_idx, hunk_idx, Option<line_idx>))`
    /// - ファイルヘッダ → `None`
    /// - hunk ヘッダ → `Some((file_idx, hunk_idx, None))`
    /// - コンテンツ行 → `Some((file_idx, hunk_idx, Some(line_idx)))`
    fn resolve_flat_index(&self, target: usize) -> Option<(usize, usize, Option<usize>)> {
        let mut flat_index = 0;
        for (file_idx, file_diff) in self.hunk_file_diffs.iter().enumerate() {
            if flat_index == target {
                return None; // ファイルヘッダ
            }
            flat_index += 1;

            for (hunk_idx, hunk) in file_diff.hunks.iter().enumerate() {
                if flat_index == target {
                    return Some((file_idx, hunk_idx, None)); // hunk ヘッダ
                }
                flat_index += 1;

                for line_idx in 0..hunk.lines.len() {
                    if flat_index == target {
                        return Some((file_idx, hunk_idx, Some(line_idx)));
                    }
                    flat_index += 1;
                }
            }
        }
        None
    }

    /// Visual モードの選択範囲を返す（ソート済みの min, max）
    pub fn hunk_visual_range(&self) -> Option<(usize, usize)> {
        if !self.hunk_visual_mode {
            return None;
        }
        let min = self.hunk_visual_anchor.min(self.hunk_selected_index);
        let max = self.hunk_visual_anchor.max(self.hunk_selected_index);
        Some((min, max))
    }
}
