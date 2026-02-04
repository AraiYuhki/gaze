use std::env;
use std::io::Write;
use std::process::{Command, Stdio};

use crate::error::{AppError, Result};

/// 外部ページャを使用してテキストを表示する
// Phase 1 以降で使用されるため dead_code を許可
#[allow(dead_code)]
pub struct Pager {
    command: String,
}

// Phase 1 以降で使用されるため dead_code を許可
#[allow(dead_code)]
impl Pager {
    /// 環境変数からページャコマンドを決定して Pager を作成する
    ///
    /// 優先順位:
    /// 1. $GIT_PAGER
    /// 2. $PAGER
    /// 3. less (Unix系) / more (フォールバック)
    pub fn new() -> Self {
        let command = env::var("GIT_PAGER")
            .or_else(|_| env::var("PAGER"))
            .unwrap_or_else(|_| Self::default_pager());

        Self { command }
    }

    /// OS に応じたデフォルトページャを返す
    fn default_pager() -> String {
        if cfg!(target_os = "windows") {
            "more".to_string()
        } else {
            "less".to_string()
        }
    }

    /// ページャでテキストを表示する
    ///
    /// # Errors
    /// - ページャの起動に失敗した場合
    /// - ページャへの書き込みに失敗した場合
    pub fn display(&self, content: &str) -> Result<()> {
        let mut child = Command::new("sh")
            .args(["-c", &self.command])
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|e| AppError::Pager(format!("Failed to spawn pager: {}", e)))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(content.as_bytes())
                .map_err(|e| AppError::Pager(format!("Failed to write to pager: {}", e)))?;
        }

        child
            .wait()
            .map_err(|e| AppError::Pager(format!("Failed to wait for pager: {}", e)))?;

        Ok(())
    }
}

impl Default for Pager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pager_new_returns_pager() {
        let pager = Pager::new();
        // ページャコマンドが空でないことを確認
        assert!(!pager.command.is_empty());
    }

    #[test]
    fn test_default_pager_is_less_on_unix() {
        if !cfg!(target_os = "windows") {
            let pager = Pager::default_pager();
            assert_eq!(pager, "less");
        }
    }
}
