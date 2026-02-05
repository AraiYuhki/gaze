# Git TUI 実装指示書

> このファイルは Claude Code への実装指示書である。
> 実装中は常にこのファイルを参照し、設計意図から逸脱しないこと。

---

## プロジェクト概要

**目的**: 軽量なGit操作補助TUIツールの開発

**絶対原則**（これに反する実装は禁止）:
1. すべてのGit操作は `git` CLIコマンド経由で実行する
2. libgit2、gitoxide 等のGitライブラリは使用禁止
3. submodule と LFS は一切解釈しない（警告表示のみ可）
4. `.gitignore` ファイルを変更・生成しない
5. 未完了の Phase を跨いだリファクタリングは禁止
6. 現在の Phase の完了条件を満たすための最小実装のみ行う

---

## 判断基準（迷った場合の優先順位）

1. **既存の制約を優先** - CLAUDE.md に記載された禁止事項・制約を守る
2. **パフォーマンスを優先** - 大規模リポジトリでの性能を考慮
3. **不明な場合は実装しない** - 理由をコードコメントで残す

**禁止**: 「より良い設計」「将来拡張」を理由に、CLAUDE.md に記載のない変更を行うこと

### TODO コメントのルール

TODO コメントは以下の場合のみ許可:
- 次 Phase で対応予定のもの
- 性能影響が不明で意図的に見送ったもの

**必須**: TODO には理由と対応予定 Phase を記載すること

```rust
// OK: 理由と Phase が明記されている
// TODO(Phase 2): TreeNode への status 反映。現時点では status_cache 未実装のため見送り

// OK: 性能理由で見送り
// TODO(Phase 3): 動的読み込み。現時点では性能影響が不明のため固定件数で実装

// NG: 理由がない
// TODO: あとで直す

// NG: Phase がない
// TODO: リファクタリングする
```

---

## 技術スタック（固定）

```toml
# Cargo.toml - この依存関係を使用すること
[package]
name = "git-tui"
version = "0.1.0"
edition = "2021"

[dependencies]
ratatui = "0.29"
crossterm = "0.28"
anyhow = "1.0"
thiserror = "2.0"
toml = "0.8"
directories = "6.0"
glob = "0.3"
```

**禁止する依存関係**:
- `git2`, `gitoxide`, `gix` （Gitライブラリ）
- `tokio`, `async-std` （Phase 2 完了まで）
- `reqwest`, `hyper` （ネットワーク不要）

---

## ディレクトリ構造（この通りに作成）

```
gaze/
├── CLAUDE.md           # この指示書
├── ROADMAP.md          # 進捗管理ファイル
├── Cargo.toml
├── src/
│   ├── main.rs         # エントリポイント
│   ├── app.rs          # AppState, イベントループ
│   ├── error.rs        # AppError 定義
│   ├── cli/
│   │   ├── mod.rs
│   │   ├── executor.rs # GitCli 構造体
│   │   └── parser.rs   # 各種パーサー
│   ├── domain/
│   │   ├── mod.rs
│   │   ├── status.rs   # FileStatus, StatusKind
│   │   ├── tree.rs     # TreeNode（遅延ロード）
│   │   └── log.rs      # GraphLine
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── status_view.rs
│   │   ├── tree_view.rs
│   │   └── log_view.rs
│   ├── filter/
│   │   ├── mod.rs
│   │   └── ignore.rs   # DisplayFilter
│   ├── config/
│   │   ├── mod.rs
│   │   └── settings.rs
│   └── pager.rs        # 外部ページャ
└── tests/
    └── integration/
```

---

# Rust コーディング規約

> 可読性を最優先とする。Rust に不慣れな読者でも意図が伝わるコードを書く。

---

## 命名規則

### 変数・関数
```rust
// OK: 意図が明確
let selected_index = 0;
let is_expanded = true;
fn parse_status_line(line: &str) -> Result<FileStatus>

// NG: 省略しすぎ
let idx = 0;
let exp = true;
fn parse_sl(l: &str) -> Result<FileStatus>
```

### 許可される短縮形
以下のみ省略を許可:
- `i`, `j` - ループインデックス
- `e` - イベント（`Event`）
- `f` - フレーム（`Frame`）
- `ctx` - コンテキスト
- `config` / `cfg` - 設定

### 型名・構造体
```rust
// OK: 役割が明確
pub struct FileStatus { ... }
pub struct TreeNode { ... }
pub enum StatusKind { ... }

// NG: 曖昧
pub struct Data { ... }
pub struct Item { ... }
pub enum Kind { ... }
```

---

## 関数設計

### 1関数1責務
```rust
// OK: 単一の責務
fn parse_status_line(line: &str) -> Result<FileStatus> { ... }
fn render_status_view(f: &mut Frame, state: &AppState) { ... }

// NG: 複数の責務が混在
fn parse_and_render_status(line: &str, f: &mut Frame) { ... }
```

### 関数の長さ
- **目安**: 50行以内
- 超える場合は責務分割を検討
- 分割不可能な場合はコメントでブロックを区切る

### 引数の数
- **目安**: 4つ以内
- 超える場合は構造体にまとめる

```rust
// OK
fn render_tree(f: &mut Frame, node: &TreeNode, options: &RenderOptions)

// NG: 引数が多すぎる
fn render_tree(f: &mut Frame, node: &TreeNode, indent: usize, 
               show_hidden: bool, status_cache: &[FileStatus], selected: bool)
```

---

## エラーハンドリング

### unwrap / expect の使用制限
```rust
// OK: 確実に Some/Ok の場合のみ、理由をコメント
let first = items.first().expect("items is guaranteed non-empty by caller");

// OK: テストコード内
#[test]
fn test_parse() {
    let result = parse("test").unwrap();
}

// NG: 本番コードでの無条件 unwrap
let value = some_option.unwrap();
```

### Result の伝播
```rust
// OK: ? 演算子で伝播
fn load_config() -> Result<Config> {
    let content = std::fs::read_to_string(path)?;
    let config = toml::from_str(&content)?;
    Ok(config)
}

// NG: unwrap で握りつぶし
fn load_config() -> Config {
    let content = std::fs::read_to_string(path).unwrap();
    toml::from_str(&content).unwrap()
}
```

---

## コメント規約

### 関数コメント
公開関数（`pub`）には必ず doc コメントを付ける:

```rust
/// git status --porcelain=v1 の出力をパースする
///
/// # Errors
/// - 行のフォーマットが不正な場合
pub fn parse_status(output: &str) -> Result<Vec<FileStatus>> { ... }
```

### 実装コメント
「何をしているか」ではなく「なぜそうしているか」を書く:

```rust
// OK: 理由を説明
// .git ディレクトリは git 管理対象外のため除外
if name == ".git" {
    continue;
}

// NG: コードをそのまま言い換えただけ
// name が ".git" なら continue する
if name == ".git" {
    continue;
}
```

### TODO コメント（再掲）
```rust
// OK
// TODO(Phase 2): TreeNode への status 反映。status_cache 未実装のため見送り

// NG
// TODO: あとで直す
```

---

## 構造体設計

### フィールドの順序
1. 識別子（id, name, path）
2. 状態（status, kind, expanded）
3. 関連データ（children, cache）
4. 設定・オプション

```rust
pub struct TreeNode {
    // 1. 識別子
    pub name: String,
    pub path: PathBuf,
    
    // 2. 状態
    pub kind: NodeKind,
    pub expanded: bool,
    pub git_status: Option<StatusKind>,
    
    // 3. 関連データ
    pub children: Option<Vec<TreeNode>>,
}
```

### derive の順序
```rust
#[derive(Debug, Clone, PartialEq, Eq)]  // この順序で統一
pub enum StatusKind { ... }
```

---

## モジュール設計

### mod.rs の役割
- 公開 API の re-export のみ
- ロジックは書かない

```rust
// src/domain/mod.rs
mod status;
mod tree;
mod log;

pub use status::{FileStatus, StatusKind};
pub use tree::{TreeNode, NodeKind};
pub use log::GraphLine;
```

### ファイル分割の基準
- 1ファイル 300行を目安
- 超える場合は責務で分割

---

## インポート順序

```rust
// 1. 標準ライブラリ
use std::path::PathBuf;
use std::process::Command;

// 2. 外部クレート（アルファベット順）
use anyhow::Result;
use ratatui::Frame;

// 3. クレート内モジュール
use crate::domain::FileStatus;
use crate::error::AppError;
```

---

## テストコード

### テスト関数名
```rust
// OK: test_ + 対象 + 状況 + 期待結果
#[test]
fn test_parse_status_modified_file_returns_modified_kind() { ... }

#[test]
fn test_parse_status_empty_input_returns_empty_vec() { ... }

// NG: 曖昧
#[test]
fn test_parse() { ... }

#[test]
fn test1() { ... }
```

### テストの構造（AAA パターン）
```rust
#[test]
fn test_parse_status_modified_file() {
    // Arrange: 準備
    let input = " M src/main.rs";
    
    // Act: 実行
    let result = parse_status_line(input).unwrap();
    
    // Assert: 検証
    assert_eq!(result.worktree, StatusKind::Modified);
    assert_eq!(result.path, PathBuf::from("src/main.rs"));
}
```

---

## 禁止事項

1. **マクロの濫用**: 標準的な制御構文で書けるならマクロを作らない
2. **過度なジェネリクス**: 具体的な型で十分なら型パラメータを使わない
3. **unsafe の使用**: このプロジェクトでは原則禁止
4. **グローバル状態**: `static mut`, `lazy_static` の使用禁止

---

## フォーマット

### 自動フォーマット
```bash
cargo fmt
```
を必ず適用する。手動での整形は禁止。

### clippy
```bash
cargo clippy -- -D warnings
```
警告をすべて解消する。`#[allow(...)]` での抑制は理由をコメントに記載。

---

## 実装フェーズ

### Phase 0: 基盤構築

#### 0-1: プロジェクト作成
```bash
cargo new git-tui
cd git-tui
```

#### 0-2: error.rs
```rust
use thiserror::Error;

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

pub type Result<T> = std::result::Result<T, AppError>;
```

#### 0-3: cli/executor.rs
```rust
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::error::{AppError, Result};

pub struct GitCli {
    repo_root: PathBuf,
}

impl GitCli {
    pub fn new(path: &Path) -> Result<Self> {
        let output = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .current_dir(path)
            .output()?;
        
        if !output.status.success() {
            return Err(AppError::NotGitRepo);
        }
        
        let root = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();
        
        Ok(Self { repo_root: PathBuf::from(root) })
    }
    
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
    
    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }
}
```

**重要**: `execute` は同期関数。async にしないこと。

---

## Git Status 取得タイミング（厳守）

### 許可されるタイミング
1. **アプリ起動時**（1回のみ）
2. **View 切り替え時**（Status/Tree View に切り替えた時）
3. **手動リフレッシュ時**（R キー）
4. **変更操作の直後**（s, r, c 等の操作完了後、1回のみ）

### 禁止されるタイミング
- 描画ループ毎
- キー入力毎
- TreeNode 展開時
- スクロール時

### 実装パターン
```rust
pub struct AppState {
    // Status キャッシュ（これを参照する）
    status_cache: Vec<FileStatus>,
    status_cache_dirty: bool,
}

impl AppState {
    /// 許可されたタイミングでのみ呼び出す
    fn refresh_status(&mut self) -> Result<()> {
        let output = self.git.execute(&["status", "--porcelain=v1"])?;
        self.status_cache = parse_status(&output)?;
        self.status_cache_dirty = false;
        Ok(())
    }
    
    /// 変更操作後にフラグを立てる
    fn mark_status_dirty(&mut self) {
        self.status_cache_dirty = true;
    }
    
    /// 操作完了後に1回だけリフレッシュ
    fn handle_stage(&mut self, file: &Path) -> Result<()> {
        self.git.execute(&["add", file.to_str().unwrap()])?;
        self.refresh_status()?;  // 操作直後に1回だけ
        Ok(())
    }
}
```

**TreeNode は status_cache を参照するのみ**:
```rust
impl TreeNode {
    /// git コマンドを実行しない。キャッシュから検索のみ。
    pub fn update_status_from_cache(&mut self, cache: &[FileStatus]) {
        if let Some(status) = cache.iter().find(|s| s.path == self.path) {
            self.git_status = Some(status.worktree);
        }
    }
}
```

#### 0-4: TUI 初期化（main.rs）
```rust
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

mod app;
mod cli;
mod config;
mod domain;
mod error;
mod filter;
mod pager;
mod ui;

fn main() -> Result<()> {
    // パニック時にターミナルを復帰させる
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    loop {
        terminal.draw(|f| {
            // TODO: UI描画
        })?;

        if let Event::Key(key) = event::read()? {
            if key.code == KeyCode::Char('q') {
                return Ok(());
            }
        }
    }
}
```

#### Phase 0 完了条件
以下がすべて成功すること:
```bash
cargo build
cargo clippy -- -D warnings
cargo test
./target/debug/git-tui  # 起動して q で終了できる
```

---

### Phase 1: Status View

#### 1-1: domain/status.rs
```rust
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Unmodified,
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    Untracked,
    Ignored,
}

#[derive(Debug, Clone)]
pub struct FileStatus {
    pub index: StatusKind,
    pub worktree: StatusKind,
    pub path: PathBuf,
    pub original_path: Option<PathBuf>,
}
```

#### 1-2: cli/parser.rs - status パーサー
```rust
use crate::domain::status::{FileStatus, StatusKind};
use crate::error::Result;
use std::path::PathBuf;

pub fn parse_status(output: &str) -> Result<Vec<FileStatus>> {
    output
        .lines()
        .filter(|line| !line.is_empty())
        .map(parse_status_line)
        .collect()
}

fn parse_status_line(line: &str) -> Result<FileStatus> {
    if line.len() < 4 {
        return Err(crate::error::AppError::Parse(
            format!("Invalid status line: {}", line)
        ));
    }
    
    let index = parse_status_char(line.chars().next().unwrap());
    let worktree = parse_status_char(line.chars().nth(1).unwrap());
    
    let path_part = &line[3..];
    let (path, original_path) = if let Some(pos) = path_part.find(" -> ") {
        (PathBuf::from(&path_part[pos + 4..]), Some(PathBuf::from(&path_part[..pos])))
    } else {
        (PathBuf::from(path_part), None)
    };
    
    Ok(FileStatus { index, worktree, path, original_path })
}

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
```

**テスト必須**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_modified() {
        let result = parse_status(" M src/main.rs").unwrap();
        assert_eq!(result.worktree, StatusKind::Modified);
        assert_eq!(result.path, PathBuf::from("src/main.rs"));
    }
    
    #[test]
    fn test_parse_renamed() {
        let result = parse_status("R  old.rs -> new.rs").unwrap();
        assert_eq!(result.index, StatusKind::Renamed);
        assert_eq!(result.path, PathBuf::from("new.rs"));
        assert_eq!(result.original_path, Some(PathBuf::from("old.rs")));
    }
}
```

#### 1-3: ui/status_view.rs 実装要件

- `git status --porcelain=v1` の出力を表示
- j/k でカーソル移動
- s でステージ/アンステージ切替
  - ステージ: `git add <file>`
  - アンステージ: `git restore --staged <file>`
- d で差分表示（外部ページャ）
- r で変更破棄（確認ダイアログ必須）
  - `git restore <file>`

#### Phase 1 完了条件
```bash
cargo test
cargo clippy -- -D warnings

# 手動確認
# 1. git リポジトリ内で起動
# 2. ファイルを変更して status に表示される
# 3. s キーでステージング/アンステージングができる
# 4. d キーで差分が外部ページャで表示される
```

---

### Phase 2: Tree View

#### 2-1: domain/tree.rs - 遅延ロード必須
```rust
use std::path::PathBuf;
use crate::domain::status::StatusKind;
use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    File,
    Directory,
}

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub name: String,
    pub path: PathBuf,
    pub kind: NodeKind,
    pub children: Option<Vec<TreeNode>>,  // None = 未ロード
    pub expanded: bool,
    pub git_status: Option<StatusKind>,
}

impl TreeNode {
    /// 新しいディレクトリノード（子は未ロード状態）
    pub fn new_dir(name: String, path: PathBuf) -> Self {
        Self {
            name,
            path,
            kind: NodeKind::Directory,
            children: None,  // 重要: 初期状態は None
            expanded: false,
            git_status: None,
        }
    }
    
    /// 展開時に呼び出す。children が None の場合のみロード実行。
    pub fn load_children(&mut self) -> Result<()> {
        if self.kind != NodeKind::Directory {
            return Ok(());
        }
        if self.children.is_some() {
            return Ok(());  // 既にロード済み
        }
        
        let mut children = Vec::new();
        for entry in std::fs::read_dir(&self.path)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            
            // .git ディレクトリは除外
            if name == ".git" {
                continue;
            }
            
            let path = entry.path();
            let kind = if path.is_dir() {
                NodeKind::Directory
            } else {
                NodeKind::File
            };
            
            children.push(TreeNode {
                name,
                path,
                kind,
                children: if kind == NodeKind::Directory { None } else { Some(vec![]) },
                expanded: false,
                git_status: None,
            });
        }
        
        // ディレクトリ優先、次に名前順でソート
        children.sort_by(|a, b| {
            match (&a.kind, &b.kind) {
                (NodeKind::Directory, NodeKind::File) => std::cmp::Ordering::Less,
                (NodeKind::File, NodeKind::Directory) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });
        
        self.children = Some(children);
        Ok(())
    }
}
```

**絶対禁止**: 
- 初期化時に再帰的に全ディレクトリを走査しないこと
- `load_children` はユーザーが展開操作をした時のみ呼び出す
- `load_children` 内で git コマンドを実行しないこと

**Git Status の反映方法**:
```rust
impl TreeNode {
    /// load_children 後にキャッシュから status を設定
    /// git コマンドは実行しない
    pub fn apply_status_cache(&mut self, cache: &[FileStatus]) {
        if let Some(children) = &mut self.children {
            for child in children {
                if let Some(status) = cache.iter().find(|s| s.path == child.path) {
                    child.git_status = Some(status.worktree);
                }
            }
        }
    }
}

// 使用例（展開操作時）
fn expand_node(&mut self, node: &mut TreeNode) -> Result<()> {
    node.load_children()?;                    // ファイルシステムのみ読む
    node.apply_status_cache(&self.status_cache);  // キャッシュから適用
    node.expanded = true;
    Ok(())
}
```

#### Phase 2 完了条件
```bash
cargo test
cargo clippy -- -D warnings

# 手動確認
# 1. 起動時はルート直下のみ表示（深い階層は未ロード）
# 2. Enter でディレクトリ展開時に初めて子がロードされる
# 3. 数万ファイルのリポジトリでも起動が高速
```

---

### Phase 3: Log View

#### 3-1: domain/log.rs
```rust
#[derive(Debug, Clone)]
pub struct GraphLine {
    pub raw_line: String,           // フォールバック用
    pub graph_chars: String,        // グラフ部分（解析しない）
    pub hash: Option<String>,       // 7文字ハッシュ
    pub refs: Vec<String>,          // ブランチ名、タグ
    pub message: Option<String>,
}
```

#### 3-2: パーサー実装方針

**禁止事項**:
- グラフ構造（親子関係）の解析
- コミットツリーの再構築

**許可事項**:
- 行を「グラフ文字」「ハッシュ」「refs」「メッセージ」に分割
- パース失敗時は `raw_line` をそのまま表示

```rust
pub fn parse_log_line(line: &str) -> GraphLine {
    // パースに失敗したら raw_line だけ設定して返す
    GraphLine {
        raw_line: line.to_string(),
        graph_chars: extract_graph_chars(line).unwrap_or_default(),
        hash: extract_hash(line),
        refs: extract_refs(line),
        message: extract_message(line),
    }
}
```

#### Phase 3 完了条件
```bash
cargo test
cargo clippy -- -D warnings

# 手動確認
# 1. git log --oneline --graph の出力が表示される
# 2. パースできない行もクラッシュせず表示される
# 3. Enter でコミット詳細がページャで表示される
```

---

### Phase 4: 統合

- View 切り替え（1, 2, 3 キー）
- 設定ファイル読み込み
- ヘルプ画面（? キー）
- README.md 作成

---

### Phase 6: 差分表示の改善

#### 6-1: 差分の色分け表示

**実装方針**:
- `git diff --color=always` オプションを使用して色付き出力を取得
- ANSIカラーコードを含んだ出力をそのままページャに渡す
- ページャ側で色コードを解釈して表示

**対象箇所**:
1. Status View の差分表示（`d` キー）
2. コミットモードの staged diff（`d` キー）
3. Log View のコミット詳細表示（Enter キー）

**実装例**:
```rust
// 差分取得時に --color=always を追加
pub fn get_diff(&self) -> Result<String> {
    if let Some(file) = self.selected_file() {
        let path_str = file.path.to_string_lossy();
        if file.index != StatusKind::Unmodified && file.index != StatusKind::Untracked {
            self.git.execute(&["diff", "--cached", "--color=always", path_str.as_ref()])
        } else {
            self.git.execute(&["diff", "--color=always", path_str.as_ref()])
        }
    } else {
        Ok(String::new())
    }
}
```

#### 6-2: デフォルトページャ設定の改善

**変更内容**:
- Unix系のデフォルトページャを `less` から `less -R` に変更
- `-R` オプションにより ANSI カラーコードを解釈して色付き表示

```rust
fn default_pager() -> String {
    if cfg!(target_os = "windows") {
        "more".to_string()  // Windows は色対応が限定的
    } else {
        "less -R".to_string()  // -R で色付き出力対応
    }
}
```

**注意点**:
- 環境変数 `$PAGER` や設定ファイルでユーザーがページャを指定している場合は、
  ユーザー設定を優先する（`-R` は自動付与しない）
- `git show` も同様に `--color=always` を付与する

---

## 重要な制約（常に遵守）

### Git Status 取得タイミング
```rust
// 許可: 操作完了後に1回だけ
fn handle_stage(&mut self, file: &Path) -> Result<()> {
    self.git.execute(&["add", file.to_str().unwrap()])?;
    self.refresh_status()?;  // OK: 操作直後に1回
    Ok(())
}

// 禁止: 描画ループ内
fn draw(&mut self) {
    self.refresh_status();  // NG! 毎フレーム実行される
    // ...
}

// 禁止: キー入力毎
fn handle_key(&mut self, key: KeyCode) {
    self.refresh_status();  // NG! 全キー入力で実行される
    match key { ... }
}

// 禁止: TreeNode 展開時
fn load_children(&mut self) -> Result<()> {
    self.git.execute(&["status", ...])?;  // NG!
    // ...
}
```

### Git コマンド実行
```rust
// 正しい実装
let output = self.git.execute(&["status", "--porcelain=v1"])?;

// 禁止（ライブラリ使用）
let repo = git2::Repository::open(".")?;
```

### Tree の遅延ロード
```rust
// 正しい実装
if self.children.is_none() {
    self.load_children()?;
}

// 禁止（初期化時の全走査）
fn new(path: &Path) -> Self {
    let children = walk_dir_recursive(path);  // NG!
}
```

### エラーハンドリング
```rust
// 正しい実装（アプリ継続）
if let Err(e) = self.pager.display(&diff) {
    self.show_error(&format!("Pager failed: {}", e));
}

// 禁止（即座にパニック）
self.pager.display(&diff).unwrap();
```

---

## トラブルシューティング

### コンパイルエラーが解消しない
1. `cargo clean` を実行
2. エラーメッセージの最初のエラーから順に対処
3. 依存関係のバージョン不整合を確認

### TUI が崩れる
1. ターミナルサイズ取得を確認: `terminal.size()?`
2. 描画エリアの境界チェックを追加

### git コマンドが失敗する
1. `GitCli::execute` の stderr 出力を確認
2. リポジトリルートが正しいか確認
3. git コマンドを手動で実行して出力を確認

### パフォーマンスが悪い
1. Tree が初期化時に全走査していないか確認
2. 毎フレーム git コマンドを実行していないか確認
3. 必要に応じてキャッシュを導入

---

## セッション開始時の確認事項

新しいセッションを開始する際は、以下を確認:

1. 現在どの Phase にいるか
2. 直前のコミット内容
3. 未完了のタスク
4. 発生しているエラー

**コンテキストが不明な場合**:
```bash
git log --oneline -5
cargo build 2>&1 | head -20
cargo test 2>&1 | tail -20
```

---

## 最終チェックリスト（v0.1.0 リリース前）

- [ ] `cargo build --release` が成功
- [ ] `cargo clippy -- -D warnings` が警告なし
- [ ] `cargo test` が全パス
- [ ] `cargo fmt --check` が差分なし
- [ ] README.md が存在し、インストール・使用方法が記載
- [ ] git リポジトリ内で起動して基本操作が可能
- [ ] git リポジトリ外で起動するとエラーメッセージが表示される
- [ ] 全 Phase のレビューログが `docs/review/` に存在

---

## レビュー体制

### 前提
ユーザーは Rust のコード内容を詳細にレビューできない。
コード品質・設計妥当性・制約遵守のレビューは二段階で実施する。

### レビュー構成
| 担当 | 役割 | 観点 |
|------|------|------|
| Claude Code | 一次レビュー（自己レビュー） | 実装者として制約遵守・品質確認 |
| CodexCLI | 二次レビュー（第三者視点） | 別セッションで客観的に検証 |

### レビュー実施タイミング
- **各 Phase 完了時にのみ実施**
- コミット単位・Phase 途中でのレビューは行わない

理由:
- Phase 途中は設計が未完成であり、レビュー精度が低下するため
- ノイズの多い指摘（未使用コード等）を避けるため

### レビュー対象範囲
- 当該 Phase で変更・追加されたファイルのみ
- CLAUDE.md に記載された「絶対原則」「禁止事項」への違反有無を最優先で確認

### レビュー観点
最低限、以下を確認すること:
1. CLAUDE.md の絶対原則・禁止事項への違反がないか
2. git コマンドが不要に頻繁に実行されていないか
3. UI 描画ループ内に重い処理が含まれていないか
4. 責務分離が崩れていないか（Domain / UI / CLI の混在）
5. 大規模リポジトリ（数万ファイル）で性能劣化する処理がないか

### レビュー結果の出力（必須）

**出力先**: `docs/review/phase-N.md`

**記載ルール**:
- Rust の文法説明は禁止
- ユーザーが「コードを読まずに理解できる」日本語で記述

**フォーマット**:
```markdown
# Phase N Review Log

## Summary
- 指摘件数: X
- 修正対応: Y
- 見送り: Z（理由を明記）

## Issues & Fixes

### 1. 問題点の要約
- 指摘内容:
  （何が問題だったか）
- 対応:
  （どう修正したか）
- 影響:
  （何が改善されたか）

## 見送り項目（ある場合）

### 1. 見送り項目の要約
- 見送り理由:
  （設計意図・将来 Phase で対応する等）
```

### レビュー後の進行
- **レビュー対応完了後にのみ次 Phase に進むこと**
- レビューで発見された問題は、次 Phase に進む前に必ず修正する
