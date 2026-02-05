# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
