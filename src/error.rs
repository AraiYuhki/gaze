use thiserror::Error;

/// アプリケーション全体で使用するエラー型
// Phase 1 以降で使用されるため dead_code を許可
#[allow(dead_code)]
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

    #[error("Config error: {0}")]
    Config(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

// Phase 1 以降で使用されるため dead_code を許可
#[allow(dead_code)]
pub type Result<T> = std::result::Result<T, AppError>;
