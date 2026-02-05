use std::path::Path;

use directories::ProjectDirs;
use glob::Pattern;

/// 表示フィルタ
///
/// `~/.config/git-tui/display_ignore` から読み込んだパターンに
/// マッチするファイル/ディレクトリを非表示にする
#[derive(Clone)]
pub struct DisplayFilter {
    patterns: Vec<Pattern>,
    enabled: bool,
}

impl DisplayFilter {
    /// 設定ファイルからフィルタを読み込む
    pub fn load() -> Self {
        let patterns = Self::load_patterns();
        Self {
            patterns,
            enabled: true,
        }
    }

    /// 設定ファイルからパターンを読み込む
    fn load_patterns() -> Vec<Pattern> {
        let config_path = ProjectDirs::from("", "", "git-tui")
            .map(|dirs| dirs.config_dir().join("display_ignore"));

        let Some(path) = config_path else {
            return Vec::new();
        };

        if !path.exists() {
            return Vec::new();
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => content
                .lines()
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .filter_map(|line| Pattern::new(line).ok())
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    /// フィルタが有効かどうか
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// フィルタの有効/無効を切り替える
    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }

    /// 指定されたパスがフィルタにマッチするかどうか
    ///
    /// フィルタが無効な場合は常に false を返す
    pub fn should_hide(&self, path: &Path) -> bool {
        if !self.enabled {
            return false;
        }

        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        self.patterns.iter().any(|pattern| pattern.matches(name))
    }
}

impl Default for DisplayFilter {
    fn default() -> Self {
        Self::load()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_filter_load_returns_filter() {
        // Arrange & Act
        let filter = DisplayFilter::load();

        // Assert
        assert!(filter.is_enabled());
    }

    #[test]
    fn test_display_filter_toggle_changes_enabled_state() {
        // Arrange
        let mut filter = DisplayFilter::load();
        let initial_state = filter.is_enabled();

        // Act
        filter.toggle();

        // Assert
        assert_ne!(filter.is_enabled(), initial_state);
    }

    #[test]
    fn test_should_hide_returns_false_when_disabled() {
        // Arrange
        let mut filter = DisplayFilter {
            patterns: vec![Pattern::new("*.txt").unwrap()],
            enabled: false,
        };
        filter.enabled = false;

        // Act & Assert
        assert!(!filter.should_hide(Path::new("test.txt")));
    }

    #[test]
    fn test_should_hide_returns_true_when_pattern_matches() {
        // Arrange
        let filter = DisplayFilter {
            patterns: vec![Pattern::new("node_modules").unwrap()],
            enabled: true,
        };

        // Act & Assert
        assert!(filter.should_hide(Path::new("/project/node_modules")));
    }
}
