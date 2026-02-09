use super::AppState;
use crate::app::View;

impl AppState {
    /// 選択を1つ下に移動
    pub fn select_next(&mut self) {
        match self.current_view {
            View::Status => {
                if !self.status_cache.is_empty() {
                    self.selected_index =
                        (self.selected_index + 1).min(self.status_cache.len() - 1);
                }
            }
            View::Tree => {
                let max = self.get_tree_flat_len();
                if max > 0 {
                    self.tree_selected_index = (self.tree_selected_index + 1).min(max - 1);
                }
            }
            View::Log => {
                if self.is_file_log_mode() {
                    if !self.file_log_cache.is_empty() {
                        self.file_log_selected_index =
                            (self.file_log_selected_index + 1).min(self.file_log_cache.len() - 1);
                    }
                } else if !self.log_cache.is_empty() {
                    self.log_selected_index =
                        (self.log_selected_index + 1).min(self.log_cache.len() - 1);
                }
            }
            View::Stash => {
                if !self.stash_cache.is_empty() {
                    self.stash_selected_index =
                        (self.stash_selected_index + 1).min(self.stash_cache.len() - 1);
                }
            }
            View::Branch => {
                let filtered = self.filtered_branches();
                if !filtered.is_empty() {
                    self.branch_selected_index =
                        (self.branch_selected_index + 1).min(filtered.len() - 1);
                }
            }
        }
    }

    /// 選択を1つ上に移動
    pub fn select_previous(&mut self) {
        match self.current_view {
            View::Status => {
                self.selected_index = self.selected_index.saturating_sub(1);
            }
            View::Tree => {
                self.tree_selected_index = self.tree_selected_index.saturating_sub(1);
            }
            View::Log => {
                if self.is_file_log_mode() {
                    self.file_log_selected_index = self.file_log_selected_index.saturating_sub(1);
                } else {
                    self.log_selected_index = self.log_selected_index.saturating_sub(1);
                }
            }
            View::Stash => {
                self.stash_selected_index = self.stash_selected_index.saturating_sub(1);
            }
            View::Branch => {
                self.branch_selected_index = self.branch_selected_index.saturating_sub(1);
            }
        }
    }

    /// 選択を先頭に移動
    pub fn select_first(&mut self) {
        match self.current_view {
            View::Status => {
                self.selected_index = 0;
            }
            View::Tree => {
                self.tree_selected_index = 0;
            }
            View::Log => {
                if self.is_file_log_mode() {
                    self.file_log_selected_index = 0;
                } else {
                    self.log_selected_index = 0;
                }
            }
            View::Stash => {
                self.stash_selected_index = 0;
            }
            View::Branch => {
                self.branch_selected_index = 0;
            }
        }
    }

    /// 選択を末尾に移動
    pub fn select_last(&mut self) {
        match self.current_view {
            View::Status => {
                if !self.status_cache.is_empty() {
                    self.selected_index = self.status_cache.len() - 1;
                }
            }
            View::Tree => {
                let max = self.get_tree_flat_len();
                if max > 0 {
                    self.tree_selected_index = max - 1;
                }
            }
            View::Log => {
                if self.is_file_log_mode() {
                    if !self.file_log_cache.is_empty() {
                        self.file_log_selected_index = self.file_log_cache.len() - 1;
                    }
                } else if !self.log_cache.is_empty() {
                    self.log_selected_index = self.log_cache.len() - 1;
                }
            }
            View::Stash => {
                if !self.stash_cache.is_empty() {
                    self.stash_selected_index = self.stash_cache.len() - 1;
                }
            }
            View::Branch => {
                let filtered = self.filtered_branches();
                if !filtered.is_empty() {
                    self.branch_selected_index = filtered.len() - 1;
                }
            }
        }
    }
}
