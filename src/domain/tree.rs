use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::domain::{FileStatus, StatusKind};
use crate::error::Result;

/// ステータスマップのエントリ（index と worktree のステータスを保持）
#[derive(Debug, Clone, Copy)]
pub struct StatusEntry {
    pub index: StatusKind,
    pub worktree: StatusKind,
}

/// 相対パスからステータスへのマッピング
pub type StatusMap = HashMap<PathBuf, StatusEntry>;

/// FileStatus のスライスから HashMap を構築する
///
/// 複数の TreeNode に対してステータスを適用する場合は、
/// この関数で1回だけ HashMap を構築し、各ノードの `apply_status_map` に渡す。
pub fn build_status_map(cache: &[FileStatus]) -> StatusMap {
    cache
        .iter()
        .map(|s| {
            (
                s.path.clone(),
                StatusEntry {
                    index: s.index,
                    worktree: s.worktree,
                },
            )
        })
        .collect()
}

/// ノードの種類
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    File,
    Directory,
}

/// ファイルツリーのノード
#[derive(Debug, Clone)]
pub struct TreeNode {
    /// ファイル名
    pub name: String,
    /// フルパス
    pub path: PathBuf,
    /// ノードの種類
    pub kind: NodeKind,
    /// 展開状態
    pub expanded: bool,
    /// Git ステータス（キャッシュから取得）
    pub git_status: Option<StatusKind>,
    /// 子ノード（None = 未ロード、Some([]) = ロード済みで空）
    pub children: Option<Vec<TreeNode>>,
}

impl TreeNode {
    /// 新しいディレクトリノードを作成する（子は未ロード状態）
    pub fn new_dir(name: String, path: PathBuf) -> Self {
        Self {
            name,
            path,
            kind: NodeKind::Directory,
            expanded: false,
            git_status: None,
            children: None, // 重要: 初期状態は None（未ロード）
        }
    }

    /// 新しいファイルノードを作成する
    #[allow(dead_code)] // TODO(Phase 2): テストおよび将来の拡張用
    pub fn new_file(name: String, path: PathBuf) -> Self {
        Self {
            name,
            path,
            kind: NodeKind::File,
            expanded: false,
            git_status: None,
            children: Some(vec![]), // ファイルは子を持たない
        }
    }

    /// 子ノードをロードする（遅延ロード）
    ///
    /// このメソッドはユーザーが展開操作をした時のみ呼び出すこと。
    /// 初期化時に再帰的に呼び出すことは禁止。
    ///
    /// # 重要
    /// - このメソッド内で git コマンドを実行しないこと
    /// - ファイルシステムの読み取りのみ行う
    pub fn load_children(&mut self) -> Result<()> {
        // ディレクトリでなければ何もしない
        if self.kind != NodeKind::Directory {
            return Ok(());
        }
        // 既にロード済みなら何もしない
        if self.children.is_some() {
            return Ok(());
        }

        let mut children = Vec::new();
        for entry in std::fs::read_dir(&self.path)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();

            // .git ディレクトリは git 管理対象外のため除外
            if name == ".git" {
                continue;
            }

            let path = entry.path();
            let kind = if path.is_dir() {
                NodeKind::Directory
            } else {
                NodeKind::File
            };

            let node = TreeNode {
                name,
                path,
                kind: kind.clone(),
                expanded: false,
                git_status: None,
                // ディレクトリは None（未ロード）、ファイルは Some([])
                children: if kind == NodeKind::Directory {
                    None
                } else {
                    Some(vec![])
                },
            };
            children.push(node);
        }

        // ディレクトリ優先、次に名前順でソート
        children.sort_by(|a, b| match (&a.kind, &b.kind) {
            (NodeKind::Directory, NodeKind::File) => std::cmp::Ordering::Less,
            (NodeKind::File, NodeKind::Directory) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });

        self.children = Some(children);
        Ok(())
    }

    /// Status キャッシュから Git ステータスを適用する
    ///
    /// HashMap を使用して O(children + cache) で検索する。
    /// 内部で HashMap を構築するため、単一ノードへの適用に適している。
    /// 複数ノードへの連続適用には `build_status_map` + `apply_status_map` を使う。
    ///
    /// # 重要
    /// - このメソッド内で git コマンドを実行しないこと
    /// - キャッシュからの検索のみ行う
    #[allow(dead_code)] // apply_status_map への移行済みだが、単発用途の便利メソッドとして残す
    pub fn apply_status_cache(&mut self, cache: &[crate::domain::FileStatus], repo_root: &Path) {
        let status_map = build_status_map(cache);
        self.apply_status_map(&status_map, repo_root);
    }

    /// HashMap 化済みのステータスマップから Git ステータスを適用する
    ///
    /// 複数ノードに対して連続適用する場合は、呼び出し側で `build_status_map` を1回だけ
    /// 呼び出してこのメソッドを使うことで、HashMap の再構築コストを回避できる。
    pub fn apply_status_map(&mut self, status_map: &StatusMap, repo_root: &Path) {
        if let Some(children) = &mut self.children {
            for child in children {
                // まずステータスをリセット（クリーンになったファイルの古いステータスを消す）
                child.git_status = None;

                // child.path（絶対パス）から repo_root プレフィックスを除いて相対パスを取得
                // status_map のキーは相対パス（例: "src/main.rs"）
                // child.path は絶対パス（例: "/project/src/main.rs"）
                // strip_prefix による正確な変換でサフィックス誤検知を防ぐ
                if let Ok(relative) = child.path.strip_prefix(repo_root) {
                    if let Some(entry) = status_map.get(relative) {
                        if entry.worktree != StatusKind::Unmodified {
                            child.git_status = Some(entry.worktree);
                        } else if entry.index != StatusKind::Unmodified {
                            child.git_status = Some(entry.index);
                        }
                    }
                }
            }
        }
    }

    /// 子ノードがロード済みかどうか
    #[allow(dead_code)] // TODO(Phase 2): UI でのロード状態表示用
    pub fn is_loaded(&self) -> bool {
        self.children.is_some()
    }

    /// 展開可能かどうか（ディレクトリで、子が存在するか未ロード）
    #[allow(dead_code)] // TODO(Phase 2): UI での展開可能性判定用
    pub fn can_expand(&self) -> bool {
        self.kind == NodeKind::Directory
            && (!self.is_loaded() || !self.children.as_ref().is_none_or(|c| c.is_empty()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_status_map_sets_worktree_status() {
        // Arrange
        let mut root = TreeNode::new_dir("project".to_string(), PathBuf::from("/project"));
        root.children = Some(vec![TreeNode {
            name: "main.rs".to_string(),
            path: PathBuf::from("/project/src/main.rs"),
            kind: NodeKind::File,
            expanded: false,
            git_status: None,
            children: Some(vec![]),
        }]);

        let cache = vec![FileStatus {
            index: StatusKind::Unmodified,
            worktree: StatusKind::Modified,
            path: PathBuf::from("src/main.rs"),
            original_path: None,
        }];
        let status_map = build_status_map(&cache);
        let repo_root = PathBuf::from("/project");

        // Act
        root.apply_status_map(&status_map, &repo_root);

        // Assert
        let child = &root.children.as_ref().unwrap()[0];
        assert_eq!(child.git_status, Some(StatusKind::Modified));
    }

    #[test]
    fn test_apply_status_map_prefers_worktree_over_index() {
        // Arrange
        let mut root = TreeNode::new_dir("project".to_string(), PathBuf::from("/project"));
        root.children = Some(vec![TreeNode {
            name: "main.rs".to_string(),
            path: PathBuf::from("/project/src/main.rs"),
            kind: NodeKind::File,
            expanded: false,
            git_status: None,
            children: Some(vec![]),
        }]);

        let cache = vec![FileStatus {
            index: StatusKind::Added,
            worktree: StatusKind::Modified,
            path: PathBuf::from("src/main.rs"),
            original_path: None,
        }];
        let status_map = build_status_map(&cache);
        let repo_root = PathBuf::from("/project");

        // Act
        root.apply_status_map(&status_map, &repo_root);

        // Assert: worktree が Unmodified でないので worktree が優先される
        let child = &root.children.as_ref().unwrap()[0];
        assert_eq!(child.git_status, Some(StatusKind::Modified));
    }

    #[test]
    fn test_apply_status_map_uses_index_when_worktree_unmodified() {
        // Arrange
        let mut root = TreeNode::new_dir("project".to_string(), PathBuf::from("/project"));
        root.children = Some(vec![TreeNode {
            name: "main.rs".to_string(),
            path: PathBuf::from("/project/src/main.rs"),
            kind: NodeKind::File,
            expanded: false,
            git_status: None,
            children: Some(vec![]),
        }]);

        let cache = vec![FileStatus {
            index: StatusKind::Added,
            worktree: StatusKind::Unmodified,
            path: PathBuf::from("src/main.rs"),
            original_path: None,
        }];
        let status_map = build_status_map(&cache);
        let repo_root = PathBuf::from("/project");

        // Act
        root.apply_status_map(&status_map, &repo_root);

        // Assert: worktree が Unmodified なので index が使われる
        let child = &root.children.as_ref().unwrap()[0];
        assert_eq!(child.git_status, Some(StatusKind::Added));
    }

    #[test]
    fn test_apply_status_map_no_false_positive_for_suffix_match() {
        // Arrange: tests/src/main.rs と src/main.rs が共存する場合
        let mut root = TreeNode::new_dir("project".to_string(), PathBuf::from("/project"));
        root.children = Some(vec![TreeNode {
            name: "main.rs".to_string(),
            path: PathBuf::from("/project/tests/src/main.rs"),
            kind: NodeKind::File,
            expanded: false,
            git_status: None,
            children: Some(vec![]),
        }]);

        // status_cache には src/main.rs のみ存在
        let cache = vec![FileStatus {
            index: StatusKind::Unmodified,
            worktree: StatusKind::Modified,
            path: PathBuf::from("src/main.rs"),
            original_path: None,
        }];
        let status_map = build_status_map(&cache);
        let repo_root = PathBuf::from("/project");

        // Act
        root.apply_status_map(&status_map, &repo_root);

        // Assert: tests/src/main.rs は src/main.rs にマッチしないはず
        let child = &root.children.as_ref().unwrap()[0];
        assert_eq!(child.git_status, None);
    }

    #[test]
    fn test_new_dir_creates_unloaded_directory() {
        // Arrange & Act
        let node = TreeNode::new_dir("src".to_string(), PathBuf::from("/project/src"));

        // Assert
        assert_eq!(node.name, "src");
        assert_eq!(node.kind, NodeKind::Directory);
        assert!(!node.expanded);
        assert!(node.children.is_none()); // 未ロード
    }

    #[test]
    fn test_new_file_creates_file_with_empty_children() {
        // Arrange & Act
        let node = TreeNode::new_file("main.rs".to_string(), PathBuf::from("/project/src/main.rs"));

        // Assert
        assert_eq!(node.name, "main.rs");
        assert_eq!(node.kind, NodeKind::File);
        assert!(node.children.is_some());
        assert!(node.children.unwrap().is_empty());
    }

    #[test]
    fn test_is_loaded_returns_false_for_unloaded_directory() {
        // Arrange
        let node = TreeNode::new_dir("src".to_string(), PathBuf::from("/project/src"));

        // Act & Assert
        assert!(!node.is_loaded());
    }

    #[test]
    fn test_is_loaded_returns_true_for_file() {
        // Arrange
        let node = TreeNode::new_file("main.rs".to_string(), PathBuf::from("/project/src/main.rs"));

        // Act & Assert
        assert!(node.is_loaded());
    }
}
