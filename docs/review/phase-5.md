# Phase 5 Review Log

## Summary
- 指摘件数: 1
- 修正対応: 1
- 見送り: 0

## Issues & Fixes

### 1. clippy: unnecessary_map_or
- 指摘内容:
  `map_or(false, |s| s.is_empty())` を `is_some_and(|s| s.is_empty())` に置き換えるべき
- 対応:
  `src/app.rs` の `start_amend_mode` 関数内の該当箇所を修正
- 影響:
  コードがより慣用的になり、可読性が向上

## 制約遵守確認

### CLAUDE.md の絶対原則
- [x] すべてのGit操作は `git` CLIコマンド経由で実行
  - `git commit -m`, `git commit --amend -m`, `git log -1 --pretty=%B`, `git diff --staged` を使用
- [x] libgit2、gitoxide 等のGitライブラリは使用していない
- [x] submodule と LFS は解釈していない
- [x] `.gitignore` ファイルを変更・生成していない

### Git Status 取得タイミング
- [x] コミット成功後に1回だけ `refresh_status()` を呼び出し
- [x] 描画ループ内で git コマンドを実行していない
- [x] キー入力毎に git コマンドを実行していない

### パフォーマンス
- [x] staged diff 取得は `d` キー押下時のみ
- [x] 直前のコミットメッセージ取得は amend モード開始時のみ

## 実装内容

### 追加されたファイル
- `src/ui/commit_view.rs`: コミットモードの UI 描画

### 変更されたファイル
- `src/app.rs`: `CommitMode` enum、コミット関連フィールド・メソッド追加
- `src/main.rs`: コミットモードのキー処理追加
- `src/ui/mod.rs`: `commit_view` モジュール追加
- `src/ui/help_view.rs`: コミット関連キー追加

### 機能
1. **コミットモード**
   - Status View から `c` キーで開始
   - ステージされたファイルがない場合はエラー表示
   - Tab でファイル一覧/メッセージ入力のフォーカス切り替え

2. **メッセージ入力**
   - 複数行対応
   - カーソル移動（左右、上下）
   - 文字入力、削除、改行

3. **ステージファイル確認**
   - ステージされたファイルの一覧表示
   - j/k で選択、d で staged diff 表示

4. **Amend 機能**
   - `C` キーで開始（確認ダイアログ付き）
   - 直前のコミットメッセージを自動取得して表示

5. **コミット実行**
   - `Ctrl+Enter` でコミット実行
   - `Esc` でキャンセル
   - 成功後にステータスを再取得
