# Phase 2 Review Log

## Summary
- 一次レビュー指摘件数: 3（修正: 2, 見送り: 1）
- 二次レビュー（Codex CLI）指摘件数: 3（全て修正済み）

## Issues & Fixes

### 1. 未使用パラメータの削除

- 指摘内容:
  `apply_status_to_tree` メソッド内の `apply_status_recursive` が `_node` パラメータを受け取りながら使用していなかった。不要なコードの臭いがあった。

- 対応:
  `apply_status_recursive` メソッドを削除し、`apply_status_to_tree` 内にローカル関数として統合。不要なパラメータを完全に除去した。

- 影響:
  コードの可読性と保守性が向上。

### 2. borrow checker 対策での status_cache クローン

- 指摘内容:
  `expand_tree_node` および `toggle_tree_node` メソッドで、borrow checker 対策として `self.status_cache.clone()` を行っている。大規模リポジトリでステータスエントリが多い場合、クローンのコストが性能に影響する可能性がある。

- 対応:
  現段階では見送り。理由: `status_cache` は通常数百〜数千件程度であり、`Vec<FileStatus>` のクローンコストは許容範囲内。Rust の所有権システムの制約上、この対策は妥当。

- 影響:
  なし。将来的に性能問題が顕在化した場合は、アーキテクチャレベルの見直しを検討。

### 3. load_children 内での git コマンド実行禁止の遵守

- 指摘内容:
  CLAUDE.md の制約「load_children 内で git コマンドを実行しないこと」が遵守されているかを確認。

- 対応:
  確認完了。`load_children` はファイルシステムの読み取りのみを行い、git コマンドは一切実行していない。ステータスの適用は `apply_status_cache` メソッドを通じてキャッシュから行われている。

- 影響:
  制約遵守を確認。

## 見送り項目

### 1. status_cache のクローンコスト最適化

- 見送り理由:
  現時点では性能影響が小さいと判断。Rust の所有権システムの制約により、`&mut self` と `&self.status_cache` を同時に借用することは不可能であり、クローンは合理的な解決策。将来的に問題が顕在化した場合は、`Rc<RefCell<>>` やアーキテクチャ変更を検討する。

## 二次レビュー（Codex CLI）指摘事項

### 4. パスマッチングの曖昧性 [P2]

- 指摘内容:
  `apply_status_cache` で `Path::ends_with` を使用しているため、2つのファイルが同じ末尾コンポーネントを共有している場合（例: `a/b/file.txt` と `b/file.txt`）に誤ったステータスが適用される可能性がある。

- 対応:
  双方向の `ends_with` チェック（`||` で結合）を削除し、`child.path.ends_with(&s.path)` のみに変更。リポジトリ相対パスとの比較を明確化。

- 影響:
  ステータス表示の正確性が向上。

### 5. 古いステータスのクリア漏れ [P2]

- 指摘内容:
  ファイルがクリーンな状態に戻った場合、`status_cache` から消えるが、`apply_status_cache` は一致するもののみを設定し、不一致のものはリセットしない。これにより古い `[M]/[A]/[?]` インジケータが残る。

- 対応:
  `apply_status_cache` の各子ノードのループの最初に `child.git_status = None` でリセットを追加。

- 影響:
  ファイルがクリーンになった際にステータスインジケータが正しく消える。

### 6. Tree View でのリフレッシュ後のステータス未更新 [P2]

- 指摘内容:
  Tree View で `R` キーを押すと `status_cache` のみが更新され、`tree_root` にステータスが再適用されない。ユーザーはリフレッシュ後も古いマーカーを見ることになる。

- 対応:
  `handle_tree_view_keys` の `R` キー処理で、`refresh_status()` 成功後に `refresh_tree_status()` を呼び出すように修正。`refresh_tree_status()` は `apply_status_to_tree()` を呼び出す公開メソッドとして `AppState` に追加。

- 影響:
  Tree View での手動リフレッシュ後にステータスインジケータが正しく更新される。

## 禁止事項の遵守確認

| 禁止事項 | 状態 |
|---------|------|
| 初期化時の再帰的走査 | ✅ 遵守（ルート直下のみ load） |
| load_children 内での git 実行 | ✅ 遵守 |
| Tree View 表示時の git status 実行 | ✅ 遵守（キャッシュ参照のみ） |
| 描画ループ内での git status 実行 | ✅ 遵守 |

## テスト結果

```
running 25 tests
test cli::parser::tests::test_parse_status_added_in_index_returns_added_kind ... ok
test cli::parser::tests::test_parse_status_deleted_in_worktree_returns_deleted_kind ... ok
...
test domain::tree::tests::test_new_dir_creates_unloaded_directory ... ok
test domain::tree::tests::test_new_file_creates_file_with_empty_children ... ok
test domain::tree::tests::test_is_loaded_returns_false_for_unloaded_directory ... ok
test domain::tree::tests::test_is_loaded_returns_true_for_file ... ok
test filter::ignore::tests::test_display_filter_load_returns_filter ... ok
test filter::ignore::tests::test_display_filter_toggle_changes_enabled_state ... ok
test filter::ignore::tests::test_should_hide_returns_false_when_disabled ... ok
test filter::ignore::tests::test_should_hide_returns_true_when_pattern_matches ... ok
...
test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Phase 2 完了確認

- [x] cargo build 成功
- [x] cargo clippy -- -D warnings 警告なし
- [x] cargo test 全パス
- [x] cargo fmt --check 差分なし
- [x] Tree View が 2 キーで表示される
- [x] j/k でカーソル移動が動作する
- [x] Enter/l で展開、h で折りたたみが動作する
- [x] H キーでフィルタ切替が動作する
- [x] Git ステータスインジケータが正しく表示される
