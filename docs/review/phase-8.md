# Phase 8 Review Log

## Summary
- 指摘件数: 3
- 修正対応: 3
- 見送り: 0

## 変更内容

### 追加されたファイル
1. `src/domain/branch.rs` - ブランチエントリのドメインモデル
2. `src/ui/branch_view.rs` - Branch View の描画ロジック

### 変更されたファイル
1. `src/domain/mod.rs` - BranchEntry のエクスポート追加
2. `src/cli/parser.rs` - branch パーサー（`parse_branch_list`）の追加、テスト追加
3. `src/cli/mod.rs` - `parse_branch_list` のエクスポート追加
4. `src/ui/mod.rs` - `branch_view` モジュールの追加
5. `src/ui/help_view.rs` - Branch View のヘルプ追加
6. `src/app.rs` - Branch 関連の状態とメソッド追加（View enum, ConfirmDialog, BranchInputMode, AppState フィールド・メソッド）
7. `src/main.rs` - Branch View のキーバインドと描画処理追加

## Issues & Fixes

### 1. 未使用の `from_raw` 関数（一次レビュー）
- 指摘内容:
  `BranchEntry::from_raw` 関数が定義されているが、現在のパーサーでは使用されていないため Clippy 警告が発生
- 対応:
  `#[allow(dead_code)]` を追加し、TODO コメントで将来のパース失敗時のフォールバックとして使用予定であることを明記
- 影響:
  警告が解消され、コードの意図が明確化

### 2. Branch View 切り替え時の status_cache 更新漏れ（二次レビュー: Codex CLI）
- 指摘内容:
  Branch View に切り替えた時に `status_cache` を更新していないため、ブランチチェックアウト時の uncommitted changes 検出が古い情報を使用する可能性がある
- 対応:
  `switch_view()` で `View::Branch` に切り替える際に `refresh_status()` を呼び出すように修正
- 影響:
  ブランチ切り替え前の未コミット変更検出が正確になった

### 3. non-origin リモートブランチ名の抽出不具合（二次レビュー: Codex CLI）
- 指摘内容:
  リモートブランチのチェックアウト時、ローカルブランチ名の抽出が `origin/` のみを strip しているため、`upstream/feature` のようなリモートの場合、ローカルブランチ名が `upstream/feature` になってしまう
- 対応:
  最初の `/` 以降をローカルブランチ名として抽出するように修正（`name.find('/').map_or(&name[..], |pos| &name[pos + 1..])）
- 影響:
  任意のリモート（origin, upstream, etc.）から正しいローカルブランチ名が抽出されるようになった

## CLAUDE.md 遵守確認

### 絶対原則
- [x] Git操作は `git` CLI コマンド経由（`git branch -a`, `git checkout`）
- [x] Gitライブラリ（libgit2, gitoxide等）は使用していない
- [x] submodule/LFS の解釈なし
- [x] `.gitignore` への変更なし

### Git コマンド実行タイミング
- [x] View 切り替え時（Branch View に切り替えた時）に1回のみ `git branch -a` を実行
- [x] 手動リフレッシュ時（R キー）に1回のみ実行
- [x] ブランチ切り替え操作後に1回のみ実行
- [x] 描画ループ内でのコマンド実行なし
- [x] キー入力毎のコマンド実行なし

### UI 描画
- [x] 描画ループ内に重い処理なし
- [x] キャッシュ（`branch_cache`）を参照して描画

### 責務分離
- [x] Domain: `BranchEntry` - データ構造のみ
- [x] CLI: `parse_branch_list` - パースロジックのみ
- [x] UI: `branch_view` - 描画ロジックのみ
- [x] App: `AppState` - 状態管理と操作

### パフォーマンス
- [x] ブランチ一覧はキャッシュから取得
- [x] 検索フィルタリングはキャッシュ上で実行（Git コマンドを再実行しない）
- [x] 大規模リポジトリでも影響なし（ブランチ数は通常限定的）

## 機能確認

### 実装された機能
1. Branch View の追加（キー `5` で切り替え）
2. ブランチ一覧の表示（ローカル/リモート区別）
3. 現在のブランチのハイライト表示
4. j/k でカーソル移動
5. g/G で先頭/末尾へ移動
6. `/` キーで検索フィルタリング
7. `Enter` キーでブランチ切り替え（確認ダイアログ付き）
8. リモートブランチのトラッキングチェックアウト対応
9. 未コミットの変更がある場合の警告
10. Tab キーで View サイクルに Branch を追加

## 結論

問題なし。一次レビュー（Clippy 警告）および二次レビュー（Codex CLI）で指摘された問題はすべて修正済み。CLAUDE.md の制約に準拠した実装。
