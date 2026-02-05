# Phase 3 Review Log

## Summary
- 一次レビュー指摘件数: 2（軽微、修正対応済み）
- 二次レビュー（Codex CLI）指摘件数: 1（致命的、修正対応済み）

## Issues & Fixes

### 1. if_same_then_else の冗長なコード

- 指摘内容:
  `log_view.rs` で refs の色分け処理において、`tag:` で始まる場合と else の両方で同じ `Color::Green` を返していた。

- 対応:
  重複した分岐を削除し、ブランチ名とタグを同じ緑色で表示するように統合。コメントで意図を明確化。

- 影響:
  コードの可読性が向上。

### 2. manual_range_contains の警告

- 指摘内容:
  `parser.rs` のハッシュ長チェックで `hash_len >= 7 && hash_len <= 40` という手動の範囲チェックを使用していた。

- 対応:
  `(7..=40).contains(&hash_len)` に変更。

- 影響:
  Rust の慣用的な書き方に統一。

## 二次レビュー（Codex CLI）指摘

### 3. [P1] git log の -n 引数の誤り（致命的）

- 指摘内容:
  `refresh_log()` で `-n` オプションを単一の文字列 `"-n 200"` として渡していた。
  `GitCli::execute` は引数をシェル展開せず直接 argv として渡すため、git が `-n 200` を無効なオプションとして解釈し、コマンドが失敗していた。

- 対応:
  `-n` と `200` を別々の引数として渡すように変更:
  ```rust
  &[
      "log",
      "--oneline",
      "--graph",
      "--all",
      "-n",
      &LOG_LIMIT.to_string(),
  ]
  ```

- 影響:
  Log View がコミット履歴を正しく取得できるようになった。これは機能の根幹に関わる致命的なバグであり、Codex CLI のレビューで発見された。

## 禁止事項の遵守確認

| 禁止事項 | 状態 |
|---------|------|
| グラフ構造（親子関係）の解析 | ✅ 遵守（行単位のパースのみ） |
| コミットツリーの再構築 | ✅ 遵守 |
| パース失敗時のクラッシュ | ✅ 遵守（raw_line でフォールバック） |

## 実装の概要

### GraphLine ドメインモデル
- `raw_line`: 元の行（フォールバック用）
- `graph_chars`: グラフ部分（`*`, `|`, `/`, `\` など）
- `hash`: コミットハッシュ（7-40文字の16進数）
- `refs`: ブランチ名、タグのリスト
- `message`: コミットメッセージ

### Log パーサー
- `parse_log()`: 複数行をパース
- `parse_log_line()`: 1行をパース
- `find_graph_end()`: グラフ文字の終端を検出
- `extract_hash()`: ハッシュを抽出
- `extract_refs()`: 括弧内の refs を抽出

### Log View UI
- グラフ文字の色分け（`*`=黄, `|`=青, `/\`=緑）
- ハッシュの黄色表示
- refs の色分け（HEAD=シアン, origin=赤, その他=緑）
- スクロール対応（選択位置が画面中央付近に来るように）

### Log 操作
- `Enter`: コミット詳細を外部ページャで表示
- `c`: チェックアウト（確認ダイアログ付き）
- `R`: ログの手動リフレッシュ

## テスト結果

```
running 32 tests
test cli::parser::tests::test_parse_log_line_with_hash_refs_and_message ... ok
test cli::parser::tests::test_parse_log_line_without_refs ... ok
test cli::parser::tests::test_parse_log_line_graph_only ... ok
test cli::parser::tests::test_parse_log_line_with_branch_graph ... ok
test cli::parser::tests::test_parse_log_empty_returns_empty_vec ... ok
test cli::parser::tests::test_parse_log_multiple_lines ... ok
test domain::log::tests::test_from_raw_creates_graphline_with_raw_line ... ok
...
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Phase 3 完了確認

- [x] cargo build 成功
- [x] cargo clippy -- -D warnings 警告なし
- [x] cargo test 全パス（32件）
- [x] cargo fmt --check 差分なし
- [x] Log View が 3 キーで表示される
- [x] j/k でカーソル移動が動作する
- [x] Enter でコミット詳細が表示される
- [x] c キーでチェックアウト確認ダイアログが表示される
- [x] グラフ文字が色分けされている
- [x] ブランチ名がハイライトされている
