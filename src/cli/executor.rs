use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{AppError, Result};

/// Git CLI コマンドを実行するための構造体
// Phase 1 以降で使用されるため dead_code を許可
#[allow(dead_code)]
pub struct GitCli {
    repo_root: PathBuf,
}

// Phase 1 以降で使用されるため dead_code を許可
#[allow(dead_code)]
impl GitCli {
    /// 指定されたパスから Git リポジトリを検出して GitCli を作成する
    ///
    /// # Errors
    /// - 指定されたパスが Git リポジトリ内でない場合
    pub fn new(path: &Path) -> Result<Self> {
        let output = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .current_dir(path)
            .output()?;

        if !output.status.success() {
            return Err(AppError::NotGitRepo);
        }

        let root = String::from_utf8_lossy(&output.stdout).trim().to_string();

        Ok(Self {
            repo_root: PathBuf::from(root),
        })
    }

    /// Git コマンドを実行して標準出力を返す
    ///
    /// # Errors
    /// - コマンド実行に失敗した場合
    /// - コマンドが非ゼロの終了コードを返した場合
    pub fn execute(&self, args: &[&str]) -> Result<String> {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.repo_root)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::GitCommand(stderr.to_string()));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// リポジトリのルートパスを返す
    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_cli_new_in_git_repo_succeeds() {
        // このテストは git リポジトリ内で実行されることを前提とする
        let current_dir = std::env::current_dir().expect("Failed to get current directory");
        let result = GitCli::new(&current_dir);
        assert!(result.is_ok());
    }

    #[test]
    fn test_git_cli_new_outside_git_repo_fails() {
        // /tmp は通常 git リポジトリではない
        let result = GitCli::new(Path::new("/tmp"));
        assert!(result.is_err());
    }

    #[test]
    fn test_git_cli_execute_status_succeeds() {
        let current_dir = std::env::current_dir().expect("Failed to get current directory");
        let git = GitCli::new(&current_dir).expect("Failed to create GitCli");
        let result = git.execute(&["status", "--porcelain=v1"]);
        assert!(result.is_ok());
    }
}
