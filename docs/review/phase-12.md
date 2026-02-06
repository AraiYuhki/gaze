# Phase 12 Review Log

## Summary
- 指摘件数: 1
- 修正対応: 1
- 見送り: 0

## 変更概要

Phase 12 は大規模リポジトリ（367K ファイル / 276 ブランチ規模）での操作遅延を解消するための
パフォーマンス改善を 3 つのサブタスクで実施した。

| サブタスク | 対象ファイル | 改善内容 |
|---|---|---|
| 12-2 | `app.rs` | `git log` から `--all` 除去 |
| 12-1 | `app.rs` | stage / unstage / discard で楽観的キャッシュ更新 |
| 12-3 | `app.rs`, `main.rs` | 起動時の二段階 status 読み込み |

### 期待効果
- stage / unstage / discard: ~4.2s → ほぼ 0s
- Log View 切替: ~3.2s → ~80ms
- 起動体感: ~4.2s → ~1.2s

## レビュー観点

### 1. CLAUDE.md の制約遵守
- Git 操作はすべて `git` CLI 経由（`std::process::Command` を直接使用）: **違反なし**
- `git status` 取得タイミングの制約:
  - `toggle_stage()`, `discard_changes()`, `stage_selected_hunk()` は `refresh_status()` を呼ばなくなったが、
    git コマンド自体は依然として実行後にのみキャッシュ更新しており、制約の趣旨（無駄な `git status` 抑制）に合致
  - View 切替、R キー、commit/stash/branch 操作では従来通り `refresh_status()` を維持: **適切**

### 2. Git コマンドの実行頻度
- stage/unstage/discard で `git status` が不要になったため、これらの操作あたり 1 回の
  git コマンド削減（`git add` or `git restore --staged` のみ）
- バックグラウンドスレッドの `git status` は起動時の 1 回限り: **適切**

### 3. 描画ループ内の重い処理
- `check_background_status()` は `try_recv()` を使用（ノンブロッキング）: **問題なし**
- `event::poll(100ms)` により描画ループは最大 100ms ブロック: **許容範囲**
  - 以前は `event::read()` で無期限ブロックだったため、バックグラウンド結果の
    反映が不可能だった。100ms の poll に変更することで非キー入力時にも
    ループが回り、バックグラウンド結果を反映可能になった

### 4. 責務分離
- バックグラウンドスレッドは `GitCli` を渡さず `std::process::Command` を直接使用:
  `GitCli` はスレッドセーフでないため正しい判断
- `parse_status()` はスレッド内でも呼び出し可能（純粋関数）: **適切**
- 楽観的更新のロジックは `app.rs` 内の private メソッドに集約: **適切**

### 5. 大規模リポジトリでの性能
- `git status -uno` により起動時の初回 status 取得が高速化
- `git log --all` 除去により全ブランチのトラバースが不要に
- 楽観的更新により操作ごとの `git status` 再実行が不要に
- いずれも大規模リポジトリで最も時間のかかる処理を削減: **効果的**

## Issues & Fixes

### 1. `stage_selected_hunk()` で `bg_status_receiver = None` が欠落していた

- 指摘内容:
  `stage_selected_hunk()` は楽観的に `status_cache` を更新しているが、
  `self.bg_status_receiver = None` を設定していなかった。
  バックグラウンドスレッドの結果が後から到着した場合、楽観的に更新した
  キャッシュが上書きされるレースコンディションが発生し得る。
  他の楽観的更新メソッド（`optimistic_update_after_stage`, `optimistic_update_unstage`,
  `discard_changes`）では正しく設定されていたため、`stage_selected_hunk` のみの漏れ。

- 対応:
  `stage_selected_hunk()` 内の楽観的キャッシュ更新箇所（diff 空の場合と
  diff 残存の場合の両方）に `self.bg_status_receiver = None` を追加した。

- 影響:
  起動直後に hunk ステージングを行った場合のレースコンディションが解消された。

## テスト確認

- 新規テスト 7 件を追加（楽観的更新の各パターン）
- 全 60 テストがパス
- `cargo clippy -- -D warnings`: 警告なし
- `cargo fmt --check`: 差分なし

## Renamed / Copied の楽観的更新について

Renamed / Copied ファイルの unstage は、パスの変更を伴う可能性があるため
楽観的更新が困難である。このため `optimistic_update_unstage()` 内で
`refresh_status()` にフォールバックする設計とした。
Renamed / Copied は使用頻度が低いため、このフォールバックによる性能影響は限定的。

## ROADMAP.md の変更

- 12-1, 12-2, 12-3 を完了マーク
- 12-4（バックグラウンド status の完全非同期化）を Phase 13 として分離
  - 理由: イベントループの設計変更を伴う大きな変更であり、Phase 12 のスコープとは独立

---

# Phase 12 二次レビュー（Codex CLI）

## Summary
- 指摘件数: 4
- 要修正: 1
- 情報: 3

## Issues

### 1. `check_background_status()` で `status_cache` が空になった場合の `selected_index` 未調整

- 指摘内容:
  `check_background_status()` 内で `self.status_cache = statuses;` の後、
  `status_cache` が空でない場合に `selected_index` を調整しているが、
  空になった場合（バックグラウンド status が 0 件を返す＝全変更が解消された状態）には
  `selected_index` が 0 にリセットされない。
  同様のロジックは `discard_changes()` では空チェック付きで `selected_index = 0` を設定しているが、
  `check_background_status()` にはこのパスが欠落している。
  実害は限定的（空リストでは選択が描画されないため）だが、一貫性の観点から修正が望ましい。
- 重要度: 低
- 推奨対応:
  `check_background_status()` 内の `selected_index` 範囲外チェックに
  `else if self.status_cache.is_empty() { self.selected_index = 0; }` を追加する。

### 2. `optimistic_update_after_stage()` の `_` キャッチオール分岐の意味が不明確

- 指摘内容:
  `optimistic_update_after_stage()` の `match file.worktree` に `_ => { entry.index = file.worktree; ... }` がある。
  この分岐には `Renamed`, `Copied`, `Ignored`, `Unmodified` が入り得る。
  `file.worktree` が `Unmodified` の場合、`entry.index = Unmodified` かつ `entry.worktree = Unmodified`
  となり、エントリが削除される。`toggle_stage()` の呼び出しフロー上、`worktree == Unmodified`
  でこの関数に到達するのは、index が Unmodified/Untracked かつ worktree が Unmodified のファイルを
  `git add` した場合のみ（実質的に変更がないファイル）で、正常系では起きにくいが、
  楽観的更新のキャッシュが古い場合に意図しない削除が発生する可能性がある。
  ただし、R キーや View 切替で `refresh_status()` が呼ばれるため自己修復される。
- 重要度: 低
- 推奨対応:
  現時点では実害が極めて限定的であり、修正不要。
  次回リファクタリング時に `_` 分岐にコメントで「到達条件と影響」を記載することを推奨。

### 3. `spawn_background_full_status()` のスレッドがパニックした場合のハンドリング

- 指摘内容:
  `std::thread::spawn` 内で `parse_status()` がパニックした場合、スレッドは異常終了し、
  `sender` がドロップされる。この場合 `receiver.try_recv()` は `Err(TryRecvError::Disconnected)` を
  返し、`.ok()` で `None` に変換されるため、アプリケーション本体はクラッシュしない。
  ただし `bg_status_receiver` が永久に `Some` のまま残り、毎ループ無駄な `try_recv()` が走り続ける。
  `try_recv()` は `Err(Disconnected)` を返し `.ok()` で `None` になるので `receiver` は破棄されない。
- 重要度: 中
- 推奨対応:
  `check_background_status()` で `try_recv()` が `Err(Disconnected)` を返した場合にも
  `bg_status_receiver = None` を設定するよう修正する。具体的には、`try_recv()` の結果を
  `Ok` / `Err(Empty)` / `Err(Disconnected)` で場合分けし、`Disconnected` でも receiver を破棄する。
  毎ループの無駄な呼び出しを回避でき、コードの意図も明確になる。

### 4. イベントループで `check_background_status()` が描画の後にある配置について

- 指摘内容:
  現在のループ構造は `draw() -> check_background_status() -> poll() -> read()` の順。
  バックグラウンド結果が到着した場合、`check_background_status()` でキャッシュは更新されるが、
  画面への反映はその次の `draw()` 呼び出し（次ループ先頭）まで遅延する。
  `poll(100ms)` によりイベントがなくても最大 100ms 後に次の描画が行われるため、
  ユーザー体感上は問題にならない。ただし、描画の直前に `check_background_status()` を
  配置した方が論理的に明快で、結果到着から画面反映までの遅延が確実に 0 になる。
- 重要度: 低
- 推奨対応:
  情報のみ。現在の実装でも最大 100ms の遅延で反映されるため、実用上は問題なし。
  将来リファクタリングする際に `draw()` の直前に移動することを検討してもよい。

## 一次レビューの検証

一次レビューで指摘・修正された `stage_selected_hunk()` の `bg_status_receiver = None` 追加を
確認した。`stage_selected_hunk()` の 2 箇所（diff 空の場合: 1773行目、diff 残存の場合: 1787行目）に
正しく設定されており、一次レビューの対応は完了している。

## CLAUDE.md 制約遵守の確認

| 制約 | 遵守状況 |
|------|----------|
| Git 操作は CLI 経由のみ | 遵守。`std::process::Command` を直接使用 |
| Git ライブラリ禁止 | 遵守。`git2`/`gitoxide` 等の使用なし |
| `tokio`/`async-std` 禁止 | 遵守。`std::thread` + `mpsc` は標準ライブラリでありこの制約に該当しない |
| Status 取得タイミング | 遵守。楽観的更新は `git status` を省略しており制約の趣旨に合致 |
| 描画ループ内の重い処理 | 遵守。`try_recv()` はノンブロッキング |
| Tree の遅延ロード | 変更なし |

## 総合判定

Phase 12 の変更は CLAUDE.md の制約を遵守しており、パフォーマンス改善の目的を適切に達成している。
Issue #3（`Disconnected` 時の receiver 未破棄）は軽微なリソースリークに相当するため修正を推奨するが、
バックグラウンド status は起動時に 1 回しか発生せず、かつ `parse_status` がパニックする可能性は
極めて低いため、ブロッカーではない。

**次 Phase に進んでよい。** ~~Issue #3 は次 Phase の冒頭で対応することを推奨する。~~

---

# 二次レビュー指摘への対応

## 対応済み

### Issue #1 + #3: `check_background_status()` の改善
- `try_recv()` の結果を `Ok` / `Err(Empty)` / `Err(Disconnected)` で場合分けし、
  `Disconnected` でも `bg_status_receiver = None` を設定するよう修正（Issue #3）
- `status_cache` が空の場合に `selected_index = 0` を設定するよう修正（Issue #1）

## 見送り

### Issue #2: `optimistic_update_after_stage()` の `_` キャッチオール
- 二次レビューでも「修正不要」判定。実害が限定的かつ自己修復されるため見送り。

### Issue #4: `check_background_status()` の配置
- 二次レビューでも「情報のみ」判定。最大 100ms の遅延は実用上問題なし。
