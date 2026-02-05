# 進捗管理ファイル

> このファイルはタスク完了時に Claude Code が更新する。
> 各タスク完了時に `[x]` に変更し、完了日時をコメントで記録すること。

---

## 重要ルール

### Phase 間の制約
- **未完了の Phase を跨いだリファクタリングは禁止**
- 現在の Phase の完了条件を満たすための最小実装のみ行うこと
- 「将来のため」の先行実装は禁止

### 判断に迷う場合の優先順位
1. 既存の制約を優先（CLAUDE.md の禁止事項）
2. パフォーマンスを優先（大規模リポジトリ対応）
3. それでも不明な場合は実装せず、理由をコメントで残す

**禁止**: 「より良い設計」「将来拡張」を理由に CLAUDE.md に記載のない変更を行うこと

### TODO コメントのルール
- 次 Phase で対応予定のもの → OK
- 性能影響が不明で見送ったもの → OK
- 理由・Phase の記載がないもの → NG

### レビュー必須
- 各 Phase 完了時にレビューを実施
  - **一次レビュー**: Claude Code（自己レビュー）
  - **二次レビュー**: Claude Code（第三者視点、別セッション推奨）
- `docs/review/phase-N.md` にレビューログを出力
- レビュー完了後にのみ次 Phase に進む

---

## 現在のフェーズ

**Phase 0: 基盤構築** ← 開始位置

---

## Phase 0: 基盤構築

### 0-1: プロジェクト初期化
- [x] `cargo new git-tui` 実行 <!-- 2026-02-05 -->
- [x] Cargo.toml に依存関係を追加（CLAUDE.md 参照） <!-- 2026-02-05 -->
- [x] ディレクトリ構造を作成 <!-- 2026-02-05 -->

**検証コマンド**:
```bash
cargo build
```

### 0-2: エラー型定義
- [x] `src/error.rs` を作成 <!-- 2026-02-05 -->
- [x] `AppError` enum を実装 <!-- 2026-02-05 -->
- [x] `pub type Result<T>` を定義 <!-- 2026-02-05 -->

**検証コマンド**:
```bash
cargo build
```

### 0-3: Git CLI 実行基盤
- [x] `src/cli/mod.rs` を作成 <!-- 2026-02-05 -->
- [x] `src/cli/executor.rs` に `GitCli` を実装 <!-- 2026-02-05 -->
- [x] `GitCli::new()` - リポジトリ検出 <!-- 2026-02-05 -->
- [x] `GitCli::execute()` - コマンド実行（同期） <!-- 2026-02-05 -->
- [x] `GitCli::repo_root()` - ルートパス取得 <!-- 2026-02-05 -->

**検証コマンド**:
```bash
cargo test
# git リポジトリ内で GitCli::new() が成功することを確認
```

### 0-4: TUI 初期化
- [x] `src/main.rs` にターミナル初期化を実装 <!-- 2026-02-05 -->
- [x] パニックハンドラを設置 <!-- 2026-02-05 -->
- [x] 空のイベントループを実装 <!-- 2026-02-05 -->
- [x] `q` キーで終了できることを確認 <!-- 2026-02-05 -->

**検証コマンド**:
```bash
cargo run
# q キーで正常終了することを確認
```

### 0-5: 外部ページャ基盤
- [x] `src/pager.rs` を作成 <!-- 2026-02-05 -->
- [x] `$PAGER` / `$GIT_PAGER` 環境変数対応 <!-- 2026-02-05 -->
- [x] OS別フォールバック（less / more） <!-- 2026-02-05 -->

**検証コマンド**:
```bash
cargo test
```

### Phase 0 完了判定
```bash
cargo build && cargo clippy -- -D warnings && cargo test
./target/debug/git-tui  # 起動して q で終了
```
- [x] **Phase 0 完了** <!-- 完了日時: 2026-02-05 -->
- [x] **Phase 0 レビュー完了** - `docs/review/phase-0.md` 作成済み <!-- 2026-02-05 -->

---

## Phase 1: Status View

### 1-1: ドメインモデル
- [x] `src/domain/mod.rs` を作成 <!-- 2026-02-05 -->
- [x] `src/domain/status.rs` に `StatusKind`, `FileStatus` を実装 <!-- 2026-02-05 -->

**検証コマンド**:
```bash
cargo build
```

### 1-2: Status パーサー
- [x] `src/cli/parser.rs` を作成 <!-- 2026-02-05 -->
- [x] `parse_status()` 関数を実装 <!-- 2026-02-05 -->
- [x] パーサーの単体テストを追加 <!-- 2026-02-05 -->
  - [x] Modified のテスト <!-- 2026-02-05 -->
  - [x] Added のテスト <!-- 2026-02-05 -->
  - [x] Deleted のテスト <!-- 2026-02-05 -->
  - [x] Renamed のテスト <!-- 2026-02-05 -->
  - [x] Untracked のテスト <!-- 2026-02-05 -->

**検証コマンド**:
```bash
cargo test parser
```

### 1-3: AppState
- [x] `src/app.rs` に `AppState` を実装 <!-- 2026-02-05 -->
- [x] 現在の View（Status/Tree/Log）を保持 <!-- 2026-02-05 -->
- [x] Status View の選択状態を保持 <!-- 2026-02-05 -->
- [x] `status_cache: Vec<FileStatus>` を保持 <!-- 2026-02-05 -->
- [x] `refresh_status()` メソッドを実装 <!-- 2026-02-05 -->

**検証コマンド**:
```bash
cargo build
```

### 1-4: Status View UI
- [x] `src/ui/mod.rs` を作成 <!-- 2026-02-05 -->
- [x] `src/ui/status_view.rs` を実装 <!-- 2026-02-05 -->
- [x] ファイル一覧表示 <!-- 2026-02-05 -->
- [x] ステータスの色分け（M=黄, A=緑, D=赤, ?=灰） <!-- 2026-02-05 -->
- [x] カーソル表示 <!-- 2026-02-05 -->

**検証コマンド**:
```bash
cargo run
# ファイルを変更して一覧に表示されることを確認
```

### 1-5: ナビゲーション
- [x] j/k でカーソル移動 <!-- 2026-02-05 -->
- [x] g で先頭へ <!-- 2026-02-05 -->
- [x] G で末尾へ <!-- 2026-02-05 -->

**検証コマンド**:
```bash
cargo run
# j/k でカーソル移動を確認
```

### 1-6: ステージング操作
- [x] s キーでステージ/アンステージ切替 <!-- 2026-02-05 -->
- [x] `git add <file>` 実行 <!-- 2026-02-05 -->
- [x] `git restore --staged <file>` 実行 <!-- 2026-02-05 -->
- [x] 操作完了後に status を1回だけ再取得 <!-- 2026-02-05 -->

**検証コマンド**:
```bash
cargo run
# s キーでステージング状態が変わることを確認
```

**禁止事項の確認**:
- [x] 描画ループ内で git status を実行していないこと <!-- 2026-02-05 -->
- [x] キー入力毎に git status を実行していないこと <!-- 2026-02-05 -->

### 1-7: 差分表示
- [x] d キーで差分表示 <!-- 2026-02-05 -->
- [x] `git diff <file>` の出力を取得 <!-- 2026-02-05 -->
- [x] 外部ページャで表示 <!-- 2026-02-05 -->
- [x] ページャ失敗時はエラー表示（アプリは継続） <!-- 2026-02-05 -->

**検証コマンド**:
```bash
cargo run
# d キーで差分が表示されることを確認
```

### 1-8: 変更破棄
- [x] r キーで変更破棄 <!-- 2026-02-05 -->
- [x] 確認ダイアログを表示（y/n） <!-- 2026-02-05 -->
- [x] `git restore <file>` 実行 <!-- 2026-02-05 -->
- [x] 操作完了後に status を1回だけ再取得 <!-- 2026-02-05 -->

**検証コマンド**:
```bash
cargo run
# r キーで確認後に変更が破棄されることを確認
```

### 1-9: リフレッシュ
- [x] R キーで手動リフレッシュ <!-- 2026-02-05 -->
- [x] ステータスバーに「Refreshed」表示 <!-- 2026-02-05 -->

**検証コマンド**:
```bash
cargo run
# R キーでリフレッシュされることを確認
```

### Phase 1 完了判定
```bash
cargo test && cargo clippy -- -D warnings
```
- [x] **Phase 1 完了** <!-- 完了日時: 2026-02-05 -->
- [x] **Phase 1 レビュー完了** - `docs/review/phase-1.md` 作成済み <!-- 2026-02-05 -->

---

## Phase 2: Tree View

### 2-1: TreeNode ドメインモデル
- [ ] `src/domain/tree.rs` を作成
- [ ] `NodeKind` enum を実装
- [ ] `TreeNode` 構造体を実装
- [ ] `children: Option<Vec<TreeNode>>` で遅延ロード対応

**検証コマンド**:
```bash
cargo build
```

### 2-2: 遅延ロード実装
- [ ] `TreeNode::new_dir()` - 子は None で初期化
- [ ] `TreeNode::load_children()` - 展開時のみ読み込み
- [ ] `.git` ディレクトリを除外
- [ ] ディレクトリ優先ソート

**検証コマンド**:
```bash
cargo test tree
```

**禁止事項の確認**:
- [ ] 初期化時に再帰的走査をしていないことを確認

### 2-3: Tree View UI
- [ ] `src/ui/tree_view.rs` を作成
- [ ] ツリー表示（インデント付き）
- [ ] 展開アイコン（▼ = 展開, ▶ = 折りたたみ/未ロード）
- [ ] Git ステータスインジケータ `[M]`, `[A]`, `[?]`（キャッシュから取得）

**検証コマンド**:
```bash
cargo run
# 2 キーで Tree View に切り替え
```

**禁止事項の確認**:
- [ ] Tree View 表示時に git status を実行していないこと（キャッシュ参照のみ）

### 2-4: Tree ナビゲーション
- [ ] j/k でカーソル移動
- [ ] Enter/l で展開（load_children + キャッシュから status 適用）
- [ ] h で折りたたみ
- [ ] Enter で折りたたんだディレクトリを展開

**検証コマンド**:
```bash
cargo run
# 展開操作を確認
```

**禁止事項の確認**:
- [ ] load_children 内で git コマンドを実行していないこと

### 2-5: 表示フィルタ
- [ ] `src/filter/mod.rs` を作成
- [ ] `src/filter/ignore.rs` に `DisplayFilter` を実装
- [ ] `~/.config/git-tui/display_ignore` から読み込み
- [ ] H キーでフィルタ表示切替

**検証コマンド**:
```bash
cargo run
# H キーでフィルタ切替を確認
```

### Phase 2 完了判定
```bash
cargo test && cargo clippy -- -D warnings
```
**パフォーマンス確認**:
- [ ] 大規模リポジトリ（node_modules 等）で起動が高速
- [ ] **Phase 2 完了** <!-- 完了日時: -->
- [ ] **Phase 2 レビュー完了** - `docs/review/phase-2.md` 作成済み

---

## Phase 3: Log View

### 3-1: GraphLine ドメインモデル
- [ ] `src/domain/log.rs` を作成
- [ ] `GraphLine` 構造体を実装
- [ ] `raw_line` フィールドでフォールバック対応

**検証コマンド**:
```bash
cargo build
```

### 3-2: Log パーサー
- [ ] `parse_log_line()` 関数を実装
- [ ] グラフ文字の抽出（構造解析はしない）
- [ ] ハッシュの抽出
- [ ] refs（ブランチ名、タグ）の抽出
- [ ] メッセージの抽出
- [ ] パース失敗時は raw_line を返す

**検証コマンド**:
```bash
cargo test log
```

**禁止事項の確認**:
- [ ] グラフ構造（親子関係）を解析していないことを確認

### 3-3: Log View UI
- [ ] `src/ui/log_view.rs` を作成
- [ ] `git log --oneline --graph --all -n 200` の表示
- [ ] グラフ文字の色分け
- [ ] ブランチ名のハイライト
- [ ] スクロール対応

**検証コマンド**:
```bash
cargo run
# 3 キーで Log View に切り替え
```

### 3-4: Log 操作
- [ ] Enter でコミット詳細表示（`git show <hash>`）
- [ ] 外部ページャで表示
- [ ] c でチェックアウト（確認ダイアログ必須）

**検証コマンド**:
```bash
cargo run
# Enter でコミット詳細を確認
```

### Phase 3 完了判定
```bash
cargo test && cargo clippy -- -D warnings
```
- [ ] **Phase 3 完了** <!-- 完了日時: -->
- [ ] **Phase 3 レビュー完了** - `docs/review/phase-3.md` 作成済み

---

## Phase 4: 統合と仕上げ

### 4-1: View 切り替え
- [ ] 1 キーで Status View
- [ ] 2 キーで Tree View
- [ ] 3 キーで Log View
- [ ] Tab キーで順次切り替え

**検証コマンド**:
```bash
cargo run
# 1, 2, 3 キーで切り替えを確認
```

### 4-2: 設定ファイル
- [ ] `src/config/mod.rs` を作成
- [ ] `src/config/settings.rs` を実装
- [ ] `~/.config/git-tui/config.toml` から読み込み
- [ ] ページャ設定の反映

**検証コマンド**:
```bash
cargo run
# 設定ファイルの反映を確認
```

### 4-3: ヘルプ画面
- [ ] ? キーでヘルプ画面表示
- [ ] キーバインド一覧を表示
- [ ] 任意のキーで閉じる

**検証コマンド**:
```bash
cargo run
# ? キーでヘルプを確認
```

### 4-4: ドキュメント
- [ ] README.md を作成
  - [ ] プロジェクト概要
  - [ ] インストール方法
  - [ ] 使用方法
  - [ ] キーバインド一覧
  - [ ] 設定ファイルの説明
- [ ] CHANGELOG.md を作成

### 4-5: 最終品質チェック
- [ ] `cargo build --release`
- [ ] `cargo clippy -- -D warnings`
- [ ] `cargo test`
- [ ] `cargo fmt --check`

### Phase 4 完了判定
```bash
cargo build --release && cargo clippy -- -D warnings && cargo test && cargo fmt --check
```
- [ ] **Phase 4 完了** <!-- 完了日時: -->
- [ ] **Phase 4 レビュー完了** - `docs/review/phase-4.md` 作成済み

---

## v0.1.0 リリース判定

すべての Phase が完了し、以下が確認できたらリリース:

- [ ] Phase 0 完了 + レビュー完了
- [ ] Phase 1 完了 + レビュー完了
- [ ] Phase 2 完了 + レビュー完了
- [ ] Phase 3 完了 + レビュー完了
- [ ] Phase 4 完了 + レビュー完了
- [ ] git リポジトリ内で全機能が動作
- [ ] git リポジトリ外で適切なエラー表示
- [ ] README.md が完備
- [ ] `docs/review/` に全 Phase のレビューログが存在

**v0.1.0 リリース日**: <!-- 日時を記入 -->

---

## 延期した機能（v0.2.0 以降）

以下は v0.1.0 のスコープ外。実装しないこと。

- Log の動的読み込み（スクロール時に追加取得）
- 特定ファイルのログ表示
- 非同期化（tokio 導入）
- Stash 対応
- ブランチ操作
- カスタムキーバインド

---

## メモ欄

<!-- 
セッション間で引き継ぐべき情報をここに記録:
- 発生した問題と解決策
- 設計判断の理由
- 次回セッションで最初にやるべきこと
-->
