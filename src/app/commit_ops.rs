use crate::error::Result;

use super::{AppState, CommitMode};

impl AppState {
    /// コミットモードを開始（通常コミット）
    pub fn start_commit_mode(&mut self) {
        let staged = self.get_staged_files();
        if staged.is_empty() {
            self.set_status_message("No staged changes to commit");
            return;
        }
        self.commit_mode = CommitMode::Normal;
        self.commit_message = vec![String::new()];
        self.commit_cursor_x = 0;
        self.commit_cursor_y = 0;
        self.commit_file_index = 0;
        self.commit_focus_files = false;
    }

    /// コミットモードを開始（Amend）
    pub fn start_amend_mode(&mut self) -> Result<()> {
        // 直前のコミットメッセージを取得
        let output = self.git.execute(&["log", "-1", "--pretty=%B"])?;
        let lines: Vec<String> = output.lines().map(|s| s.to_string()).collect();

        self.commit_mode = CommitMode::Amend;
        self.commit_message = if lines.is_empty() {
            vec![String::new()]
        } else {
            // 末尾の空行を削除
            let mut trimmed = lines;
            while trimmed.last().is_some_and(|s| s.is_empty()) {
                trimmed.pop();
            }
            if trimmed.is_empty() {
                vec![String::new()]
            } else {
                trimmed
            }
        };
        // カーソルを末尾に配置
        self.commit_cursor_y = self.commit_message.len().saturating_sub(1);
        self.commit_cursor_x = self.commit_message.last().map_or(0, |s| s.chars().count());
        self.commit_file_index = 0;
        self.commit_focus_files = false;
        Ok(())
    }

    /// コミットモードをキャンセル
    pub fn cancel_commit_mode(&mut self) {
        self.commit_mode = CommitMode::None;
        self.commit_message = vec![String::new()];
        self.commit_cursor_x = 0;
        self.commit_cursor_y = 0;
    }

    /// コミットを実行
    pub fn execute_commit(&mut self) -> Result<()> {
        let message = self.commit_message.join("\n");
        if message.trim().is_empty() {
            self.set_status_message("Commit message cannot be empty");
            return Ok(());
        }

        let result = match self.commit_mode {
            CommitMode::Normal => self.git.execute(&["commit", "-m", &message]),
            CommitMode::Amend => self.git.execute(&["commit", "--amend", "-m", &message]),
            CommitMode::None => return Ok(()),
        };

        match result {
            Ok(_) => {
                let mode_str = if self.commit_mode == CommitMode::Amend {
                    "Amended"
                } else {
                    "Committed"
                };
                self.set_status_message(&format!("{} successfully", mode_str));
                self.cancel_commit_mode();
                self.refresh_status()?;
            }
            Err(e) => {
                self.set_status_message(&format!("Commit failed: {}", e));
            }
        }
        Ok(())
    }

    /// コミットメッセージに文字を追加
    pub fn commit_insert_char(&mut self, c: char) {
        if self.commit_cursor_y < self.commit_message.len() {
            let line = &mut self.commit_message[self.commit_cursor_y];
            let char_indices: Vec<(usize, char)> = line.char_indices().collect();
            let byte_pos = if self.commit_cursor_x >= char_indices.len() {
                line.len()
            } else {
                char_indices[self.commit_cursor_x].0
            };
            line.insert(byte_pos, c);
            self.commit_cursor_x += 1;
        }
    }

    /// コミットメッセージから文字を削除（Backspace）
    pub fn commit_delete_char(&mut self) {
        if self.commit_cursor_x > 0 {
            let line = &mut self.commit_message[self.commit_cursor_y];
            let char_indices: Vec<(usize, char)> = line.char_indices().collect();
            if self.commit_cursor_x <= char_indices.len() {
                let byte_pos = char_indices[self.commit_cursor_x - 1].0;
                line.remove(byte_pos);
                self.commit_cursor_x -= 1;
            }
        } else if self.commit_cursor_y > 0 {
            // 行頭で Backspace: 前の行と結合
            let current_line = self.commit_message.remove(self.commit_cursor_y);
            self.commit_cursor_y -= 1;
            self.commit_cursor_x = self.commit_message[self.commit_cursor_y].chars().count();
            self.commit_message[self.commit_cursor_y].push_str(&current_line);
        }
    }

    /// コミットメッセージで改行
    pub fn commit_new_line(&mut self) {
        let line = &mut self.commit_message[self.commit_cursor_y];
        let char_indices: Vec<(usize, char)> = line.char_indices().collect();
        let byte_pos = if self.commit_cursor_x >= char_indices.len() {
            line.len()
        } else {
            char_indices[self.commit_cursor_x].0
        };
        let rest = line[byte_pos..].to_string();
        line.truncate(byte_pos);
        self.commit_cursor_y += 1;
        self.commit_message.insert(self.commit_cursor_y, rest);
        self.commit_cursor_x = 0;
    }

    /// コミットカーソルを左に移動
    pub fn commit_cursor_left(&mut self) {
        if self.commit_cursor_x > 0 {
            self.commit_cursor_x -= 1;
        } else if self.commit_cursor_y > 0 {
            self.commit_cursor_y -= 1;
            self.commit_cursor_x = self.commit_message[self.commit_cursor_y].chars().count();
        }
    }

    /// コミットカーソルを右に移動
    pub fn commit_cursor_right(&mut self) {
        let line_len = self.commit_message[self.commit_cursor_y].chars().count();
        if self.commit_cursor_x < line_len {
            self.commit_cursor_x += 1;
        } else if self.commit_cursor_y < self.commit_message.len() - 1 {
            self.commit_cursor_y += 1;
            self.commit_cursor_x = 0;
        }
    }

    /// コミットカーソルを上に移動
    pub fn commit_cursor_up(&mut self) {
        if self.commit_cursor_y > 0 {
            self.commit_cursor_y -= 1;
            let line_len = self.commit_message[self.commit_cursor_y].chars().count();
            self.commit_cursor_x = self.commit_cursor_x.min(line_len);
        }
    }

    /// コミットカーソルを下に移動
    pub fn commit_cursor_down(&mut self) {
        if self.commit_cursor_y < self.commit_message.len() - 1 {
            self.commit_cursor_y += 1;
            let line_len = self.commit_message[self.commit_cursor_y].chars().count();
            self.commit_cursor_x = self.commit_cursor_x.min(line_len);
        }
    }

    /// コミット画面でのファイル選択を下に移動
    pub fn commit_file_next(&mut self) {
        let staged_count = self.get_staged_files().len();
        if staged_count > 0 {
            self.commit_file_index = (self.commit_file_index + 1).min(staged_count - 1);
        }
    }

    /// コミット画面でのファイル選択を上に移動
    pub fn commit_file_prev(&mut self) {
        if self.commit_file_index > 0 {
            self.commit_file_index -= 1;
        }
    }

    /// コミット画面でフォーカスを切り替え
    pub fn commit_toggle_focus(&mut self) {
        self.commit_focus_files = !self.commit_focus_files;
    }

    /// コミット画面で選択されたファイルの staged diff を取得（色付き）
    pub fn get_commit_staged_diff(&self) -> Result<String> {
        let staged = self.get_staged_files();
        if let Some(file) = staged.get(self.commit_file_index) {
            let path_str = file.path.to_string_lossy();
            self.git
                .execute(&["diff", "--staged", "--color=always", &path_str])
        } else {
            Ok(String::new())
        }
    }
}
