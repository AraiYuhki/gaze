# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.0] - 2026-02-05

### Added

- **Branch View（Phase 8）**
  - `5` キーで Branch View に切り替え
  - ローカル/リモートブランチの一覧表示
  - 現在のブランチを `*` マークでハイライト
  - リモートブランチを `[remote]` タグ付きで表示
  - `/` キーでインクリメンタル検索（フィルタリング）
  - `Enter` キーでブランチをチェックアウト
  - リモートブランチのトラッキングチェックアウト対応
  - 未コミット変更がある場合の警告ダイアログ

## [0.5.0] - 2026-02-05

### Added

- **Stash View（Phase 7）**
  - `4` キーで Stash View に切り替え
  - stash 一覧の表示（ブランチ名、メッセージ付き）
  - `s` キーで現在の変更を stash に保存（メッセージ入力ダイアログ付き）
  - `p` キーで stash を適用して削除（pop）
  - `a` キーで stash を適用（削除せず）
  - `d` キーで stash を削除（確認ダイアログ付き）
  - `Enter` キーで stash の内容を表示（色付き差分）

## [0.4.0] - 2026-02-05

### Added

- **差分表示の色分け**
  - `git diff --color=always` による色付き差分表示
  - 追加行（緑）、削除行（赤）、ハンク情報（シアン）で色分け
  - Status View、コミットモード、Log View すべてに適用

### Changed

- デフォルトページャを `less -R` に変更（ANSI カラーコード対応）

## [0.3.0] - 2026-02-05

### Added

- **自己更新機能**
  - `--check-update`: 最新バージョンの確認
  - `--update`: 最新バージョンへの更新
  - `--version`, `-V`: バージョン表示
  - `--help`, `-h`: ヘルプ表示

## [0.2.1] - 2026-02-05

### Fixed

- コミット実行キーを `Ctrl+D` に変更（`Ctrl+Enter` はターミナルによって認識されないため）
- IME のプリエディット（変換中文字列）がコミットメッセージ入力位置に表示されるよう修正
- discard がステージ済みの変更にも対応するよう修正
- Windows でのページャ起動に対応（`cmd /c` を使用）

### Added

- シェルスクリプトインストーラー（macOS/Linux）
- Scoop マニフェスト（Windows）

## [0.2.0] - 2026-02-05

### Added

- **コミット機能**
  - Status View から `c` キーでコミットモード開始
  - 複数行メッセージ入力対応
  - ステージされたファイル一覧の表示
  - 選択ファイルの staged diff 表示（`d` キー）
  - Amend 機能（`C` キー、確認ダイアログ付き）

- **Tree View 検索機能**
  - Vim ライクなインクリメンタル検索（`/` キー）
  - 折りたたまれたディレクトリ内も検索対象
  - マッチ時に親ディレクトリを自動展開

- **CI/CD**
  - GitHub Actions でビルドとリリースを自動化
  - Linux / macOS (Intel, Apple Silicon) / Windows 向けバイナリを提供

### Fixed

- Unicode ファイル名でのハイライト表示の不具合を修正

### Changed

- 全ビューの選択行の背景色を統一し、視認性を向上

## [0.1.0] - 2026-02-05

### Added

- **Status View**: Git status の表示
  - 変更ファイルの一覧表示
  - ステージング/アンステージング操作
  - 差分表示（外部ページャ）
  - 変更破棄機能（確認ダイアログ付き）

- **Tree View**: ディレクトリツリー表示
  - 遅延ロードによる高速起動
  - Git ステータスの色分け表示
  - ディレクトリの展開/折りたたみ
  - 表示フィルタ機能
  - Vim ライクなインクリメンタル検索（折りたたみ内も検索可能）

- **Log View**: コミット履歴表示
  - グラフ形式のコミット履歴
  - ブランチ/タグのハイライト
  - コミット詳細表示
  - チェックアウト機能（確認ダイアログ付き）

- **共通機能**
  - View 切り替え（1, 2, 3 キー、Tab キー）
  - ヘルプ画面（? キー）
  - 設定ファイルのサポート
  - 外部ページャの設定

### Technical Notes

- すべての Git 操作は `git` CLI 経由で実行
- 大規模リポジトリでの性能を考慮した遅延ロード設計
