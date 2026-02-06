# Gaze

軽量な Git 操作補助 TUI ツール

## 概要

Gaze は Git リポジトリの状態確認と基本操作を提供するターミナルユーザーインターフェースです。

- **Status View**: 変更されたファイルの一覧表示、ステージング/アンステージング、差分表示、Push/Pull
- **Hunk ステージング**: 差分を hunk 単位で表示し、個別にステージ
- **Tree View**: ディレクトリツリー形式でのファイル表示（Git ステータス付き）、インクリメンタル検索
- **Log View**: コミット履歴のグラフ表示、コミット詳細の確認
- **Stash View**: stash の一覧表示、保存、適用、削除
- **Branch View**: ブランチ一覧表示、検索、チェックアウト

## インストール

### シェルスクリプト（macOS / Linux）

```bash
curl -fsSL https://raw.githubusercontent.com/AraiYuhki/gaze/master/install.sh | sh
```

デフォルトでは `~/.local/bin` にインストールされます。インストール先を変更する場合:

```bash
INSTALL_DIR=/usr/local/bin curl -fsSL https://raw.githubusercontent.com/AraiYuhki/gaze/master/install.sh | sh
```

### Scoop（Windows）

```powershell
# バケットを追加（初回のみ）
scoop bucket add gaze https://github.com/AraiYuhki/gaze

# インストール
scoop install gaze
```

### GitHub Releases

[Releases](https://github.com/AraiYuhki/gaze/releases) から各プラットフォーム向けのバイナリをダウンロードできます。

### ソースからビルド

```bash
git clone https://github.com/AraiYuhki/gaze.git
cd gaze
cargo build --release
cp ./target/release/gaze ~/.local/bin/
```

## 使用方法

Git リポジトリ内で `gaze` を実行します:

```bash
cd /path/to/your/repo
gaze
```

## コマンドラインオプション

| オプション | 説明 |
|------------|------|
| `-h`, `--help` | ヘルプを表示 |
| `-V`, `--version` | バージョンを表示 |
| `--check-update` | 最新バージョンを確認 |
| `--update` | 最新バージョンに更新 |

### 更新の確認と実行

```bash
# 更新があるか確認
gaze --check-update

# 最新バージョンに更新
gaze --update
```

## キーバインド

### 共通

| キー | 動作 |
|------|------|
| `1` | Status View に切り替え |
| `2` | Tree View に切り替え |
| `3` | Log View に切り替え |
| `4` | Stash View に切り替え |
| `5` | Branch View に切り替え |
| `Tab` | 次の View に切り替え |
| `?` | ヘルプ画面を表示 |
| `q` | 終了 |
| `Ctrl+C` | 終了 |

### Status View

| キー | 動作 |
|------|------|
| `j` / `↓` | 下に移動 |
| `k` / `↑` | 上に移動 |
| `g` | 先頭に移動 |
| `G` | 末尾に移動 |
| `s` | ステージ/アンステージの切り替え |
| `d` | 差分を表示（外部ページャ） |
| `r` | 変更を破棄（確認ダイアログあり） |
| `c` | コミットモード開始 |
| `C` | Amend コミットモード開始（確認あり） |
| `H` | Hunk ステージングモード開始 |
| `P` | Push（確認ダイアログあり） |
| `U` | Pull（確認ダイアログあり） |
| `R` | リフレッシュ |

### コミットモード

| キー | 動作 |
|------|------|
| `Tab` | ファイル一覧/メッセージ入力の切り替え |
| `Ctrl+D` | コミット実行 |
| `Esc` | キャンセル |

#### メッセージ入力中

| キー | 動作 |
|------|------|
| 文字入力 | メッセージに追加 |
| `Enter` | 改行 |
| `Backspace` | 削除 |
| `←` / `→` | カーソル移動 |
| `↑` / `↓` | 行移動 |

#### ファイル一覧フォーカス中

| キー | 動作 |
|------|------|
| `j` / `k` | ファイル選択 |
| `d` | 選択ファイルの staged diff を表示 |

### Hunk ステージングモード

Status View で `H` キーを押すと、選択中のファイルの差分を hunk 単位で表示し、hunk / 行 / 範囲単位でステージできます。

| キー | 動作 |
|------|------|
| `j` / `↓` | 次の行に移動 |
| `k` / `↑` | 前の行に移動 |
| `s` | hunk ヘッダ上→hunk 全体をステージ / コンテンツ行上→1行ステージ / Visual モード→選択範囲ステージ |
| `v` | Visual モードの開始/解除 |
| `Esc` | Visual モード中→Visual 解除 / 通常時→hunk モード終了 |
| `q` | hunk モードを終了 |

#### Visual モード

`v` キーで Visual モードを開始し、`j`/`k` で選択範囲を拡大して `s` でまとめてステージできます。選択は同一 hunk 内に限定されます。

### Tree View

| キー | 動作 |
|------|------|
| `j` / `↓` | 下に移動 |
| `k` / `↑` | 上に移動 |
| `g` | 先頭に移動 |
| `G` | 末尾に移動 |
| `Enter` / `l` | ディレクトリを展開 |
| `h` | ディレクトリを折りたたむ |
| `H` | 表示フィルタの切り替え |
| `/` | 検索モード開始 |
| `n` | 次のマッチへ |
| `N` | 前のマッチへ |
| `R` | リフレッシュ |

#### 検索モード中

| キー | 動作 |
|------|------|
| 文字入力 | 検索文字列に追加 |
| `Backspace` | 末尾の文字を削除 |
| `Enter` | 検索確定 |
| `Esc` | 検索キャンセル |

検索機能は展開済みのディレクトリ内のファイルを対象とします。マッチした項目にジャンプすると、親ディレクトリが自動的に展開されます。

### Log View

| キー | 動作 |
|------|------|
| `j` / `↓` | 下に移動 |
| `k` / `↑` | 上に移動 |
| `g` | 先頭に移動 |
| `G` | 末尾に移動 |
| `Enter` | コミット詳細を表示（外部ページャ） |
| `c` | チェックアウト（確認ダイアログあり） |
| `R` | リフレッシュ |

### Stash View

| キー | 動作 |
|------|------|
| `j` / `↓` | 下に移動 |
| `k` / `↑` | 上に移動 |
| `g` | 先頭に移動 |
| `G` | 末尾に移動 |
| `s` | 現在の変更を stash に保存（メッセージ入力可） |
| `p` | 選択した stash を適用して削除（pop） |
| `a` | 選択した stash を適用（削除せず） |
| `d` | 選択した stash を削除（確認ダイアログあり） |
| `Enter` | stash の内容を表示（外部ページャ） |
| `R` | リフレッシュ |

### Branch View

| キー | 動作 |
|------|------|
| `j` / `↓` | 下に移動 |
| `k` / `↑` | 上に移動 |
| `g` | 先頭に移動 |
| `G` | 末尾に移動 |
| `/` | 検索モード開始 |
| `Esc` | 検索クリア |
| `Enter` | ブランチをチェックアウト（確認ダイアログあり） |
| `R` | リフレッシュ |

Branch View では、現在のブランチは `*` でマーク、リモートブランチは `[remote]` タグ付きで表示されます。未コミットの変更がある場合、チェックアウト前に警告が表示されます。

## 設定

設定ファイルは `~/.config/git-tui/config.toml` に配置します。

### 設定例

```toml
[pager]
# 外部ページャコマンド（デフォルト: $GIT_PAGER, $PAGER, または less）
command = "less -R"
```

### 表示フィルタ

Tree View で特定のファイル/ディレクトリを非表示にするには、`~/.config/git-tui/display_ignore` にパターンを記述します:

```
node_modules
.DS_Store
*.log
target
```

## 動作要件

- Git がインストールされていること
- Git リポジトリ内で実行すること

## ライセンス

MIT License
