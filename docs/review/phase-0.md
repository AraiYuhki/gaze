# Phase 0 Review Log

## Summary
- 指摘件数: 1（二次レビュー）
- 修正対応: 0
- 見送り: 1（Windows 対応は v0.1.0 スコープ外）

## レビュー観点

### 1. CLAUDE.md の絶対原則・禁止事項への違反確認
- **結果**: 違反なし
- Git 操作は `std::process::Command` 経由で `git` CLI を実行する設計
- libgit2、gitoxide 等の Git ライブラリは使用していない
- 禁止されている依存関係（git2, tokio, reqwest 等）は Cargo.toml に含まれていない

### 2. git コマンドの実行頻度
- **結果**: 問題なし
- Phase 0 では git コマンドを実行するコードは `GitCli` のテスト内のみ
- 描画ループ内での git コマンド実行は存在しない

### 3. UI 描画ループ内の処理
- **結果**: 問題なし
- `run_app` 関数のループは空の描画のみで、重い処理は含まれていない

### 4. 責務分離
- **結果**: 適切に分離されている
- `error.rs`: エラー型定義のみ
- `cli/executor.rs`: Git CLI 実行のみ
- `pager.rs`: 外部ページャ実行のみ
- `main.rs`: TUI 初期化とイベントループのみ

### 5. 大規模リポジトリでの性能
- **結果**: 問題なし
- Phase 0 ではファイルシステムの走査や大量データ処理は行っていない

## 実装内容の確認

### 作成されたファイル
| ファイル | 役割 | CLAUDE.md 準拠 |
|---------|------|---------------|
| `src/error.rs` | AppError 定義 | OK |
| `src/cli/mod.rs` | CLI モジュール公開 | OK |
| `src/cli/executor.rs` | GitCli 構造体 | OK |
| `src/pager.rs` | 外部ページャ | OK |
| `src/app.rs` | AppState プレースホルダ | OK |
| `src/main.rs` | TUI エントリポイント | OK |
| `src/domain/mod.rs` | Domain モジュール | OK |
| `src/ui/mod.rs` | UI モジュール | OK |
| `src/filter/mod.rs` | Filter モジュール | OK |
| `src/config/mod.rs` | Config モジュール | OK |

### 検証結果
- `cargo build`: 成功
- `cargo clippy -- -D warnings`: 成功（警告なし）
- `cargo test`: 5 テスト全て成功
- `cargo fmt --check`: 差分なし

## 備考
- Phase 0 では基盤コードの多くが未使用のため、`#[allow(dead_code)]` を適切に付与
- 各 `#[allow]` には「Phase 1 以降で使用されるため」という理由をコメントで記載済み

---

## 二次レビュー（Codex CLI による自動レビュー）

### 指摘事項

#### 1. [P2] Windows でのページャ起動失敗
- **指摘内容**:
  `src/pager.rs:46-48` で `sh -c` を使用してページャを起動しているが、Windows では `sh` が標準で存在しないため、`display()` が失敗する
- **推奨対応**:
  Windows では `cmd /C` を使用するか、シェルを介さずに直接実行する

### 見送り項目

#### 1. Windows 対応
- **見送り理由**:
  - CLAUDE.md に Windows 対応は明示的な要件として記載されていない
  - v0.1.0 のターゲットプラットフォームは macOS/Linux を想定
  - 将来的に Windows 対応が必要になった場合に対応する（v0.2.0 以降）
- **備考**:
  現状のコードは `default_pager()` で Windows の場合 `more` を返すが、実際には `sh` がないため動作しない。この不整合は認識済みだが、スコープ外のため見送り
