use std::path::PathBuf;

use crate::domain::StatusKind;
use crate::error::Result;

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
    /// # 重要
    /// - このメソッド内で git コマンドを実行しないこと
    /// - キャッシュからの検索のみ行う
    pub fn apply_status_cache(&mut self, cache: &[crate::domain::FileStatus]) {
        if let Some(children) = &mut self.children {
            for child in children {
                // まずステータスをリセット（クリーンになったファイルの古いステータスを消す）
                child.git_status = None;

                // キャッシュからステータスを検索
                // child.path の末尾コンポーネントと cache の path が完全一致するものを探す
                if let Some(status) = cache.iter().find(|s| {
                    // 絶対パスの末尾から cache の相対パスと比較
                    child.path.ends_with(&s.path)
                }) {
                    // ワークツリーのステータスを優先、なければインデックスのステータス
                    if status.worktree != StatusKind::Unmodified {
                        child.git_status = Some(status.worktree);
                    } else if status.index != StatusKind::Unmodified {
                        child.git_status = Some(status.index);
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
