use thiserror::Error;

/// アプリケーション全体で使用するエラー型
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Git command failed: {0}")]
    GitCommand(String),

    #[error("Not a git repository")]
    NotGitRepo,

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Pager failed: {0}")]
    Pager(String),

    // TODO(Phase 4): 設定ファイル読み込みで使用予定
    #[allow(dead_code)]
    #[error("Config error: {0}")]
    Config(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, AppError>;
