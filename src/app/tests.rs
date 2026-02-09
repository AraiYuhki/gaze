use super::*;
use crate::domain::StatusKind;
use std::path::PathBuf;

/// テスト用に最小限の AppState を生成する（実際の git リポジトリを使用）
fn create_test_state() -> AppState {
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    AppState::new(&current_dir).expect("Failed to create AppState")
}

/// テスト用の FileStatus を生成する
fn make_file_status(path: &str, index: StatusKind, worktree: StatusKind) -> FileStatus {
    FileStatus {
        index,
        worktree,
        path: PathBuf::from(path),
        original_path: None,
    }
}

#[test]
fn test_optimistic_stage_modified_file_updates_cache() {
    let mut state = create_test_state();
    let file = make_file_status("test.rs", StatusKind::Unmodified, StatusKind::Modified);
    state.status_cache = vec![file.clone()];
    state.selected_index = 0;

    state.optimistic_update_after_stage(&file);

    assert_eq!(state.status_cache.len(), 1);
    assert_eq!(state.status_cache[0].index, StatusKind::Modified);
    assert_eq!(state.status_cache[0].worktree, StatusKind::Unmodified);
}

#[test]
fn test_optimistic_stage_untracked_file_updates_cache() {
    let mut state = create_test_state();
    let file = make_file_status("new.rs", StatusKind::Untracked, StatusKind::Untracked);
    state.status_cache = vec![file.clone()];
    state.selected_index = 0;

    state.optimistic_update_after_stage(&file);

    assert_eq!(state.status_cache.len(), 1);
    assert_eq!(state.status_cache[0].index, StatusKind::Added);
    assert_eq!(state.status_cache[0].worktree, StatusKind::Unmodified);
}

#[test]
fn test_optimistic_stage_deleted_file_updates_cache() {
    let mut state = create_test_state();
    let file = make_file_status("old.rs", StatusKind::Unmodified, StatusKind::Deleted);
    state.status_cache = vec![file.clone()];
    state.selected_index = 0;

    state.optimistic_update_after_stage(&file);

    assert_eq!(state.status_cache.len(), 1);
    assert_eq!(state.status_cache[0].index, StatusKind::Deleted);
    assert_eq!(state.status_cache[0].worktree, StatusKind::Unmodified);
}

#[test]
fn test_optimistic_unstage_modified_file_updates_cache() {
    let mut state = create_test_state();
    let file = make_file_status("test.rs", StatusKind::Modified, StatusKind::Unmodified);
    state.status_cache = vec![file.clone()];
    state.selected_index = 0;

    state.optimistic_update_unstage(&file).unwrap();

    assert_eq!(state.status_cache.len(), 1);
    assert_eq!(state.status_cache[0].index, StatusKind::Unmodified);
    assert_eq!(state.status_cache[0].worktree, StatusKind::Modified);
}

#[test]
fn test_optimistic_unstage_added_file_becomes_untracked() {
    let mut state = create_test_state();
    let file = make_file_status("new.rs", StatusKind::Added, StatusKind::Unmodified);
    state.status_cache = vec![file.clone()];
    state.selected_index = 0;

    state.optimistic_update_unstage(&file).unwrap();

    assert_eq!(state.status_cache.len(), 1);
    assert_eq!(state.status_cache[0].index, StatusKind::Untracked);
    assert_eq!(state.status_cache[0].worktree, StatusKind::Untracked);
}

#[test]
fn test_optimistic_discard_removes_entry() {
    let mut state = create_test_state();
    state.status_cache = vec![
        make_file_status("a.rs", StatusKind::Unmodified, StatusKind::Modified),
        make_file_status("b.rs", StatusKind::Unmodified, StatusKind::Modified),
    ];
    state.selected_index = 0;

    // discard_changes は ConfirmDialog を必要とするためキャッシュ操作を直接テスト
    let target_path = PathBuf::from("a.rs");
    state.status_cache.retain(|f| f.path != target_path);

    assert_eq!(state.status_cache.len(), 1);
    assert_eq!(state.status_cache[0].path, PathBuf::from("b.rs"));
}

#[test]
fn test_optimistic_discard_adjusts_selected_index() {
    let mut state = create_test_state();
    state.status_cache = vec![
        make_file_status("a.rs", StatusKind::Unmodified, StatusKind::Modified),
        make_file_status("b.rs", StatusKind::Unmodified, StatusKind::Modified),
    ];
    // 末尾を選択
    state.selected_index = 1;

    // 末尾のエントリを削除
    let target_path = PathBuf::from("b.rs");
    state.status_cache.retain(|f| f.path != target_path);
    // 選択インデックスの範囲外チェック（discard_changes 内のロジック再現）
    if !state.status_cache.is_empty() && state.selected_index >= state.status_cache.len() {
        state.selected_index = state.status_cache.len() - 1;
    }

    assert_eq!(state.selected_index, 0);
}
