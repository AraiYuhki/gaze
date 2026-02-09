use std::path::PathBuf;

use crate::domain::{build_status_map, NodeKind, TreeNode};
use crate::error::Result;
use crate::ui::tree_view;

use super::AppState;

impl AppState {
    // --- Tree View フラットキャッシュ管理 ---

    /// フラットキャッシュを無効化する
    ///
    /// ツリー構造に影響する操作（展開/折りたたみ/フィルタ変更等）の後に呼び出す
    pub(super) fn invalidate_tree_flat_cache(&mut self) {
        self.tree_flat_dirty = true;
    }

    /// フラットキャッシュを必要に応じて再構築する
    fn ensure_tree_flat_cache(&mut self) {
        if !self.tree_flat_dirty {
            return;
        }
        let flat = tree_view::flatten_tree(&self.tree_root, &self.display_filter, 0);
        self.tree_flat_cache = flat
            .into_iter()
            .map(|(node, depth)| (node.path.clone(), depth))
            .collect();
        self.tree_flat_dirty = false;
    }

    /// キャッシュ済みのフラットツリーの長さを取得する
    pub fn get_tree_flat_len(&mut self) -> usize {
        self.ensure_tree_flat_cache();
        self.tree_flat_cache.len()
    }

    /// キャッシュ済みのフラットツリーから指定インデックスのパスを取得する
    pub fn get_tree_flat_path(&mut self, index: usize) -> Option<PathBuf> {
        self.ensure_tree_flat_cache();
        self.tree_flat_cache.get(index).map(|(p, _)| p.clone())
    }

    // --- Tree View 用メソッド ---

    /// 選択されているツリーノードを展開/折りたたみする
    #[allow(dead_code)] // TODO(Phase 2): toggle 機能は Enter キーでのみ使用予定
    pub fn toggle_tree_node(&mut self) {
        // borrow checker 対策: status_map を先に構築
        let status_map = build_status_map(&self.status_cache);

        if let Some(node) = self.get_selected_tree_node_mut() {
            if node.kind == NodeKind::Directory {
                if node.expanded {
                    // 折りたたみ
                    node.expanded = false;
                } else {
                    // 展開
                    if node.children.is_none() {
                        // 遅延ロード
                        let _ = node.load_children();
                        node.apply_status_map(&status_map);
                    }
                    node.expanded = true;
                }
                self.invalidate_tree_flat_cache();
            }
        }
    }

    /// 選択されているツリーノードを展開する
    pub fn expand_tree_node(&mut self) {
        // borrow checker 対策: status_map を先に構築
        let status_map = build_status_map(&self.status_cache);

        if let Some(node) = self.get_selected_tree_node_mut() {
            if node.kind == NodeKind::Directory && !node.expanded {
                if node.children.is_none() {
                    // 遅延ロード
                    let _ = node.load_children();
                    node.apply_status_map(&status_map);
                }
                node.expanded = true;
                self.invalidate_tree_flat_cache();
            }
        }
    }

    /// 選択されているツリーノードを折りたたむ
    pub fn collapse_tree_node(&mut self) {
        if let Some(node) = self.get_selected_tree_node_mut() {
            if node.kind == NodeKind::Directory && node.expanded {
                node.expanded = false;
                self.invalidate_tree_flat_cache();
            }
        }
    }

    /// 表示フィルタを切り替える
    pub fn toggle_display_filter(&mut self) {
        self.display_filter.toggle();
        self.invalidate_tree_flat_cache();
        // 選択インデックスを調整
        let max = self.get_tree_flat_len();
        if max > 0 && self.tree_selected_index >= max {
            self.tree_selected_index = max - 1;
        }
    }

    /// Tree View のステータスをリフレッシュする（公開メソッド）
    ///
    /// Tree View で R キーを押した後に呼び出す
    pub fn refresh_tree_status(&mut self) {
        self.apply_status_to_tree();
    }

    /// 選択されている Tree ノードがファイルかどうかを返す
    pub fn is_selected_tree_node_file(&mut self) -> bool {
        let path = self.get_tree_flat_path(self.tree_selected_index);
        path.and_then(|p| {
            // パスでノードを探してファイルかどうかを判定
            find_node_by_path(&self.tree_root, &p)
        })
        .is_some_and(|node| node.kind == NodeKind::File)
    }

    /// 選択されている Tree ノードのパスを取得する
    pub fn get_selected_tree_node_path(&mut self) -> Option<PathBuf> {
        self.get_tree_flat_path(self.tree_selected_index)
    }

    /// Tree View で選択されているファイルのログを開く
    ///
    /// # Errors
    /// - ファイルが選択されていない場合
    /// - Git コマンドの実行に失敗した場合
    pub fn open_file_log(&mut self) -> Result<()> {
        // ファイルでない場合は何もしない
        if !self.is_selected_tree_node_file() {
            return Ok(());
        }

        // パスを取得
        let path = self.get_selected_tree_node_path();
        if let Some(path) = path {
            // リポジトリルートからの相対パスを計算
            let repo_root = self.git.repo_root().to_path_buf();
            let relative_path = path.strip_prefix(&repo_root).unwrap_or(&path);
            self.refresh_file_log(relative_path)?;
            self.current_view = super::View::Log;
        }
        Ok(())
    }

    // --- Tree View 検索用メソッド ---

    /// 検索モードを開始する
    pub fn start_tree_search(&mut self) {
        self.tree_search_mode = true;
        self.tree_search_query.clear();
        self.tree_search_matches.clear();
        self.tree_search_current_match = 0;
    }

    /// 検索をキャンセルする
    pub fn cancel_tree_search(&mut self) {
        self.tree_search_mode = false;
        self.tree_search_query.clear();
        self.tree_search_matches.clear();
        self.tree_search_current_match = 0;
    }

    /// 検索を確定し、最初のマッチへジャンプする
    pub fn confirm_tree_search(&mut self) {
        self.tree_search_mode = false;
        self.jump_to_current_match();
    }

    /// 検索文字列に文字を追加する
    pub fn add_tree_search_char(&mut self, c: char) {
        self.tree_search_query.push(c);
        self.update_tree_search_matches();
    }

    /// 検索文字列の末尾を削除する
    pub fn remove_tree_search_char(&mut self) {
        self.tree_search_query.pop();
        self.update_tree_search_matches();
    }

    /// 次のマッチへ移動する
    pub fn next_tree_search_match(&mut self) {
        if !self.tree_search_matches.is_empty() {
            self.tree_search_current_match =
                (self.tree_search_current_match + 1) % self.tree_search_matches.len();
            self.jump_to_current_match();
        }
    }

    /// 前のマッチへ移動する
    pub fn prev_tree_search_match(&mut self) {
        if !self.tree_search_matches.is_empty() {
            if self.tree_search_current_match == 0 {
                self.tree_search_current_match = self.tree_search_matches.len() - 1;
            } else {
                self.tree_search_current_match -= 1;
            }
            self.jump_to_current_match();
        }
    }

    /// 現在のマッチへジャンプする（親を展開してからインデックスを設定）
    fn jump_to_current_match(&mut self) {
        if self.tree_search_matches.is_empty() {
            return;
        }

        let target_path = self.tree_search_matches[self.tree_search_current_match].clone();

        // 親ディレクトリを展開する（キャッシュは expand 内で無効化される）
        self.expand_parents_for_path(&target_path);

        // キャッシュを再構築してインデックスを取得
        self.ensure_tree_flat_cache();
        for (index, (path, _)) in self.tree_flat_cache.iter().enumerate() {
            if *path == target_path {
                self.tree_selected_index = index;
                break;
            }
        }
    }

    /// 指定されたパスの親ディレクトリを全て展開する
    fn expand_parents_for_path(&mut self, target_path: &std::path::Path) {
        // ルートからターゲットまでのパスを収集
        let mut ancestors: Vec<std::path::PathBuf> = Vec::new();
        let mut current = target_path.parent();
        while let Some(parent) = current {
            // ルートパス自体は除外し、ルートパスの子孫のみを追加
            if parent != self.tree_root.path && parent.starts_with(&self.tree_root.path) {
                ancestors.push(parent.to_path_buf());
            }
            current = parent.parent();
        }

        // ルートに近い順に展開
        ancestors.reverse();

        // HashMap を1回だけ構築（borrow checker 対策も兼ねる）
        let status_map = build_status_map(&self.status_cache);

        let mut changed = false;
        for ancestor_path in ancestors {
            if let Some(node) = find_node_by_path_mut(&mut self.tree_root, &ancestor_path) {
                if node.kind == NodeKind::Directory && !node.expanded {
                    if node.children.is_none() {
                        let _ = node.load_children();
                        node.apply_status_map(&status_map);
                    }
                    node.expanded = true;
                    changed = true;
                }
            }
        }
        if changed {
            self.invalidate_tree_flat_cache();
        }
    }

    /// 検索マッチリストを更新する（ロード済みノードのみ検索）
    fn update_tree_search_matches(&mut self) {
        self.tree_search_matches.clear();
        self.tree_search_current_match = 0;

        if self.tree_search_query.is_empty() {
            return;
        }

        let query_lower = self.tree_search_query.to_lowercase();

        // ロード済みのノードのみ検索（未ロードディレクトリは自動ロードしない）
        self.search_tree_recursive(&query_lower);

        // 最初のマッチへジャンプ（検索モード中）
        if !self.tree_search_matches.is_empty() {
            self.jump_to_current_match();
        }
    }

    /// ツリーを再帰的に検索してマッチを収集する
    ///
    /// ロード済みのノードのみ検索する。未ロードのディレクトリは自動ロードしない。
    /// 大規模リポジトリでの検索性能を確保するため。
    fn search_tree_recursive(&mut self, query: &str) {
        fn collect_matches(
            node: &TreeNode,
            query: &str,
            filter: &crate::filter::DisplayFilter,
            matches: &mut Vec<std::path::PathBuf>,
        ) {
            if let Some(children) = &node.children {
                for child in children {
                    // フィルタで非表示のものはスキップ
                    if filter.should_hide(&child.path) {
                        continue;
                    }

                    // 名前がクエリにマッチするか確認
                    let name_lower = child.name.to_lowercase();
                    if name_lower.contains(query) {
                        matches.push(child.path.clone());
                    }

                    // ディレクトリの場合はロード済みの子のみ再帰
                    if child.kind == NodeKind::Directory && child.children.is_some() {
                        collect_matches(child, query, filter, matches);
                    }
                }
            }
        }

        let filter = self.display_filter.clone();
        let mut matches = Vec::new();
        collect_matches(&self.tree_root, query, &filter, &mut matches);
        self.tree_search_matches = matches;
    }

    /// Status キャッシュをツリーに適用する
    ///
    /// HashMap を1回だけ構築し、全ノードに対して再利用する
    pub(super) fn apply_status_to_tree(&mut self) {
        let status_map = build_status_map(&self.status_cache);

        // ルートノードにステータスを適用
        self.tree_root.apply_status_map(&status_map);

        // 展開されている子ノードにも再帰的に適用
        fn apply_recursive(node: &mut TreeNode, status_map: &crate::domain::StatusMap) {
            if let Some(children) = &mut node.children {
                for child in children {
                    child.apply_status_map(status_map);
                    if child.expanded {
                        apply_recursive(child, status_map);
                    }
                }
            }
        }
        apply_recursive(&mut self.tree_root, &status_map);
    }

    /// 選択されているツリーノードへの可変参照を取得する
    fn get_selected_tree_node_mut(&mut self) -> Option<&mut TreeNode> {
        self.ensure_tree_flat_cache();
        let target_path = self
            .tree_flat_cache
            .get(self.tree_selected_index)
            .map(|(p, _)| p.clone());

        if let Some(path) = target_path {
            find_node_by_path_mut(&mut self.tree_root, &path)
        } else {
            None
        }
    }
}

/// パスでノードを探して不変参照を返す
///
/// パスの前方一致を使って探索を枝刈りし、ターゲットが含まれ得ないサブツリーをスキップする。
pub(super) fn find_node_by_path<'a>(
    node: &'a TreeNode,
    path: &std::path::Path,
) -> Option<&'a TreeNode> {
    if node.path == path {
        return Some(node);
    }

    if let Some(children) = &node.children {
        for child in children {
            if child.path == path {
                return Some(child);
            }
            if child.kind == NodeKind::Directory && path.starts_with(&child.path) {
                if let Some(found) = find_node_by_path(child, path) {
                    return Some(found);
                }
            }
        }
    }

    None
}

/// パスでノードを探して可変参照を返す
///
/// パスの前方一致を使って探索を枝刈りし、ターゲットが含まれ得ないサブツリーをスキップする。
fn find_node_by_path_mut<'a>(
    node: &'a mut TreeNode,
    path: &std::path::Path,
) -> Option<&'a mut TreeNode> {
    if node.path == path {
        return Some(node);
    }

    if let Some(children) = &mut node.children {
        for child in children {
            // ターゲットパスが子のパスで始まるか、子のパスがターゲットと一致する場合のみ探索
            // ファイルノードは path が一致する場合のみ、ディレクトリノードは path が
            // ターゲットの祖先である場合のみ再帰する
            if child.path == path {
                return Some(child);
            }
            if child.kind == NodeKind::Directory && path.starts_with(&child.path) {
                if let Some(found) = find_node_by_path_mut(child, path) {
                    return Some(found);
                }
            }
        }
    }

    None
}
