# Phase 11 Review Log

## Summary
- 指摘件数: 8
- 修正対応: 5
- 見送り: 3（理由を明記）

## Issues & Fixes

### 1. staged ファイルの diff 再取得モード不整合（Critical → 修正済み）
- 指摘内容:
  hunk ステージ後に diff を再取得する際、元の diff 取得モード（`--cached` か否か）を考慮せず常に `git diff`（cached なし）を使用していた。ステージ済みファイルに対して hunk モードに入った場合、hunk ステージ後の再取得で差分が見つからず、意図せず hunk モードが終了する可能性があった。
- 対応:
  `AppState` に `hunk_is_cached` フラグを追加し、`start_hunk_mode` で diff 取得モードを記録。`stage_selected_hunk` 内の diff 再取得時にも同じモードを使用するよう修正。
- 影響:
  ステージ済みファイルに対する hunk 単位操作が正しく動作するようになった。

### 2. Hunk 構造体の #[allow(dead_code)] コメント不正確（Minor → 修正済み）
- 指摘内容:
  「Phase 11-2 で使用予定」と記載されていたが、Phase 11-2 は実装済み。実際にはフィールドはパース結果の保持用で、パッチ生成では header 文字列を直接使用するため参照されない。
- 対応:
  コメントを実態に合わせて更新。
- 影響:
  コードの意図が正確に伝わるようになった。

### 3. extract_diff_file_path のパス解析が脆弱（Major → 修正済み）
- 指摘内容:
  `diff --git a/path b/path` 形式の行から `find(" b/")` で分割していたため、パス内に ` b/` を含むファイルで誤分割される可能性があった。
- 対応:
  `find` を `rfind` に変更し、末尾から ` b/` を探すことで、パス内に ` b/` が含まれるケースにも対応。
- 影響:
  特殊なファイル名を持つファイルの hunk ステージングが正しく動作するようになった。

### 4. hunk_view.rs にスクロール処理がない（Major → 修正済み）
- 指摘内容:
  `List` ウィジェットに `ListState` を渡していなかったため、diff が画面に収まりきらない場合に選択中の hunk ヘッダが画面外に表示され、視認できなかった。
- 対応:
  `ListState` を導入し、`hunk_selected_index` と連動させて `render_stateful_widget` で描画するよう変更。選択位置に自動スクロールされるようになった。
- 影響:
  長い diff でも選択中の hunk が常に画面内に表示されるようになった。

### 5. hunk_selected_index 初期値のコメント追加（Minor → 修正済み）
- 指摘内容:
  `hunk_selected_index = 1` の根拠が暗黙的だった（0 がファイルヘッダ、1 が最初の hunk ヘッダという前提）。
- 対応:
  コメントを追加して前提を明記。
- 影響:
  コードの意図が明確になった。

## 見送り項目

### 1. generate_patch の新規ファイル対応
- 見送り理由:
  新規ファイル（untracked）は `git diff` で差分が出ないため、そもそも hunk モードに入れない。`git diff --cached` で新規ファイルの diff が出るケースでは `new file mode` ヘッダが必要だが、現状の使用パターンでは問題にならない。将来的にファイル全体のステージング操作を拡張する場合に対応する。

### 2. stage_selected_hunk 内の git コマンド3回実行
- 見送り理由:
  `git apply --cached` → `git status` → `git diff` の3回実行は CLAUDE.md の「変更操作直後に1回だけ refresh_status」ルールに準拠しており、設計上正しい。パフォーマンスが問題になった場合のみ最適化を検討する。

### 3. Untracked ファイルに対する hunk モードのメッセージ改善
- 見送り理由:
  現在は "No diff available for this file" と表示される。動作としてはクラッシュせず正しいが、より具体的なメッセージ（"Hunk staging is not available for untracked files"）への変更は UX 改善であり、最小実装の原則に基づき見送り。

## CLAUDE.md 準拠チェック

| 制約事項 | 準拠状況 |
|---|---|
| Git CLI 経由のみ（ライブラリ不使用） | 準拠 |
| Status 取得タイミング | 準拠（操作直後に1回のみ） |
| 描画ループ内の重い処理 | 準拠（参照のみ） |
| Domain/UI/CLI 分離 | 準拠 |
| エラーハンドリング（unwrap 禁止） | 準拠 |
