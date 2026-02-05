# Phase 1 Review Log

## Summary
- 指摘件数: 2（二次レビュー）
- 修正対応: 1
- 見送り: 1（Windows 対応は v0.1.0 スコープ外）

## レビュー観点

### 1. CLAUDE.md の絶対原則・禁止事項への違反確認
- **結果**: 違反なし
- Git 操作は全て `GitCli::execute()` 経由で `git` CLI を実行
- libgit2、gitoxide 等の Git ライブラリは使用していない
- 禁止されている依存関係は追加していない

### 2. git コマンドの実行頻度
- **結果**: 問題なし
- `refresh_status()` は許可されたタイミングでのみ呼び出される:
  - アプリ起動時（1回）
  - 手動リフレッシュ時（R キー）
  - 変更操作の直後（s, r キーの操作完了後）
- 描画ループ内での git コマンド実行は存在しない
- キー入力毎の git コマンド実行は存在しない

### 3. UI 描画ループ内の処理
- **結果**: 問題なし
- `run_app` のループは描画とイベント処理のみ
- `terminal.draw()` 内で git コマンドを実行していない
- ファイルシステムアクセスは行っていない

### 4. 責務分離
- **結果**: 適切に分離されている
- `domain/status.rs`: データ構造のみ（StatusKind, FileStatus）
- `cli/parser.rs`: パース処理のみ
- `cli/executor.rs`: git コマンド実行のみ
- `app.rs`: アプリケーション状態管理
- `ui/status_view.rs`: UI 描画のみ
- `pager.rs`: 外部ページャ連携のみ

### 5. 大規模リポジトリでの性能
- **結果**: 問題なし
- `git status --porcelain=v1` は git 自身が最適化
- status キャッシュを保持し、不要な再取得を防止
- 描画処理は `status_cache` を参照するのみ

## 実装内容の確認

### 作成・変更されたファイル
| ファイル | 役割 | CLAUDE.md 準拠 |
|---------|------|---------------|
| `src/domain/status.rs` | StatusKind, FileStatus | OK |
| `src/domain/mod.rs` | Domain モジュール公開 | OK |
| `src/cli/parser.rs` | Status パーサー | OK |
| `src/cli/mod.rs` | CLI モジュール公開 | OK |
| `src/app.rs` | AppState 実装 | OK |
| `src/ui/status_view.rs` | Status View UI | OK |
| `src/ui/mod.rs` | UI モジュール公開 | OK |
| `src/main.rs` | イベントループ統合 | OK |

### テスト結果
- `cargo build`: 成功
- `cargo clippy -- -D warnings`: 成功（警告なし）
- `cargo test`: 17 テスト全て成功
- `cargo fmt --check`: 差分なし

### 機能確認
- [x] j/k でカーソル移動
- [x] g で先頭へ、G で末尾へ
- [x] s でステージ/アンステージ切替
- [x] d で差分表示（外部ページャ）
- [x] r で変更破棄（確認ダイアログ）
- [x] R で手動リフレッシュ
- [x] q で終了

## 備考
- Phase 2 以降で使用される未使用コードには `#[allow(dead_code)]` を適切に付与
- 各 `#[allow]` には対応予定の Phase をコメントで記載済み

---

## 二次レビュー（Codex CLI による自動レビュー）

### 指摘事項

#### 1. [P2] 引用符付きパスのパース問題（修正済み）
- **指摘内容**:
  `git status --porcelain=v1` ではスペースや特殊文字を含むファイル名は C スタイルの引用符付きで出力される（例: `"my file.txt"`）。パーサーが引用符をそのまま保持していたため、`git add`/`git restore`/`git diff` が失敗する
- **対応**:
  `unquote_path()` 関数を追加し、引用符の除去と C スタイルエスケープ（`\n`, `\t`, 8進数等）の処理を実装
- **影響**:
  スペースや特殊文字を含むファイル名でもステージング・差分表示・変更破棄が正常に動作するようになった

#### 2. [P2] Windows での pager 問題（見送り）
- **指摘内容**:
  `src/pager.rs` で `sh -c` を使用してページャを起動しているが、Windows では `sh` が存在しないため失敗する
- **見送り理由**:
  - Phase 0 でも同様の指摘があり、見送りとした
  - CLAUDE.md に Windows 対応は明示的な要件として記載されていない
  - v0.1.0 のターゲットプラットフォームは macOS/Linux を想定

### 追加テスト
- `test_parse_status_quoted_path_with_space_returns_unquoted_path`
- `test_parse_status_quoted_rename_returns_unquoted_paths`
- `test_unquote_path_with_escaped_characters`
- `test_unquote_path_without_quotes_returns_as_is`
