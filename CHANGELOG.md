# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
