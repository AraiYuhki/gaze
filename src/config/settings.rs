use std::path::PathBuf;

use directories::ProjectDirs;
use serde::Deserialize;

use crate::error::{AppError, Result};

/// ページャ設定
#[derive(Debug, Clone, Deserialize, Default)]
pub struct PagerConfig {
    /// ページャコマンド（例: "less -R"）
    pub command: Option<String>,
}

/// アプリケーション設定
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Settings {
    /// ページャ設定
    #[serde(default)]
    pub pager: PagerConfig,
}

impl Settings {
    /// 設定ファイルを読み込む
    ///
    /// 設定ファイルが存在しない場合はデフォルト値を返す
    ///
    /// # Errors
    /// - 設定ファイルのパースに失敗した場合
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();

        // 設定ファイルが存在しない場合はデフォルト値を返す
        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&config_path)?;
        let settings: Settings = toml::from_str(&content)
            .map_err(|e| AppError::Config(format!("Failed to parse config: {}", e)))?;

        Ok(settings)
    }

    /// 設定ファイルのパスを取得する
    fn config_path() -> PathBuf {
        // XDG Base Directory Specification に従う
        if let Some(proj_dirs) = ProjectDirs::from("", "", "git-tui") {
            proj_dirs.config_dir().join("config.toml")
        } else {
            // フォールバック: ~/.config/git-tui/config.toml
            dirs_fallback().join("config.toml")
        }
    }
}

/// フォールバック用のディレクトリパスを取得
fn dirs_fallback() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(".config").join("git-tui")
    } else {
        PathBuf::from(".config").join("git-tui")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_default_returns_empty_pager_command() {
        let settings = Settings::default();
        assert!(settings.pager.command.is_none());
    }

    #[test]
    fn test_settings_load_returns_default_when_no_config_file() {
        // 設定ファイルが存在しない場合（通常のテスト環境）
        // load() がエラーを返さないことを確認
        let result = Settings::load();
        assert!(result.is_ok());
    }
}
