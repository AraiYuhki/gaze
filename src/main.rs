use std::env;
use std::io;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

mod app;
mod cli;
mod config;
mod domain;
mod error;
mod filter;
mod pager;
mod ui;
mod update;

use app::{AppState, BranchInputMode, CommitMode, ConfirmDialog, StashInputMode, View};
use config::Settings;
use pager::Pager;

fn main() -> Result<()> {
    // CLI 引数の処理
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "--version" | "-V" => {
                update::print_version();
                return Ok(());
            }
            "--check-update" => {
                return update::check_update();
            }
            "--update" => {
                return update::update();
            }
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            arg => {
                eprintln!("Unknown option: {}", arg);
                print_help();
                std::process::exit(1);
            }
        }
    }

    // パニック時にターミナルを復帰させる
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic);
    }));

    // 設定ファイルの読み込み
    let settings = match Settings::load() {
        Ok(settings) => settings,
        Err(e) => {
            eprintln!("Warning: Failed to load config: {}", e);
            Settings::default()
        }
    };

    // Git リポジトリの検出
    let current_dir = env::current_dir()?;
    let mut state = match AppState::new(&current_dir) {
        Ok(state) => state,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, &mut state, &settings);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut AppState,
    settings: &Settings,
) -> Result<()> {
    let pager = Pager::with_command(settings.pager.command.clone());

    loop {
        // 描画（View に応じて切り替え）
        terminal.draw(|f| {
            // Hunk モード中は Hunk 画面を表示
            if state.hunk_mode {
                ui::hunk_view::render(f, state);
            } else if state.commit_mode != CommitMode::None {
                // コミットモード中はコミット画面を表示
                ui::commit_view::render(f, state);
            } else {
                match state.current_view {
                    View::Status => ui::status_view::render(f, state),
                    View::Tree => ui::tree_view::render(f, state),
                    View::Log => ui::log_view::render(f, state),
                    View::Stash => ui::stash_view::render(f, state),
                    View::Branch => ui::branch_view::render(f, state),
                }
            }
            // ヘルプ画面をオーバーレイ表示
            if state.show_help {
                ui::help_view::render(f);
            }
        })?;

        // 終了フラグのチェック
        if state.should_quit {
            return Ok(());
        }

        // バックグラウンド status の結果をチェック
        state.check_background_status();

        // イベント処理（100ms ポーリングでバックグラウンド結果の反映を可能にする）
        if !event::poll(std::time::Duration::from_millis(100))? {
            continue;
        }
        if let Event::Key(key) = event::read()? {
            // ヘルプ画面表示中は任意のキーで閉じる
            if state.show_help {
                state.show_help = false;
                continue;
            }

            // Hunk モード中のキー処理
            if state.hunk_mode {
                handle_hunk_mode_keys(state, key.code)?;
                continue;
            }

            // 確認ダイアログ表示中の場合
            match &state.confirm_dialog {
                ConfirmDialog::DiscardChanges { .. } => {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            if let Err(e) = state.discard_changes() {
                                state.set_status_message(&format!("Error: {}", e));
                            } else {
                                state.set_status_message("Changes discarded");
                            }
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            state.cancel_confirm();
                        }
                        _ => {}
                    }
                    continue;
                }
                ConfirmDialog::Checkout { commit_hash } => {
                    let hash = commit_hash.clone();
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            if let Err(e) = state.checkout_commit() {
                                state.set_status_message(&format!("Error: {}", e));
                            } else {
                                state.set_status_message(&format!("Checked out {}", hash));
                            }
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            state.cancel_confirm();
                        }
                        _ => {}
                    }
                    continue;
                }
                ConfirmDialog::Amend => {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            state.cancel_confirm();
                            if let Err(e) = state.start_amend_mode() {
                                state.set_status_message(&format!("Error: {}", e));
                            }
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            state.cancel_confirm();
                        }
                        _ => {}
                    }
                    continue;
                }
                ConfirmDialog::DropStash { .. } => {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            if let Err(e) = state.stash_drop() {
                                state.set_status_message(&format!("Error: {}", e));
                            }
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            state.cancel_confirm();
                        }
                        _ => {}
                    }
                    continue;
                }
                ConfirmDialog::CheckoutBranch { .. } => {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            if let Err(e) = state.checkout_branch() {
                                state.set_status_message(&format!("Error: {}", e));
                            }
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            state.cancel_confirm();
                        }
                        _ => {}
                    }
                    continue;
                }
                ConfirmDialog::Push => {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            if let Err(e) = state.execute_push() {
                                state.set_status_message(&format!("Error: {}", e));
                            }
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            state.cancel_confirm();
                        }
                        _ => {}
                    }
                    continue;
                }
                ConfirmDialog::Pull => {
                    match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            if let Err(e) = state.execute_pull() {
                                state.set_status_message(&format!("Error: {}", e));
                            }
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            state.cancel_confirm();
                        }
                        _ => {}
                    }
                    continue;
                }
                ConfirmDialog::None => {}
            }

            // Stash 入力モード中のキー処理
            if state.stash_input_mode != StashInputMode::None {
                handle_stash_input_keys(state, &key)?;
                continue;
            }

            // Branch 検索モード中のキー処理
            if state.branch_input_mode != BranchInputMode::None {
                handle_branch_input_keys(state, &key)?;
                continue;
            }

            // コミットモード中のキー処理
            if state.commit_mode != CommitMode::None {
                handle_commit_mode_keys(terminal, state, &key, &pager)?;
                continue;
            }

            // View 切り替え（共通）
            match key.code {
                KeyCode::Char('1') => {
                    state.switch_view(View::Status);
                    continue;
                }
                KeyCode::Char('2') => {
                    state.switch_view(View::Tree);
                    continue;
                }
                KeyCode::Char('3') => {
                    state.switch_view(View::Log);
                    continue;
                }
                KeyCode::Char('4') => {
                    state.switch_view(View::Stash);
                    continue;
                }
                KeyCode::Char('5') => {
                    state.switch_view(View::Branch);
                    continue;
                }
                KeyCode::Tab => {
                    // Tab で順次切り替え: Status -> Tree -> Log -> Stash -> Branch -> Status
                    let next_view = match state.current_view {
                        View::Status => View::Tree,
                        View::Tree => View::Log,
                        View::Log => View::Stash,
                        View::Stash => View::Branch,
                        View::Branch => View::Status,
                    };
                    state.switch_view(next_view);
                    continue;
                }
                KeyCode::Char('?') => {
                    state.show_help = true;
                    continue;
                }
                KeyCode::Char('q') => {
                    state.should_quit = true;
                    continue;
                }
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    state.should_quit = true;
                    continue;
                }
                _ => {}
            }

            // View 固有のキー処理
            match state.current_view {
                View::Status => handle_status_view_keys(terminal, state, key.code, &pager)?,
                View::Tree => handle_tree_view_keys(state, key.code),
                View::Log => handle_log_view_keys(terminal, state, key.code, &pager)?,
                View::Stash => handle_stash_view_keys(terminal, state, key.code, &pager)?,
                View::Branch => handle_branch_view_keys(state, key.code)?,
            }
        }
    }
}

/// Status View のキー処理
fn handle_status_view_keys(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut AppState,
    code: KeyCode,
    pager: &Pager,
) -> Result<()> {
    match code {
        // ナビゲーション
        KeyCode::Char('j') | KeyCode::Down => {
            state.select_next();
            state.clear_status_message();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.select_previous();
            state.clear_status_message();
        }
        KeyCode::Char('g') => {
            state.select_first();
            state.clear_status_message();
        }
        KeyCode::Char('G') => {
            state.select_last();
            state.clear_status_message();
        }
        // ステージング
        KeyCode::Char('s') => {
            if let Err(e) = state.toggle_stage() {
                state.set_status_message(&format!("Error: {}", e));
            } else {
                state.set_status_message("Staged/Unstaged");
            }
        }
        // 差分表示
        KeyCode::Char('d') => {
            match state.get_diff() {
                Ok(diff) => {
                    if diff.is_empty() {
                        state.set_status_message("No diff available");
                    } else {
                        // ページャ表示のためにターミナルを一時的に復帰
                        disable_raw_mode()?;
                        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

                        if let Err(e) = pager.display(&diff) {
                            // ページャ失敗時はアプリを継続
                            enable_raw_mode()?;
                            execute!(terminal.backend_mut(), EnterAlternateScreen)?;
                            state.set_status_message(&format!("Pager error: {}", e));
                        } else {
                            // ページャ正常終了後にターミナルを再初期化
                            enable_raw_mode()?;
                            execute!(terminal.backend_mut(), EnterAlternateScreen)?;
                            terminal.clear()?;
                        }
                    }
                }
                Err(e) => {
                    state.set_status_message(&format!("Error: {}", e));
                }
            }
        }
        // 変更破棄
        KeyCode::Char('r') => {
            state.show_discard_confirm();
        }
        // 手動リフレッシュ
        KeyCode::Char('R') => {
            if let Err(e) = state.refresh_status() {
                state.set_status_message(&format!("Error: {}", e));
            } else {
                state.set_status_message("Refreshed");
            }
        }
        // コミット
        KeyCode::Char('c') => {
            state.start_commit_mode();
        }
        // Amend（確認ダイアログを表示）
        KeyCode::Char('C') => {
            state.confirm_dialog = ConfirmDialog::Amend;
            state.set_status_message("Amend last commit? (y/n)");
        }
        // Push（確認ダイアログを表示）
        KeyCode::Char('P') => {
            state.show_push_confirm();
            state.set_status_message("Push to remote? (y/n)");
        }
        // Pull（確認ダイアログを表示）
        KeyCode::Char('U') => {
            state.show_pull_confirm();
            if matches!(state.confirm_dialog, ConfirmDialog::Pull) {
                state.set_status_message("Pull from remote? (y/n)");
            }
        }
        // Hunk ステージングモード
        KeyCode::Char('H') => {
            if let Err(e) = state.start_hunk_mode() {
                state.set_status_message(&format!("Error: {}", e));
            }
        }
        _ => {}
    }
    Ok(())
}

/// Tree View のキー処理
fn handle_tree_view_keys(state: &mut AppState, code: KeyCode) {
    // 検索モード中の処理
    if state.tree_search_mode {
        match code {
            KeyCode::Esc => {
                state.cancel_tree_search();
            }
            KeyCode::Enter => {
                state.confirm_tree_search();
                let match_count = state.tree_search_matches.len();
                if match_count > 0 {
                    state.set_status_message(&format!("Found {} match(es)", match_count));
                } else {
                    state.set_status_message("No matches found");
                }
            }
            KeyCode::Backspace => {
                state.remove_tree_search_char();
            }
            KeyCode::Char(c) => {
                state.add_tree_search_char(c);
            }
            _ => {}
        }
        return;
    }

    // 通常モードの処理
    match code {
        // ナビゲーション
        KeyCode::Char('j') | KeyCode::Down => {
            state.select_next();
            state.clear_status_message();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.select_previous();
            state.clear_status_message();
        }
        KeyCode::Char('g') => {
            state.select_first();
            state.clear_status_message();
        }
        KeyCode::Char('G') => {
            state.select_last();
            state.clear_status_message();
        }
        // 展開（Enter）
        KeyCode::Enter => {
            state.expand_tree_node();
        }
        // 展開 or ファイルログ（l）
        KeyCode::Char('l') => {
            if state.is_selected_tree_node_file() {
                // ファイルの場合はファイルログを表示
                if let Err(e) = state.open_file_log() {
                    state.set_status_message(&format!("Error: {}", e));
                }
            } else {
                // ディレクトリの場合は展開
                state.expand_tree_node();
            }
        }
        KeyCode::Char('h') => {
            state.collapse_tree_node();
        }
        // フィルタ切替
        KeyCode::Char('H') => {
            state.toggle_display_filter();
            let status = if state.display_filter.is_enabled() {
                "Filter ON"
            } else {
                "Filter OFF"
            };
            state.set_status_message(status);
        }
        // 検索
        KeyCode::Char('/') => {
            state.start_tree_search();
        }
        // 次のマッチへ
        KeyCode::Char('n') => {
            if !state.tree_search_matches.is_empty() {
                state.next_tree_search_match();
                state.set_status_message(&format!(
                    "Match {}/{}",
                    state.tree_search_current_match + 1,
                    state.tree_search_matches.len()
                ));
            }
        }
        // 前のマッチへ
        KeyCode::Char('N') => {
            if !state.tree_search_matches.is_empty() {
                state.prev_tree_search_match();
                state.set_status_message(&format!(
                    "Match {}/{}",
                    state.tree_search_current_match + 1,
                    state.tree_search_matches.len()
                ));
            }
        }
        // 手動リフレッシュ
        KeyCode::Char('R') => {
            if let Err(e) = state.refresh_status() {
                state.set_status_message(&format!("Error: {}", e));
            } else {
                // Tree View ではリフレッシュ後にステータスを再適用
                state.refresh_tree_status();
                state.set_status_message("Refreshed");
            }
        }
        _ => {}
    }
}

/// Log View のキー処理
fn handle_log_view_keys(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut AppState,
    code: KeyCode,
    pager: &Pager,
) -> Result<()> {
    match code {
        // ナビゲーション
        KeyCode::Char('j') | KeyCode::Down => {
            state.select_next();
            state.clear_status_message();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.select_previous();
            state.clear_status_message();
        }
        KeyCode::Char('g') => {
            state.select_first();
            state.clear_status_message();
        }
        KeyCode::Char('G') => {
            state.select_last();
            state.clear_status_message();
        }
        // コミット詳細表示
        KeyCode::Enter => {
            match state.get_commit_details() {
                Ok(details) => {
                    if details.is_empty() {
                        state.set_status_message("No commit selected");
                    } else {
                        // ページャ表示のためにターミナルを一時的に復帰
                        disable_raw_mode()?;
                        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

                        if let Err(e) = pager.display(&details) {
                            // ページャ失敗時はアプリを継続
                            enable_raw_mode()?;
                            execute!(terminal.backend_mut(), EnterAlternateScreen)?;
                            state.set_status_message(&format!("Pager error: {}", e));
                        } else {
                            // ページャ正常終了後にターミナルを再初期化
                            enable_raw_mode()?;
                            execute!(terminal.backend_mut(), EnterAlternateScreen)?;
                            terminal.clear()?;
                        }
                    }
                }
                Err(e) => {
                    state.set_status_message(&format!("Error: {}", e));
                }
            }
        }
        // チェックアウト（ファイルログモードでは無効）
        KeyCode::Char('c') => {
            if !state.is_file_log_mode() {
                state.show_checkout_confirm();
            }
        }
        // 手動リフレッシュ
        KeyCode::Char('R') => {
            let result = if state.is_file_log_mode() {
                // ファイルログモードの場合は現在のパスでリフレッシュ
                if let Some(path) = state.file_log_path.clone() {
                    state.refresh_file_log(&path)
                } else {
                    Ok(())
                }
            } else {
                state.refresh_log()
            };
            if let Err(e) = result {
                state.set_status_message(&format!("Error: {}", e));
            } else {
                state.set_status_message("Refreshed");
            }
        }
        // ファイルログモードを終了して Tree View に戻る
        KeyCode::Esc => {
            if state.is_file_log_mode() {
                state.clear_file_log();
                state.switch_view(View::Tree);
            }
        }
        _ => {}
    }
    Ok(())
}

/// コミットモードのキー処理
fn handle_commit_mode_keys(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut AppState,
    key: &event::KeyEvent,
    pager: &Pager,
) -> Result<()> {
    // Ctrl+Enter または Ctrl+D でコミット実行
    // 注: 多くのターミナルでは Ctrl+Enter が正しく認識されないため Ctrl+D を推奨
    let is_commit_key = (key.code == KeyCode::Enter
        && key.modifiers.contains(KeyModifiers::CONTROL))
        || (key.code == KeyCode::Char('d') && key.modifiers.contains(KeyModifiers::CONTROL));

    if is_commit_key {
        if let Err(e) = state.execute_commit() {
            state.set_status_message(&format!("Error: {}", e));
        }
        return Ok(());
    }

    // Esc でキャンセル
    if key.code == KeyCode::Esc {
        state.cancel_commit_mode();
        state.set_status_message("Commit cancelled");
        return Ok(());
    }

    // Tab でフォーカス切り替え
    if key.code == KeyCode::Tab {
        state.commit_toggle_focus();
        return Ok(());
    }

    // ファイル一覧にフォーカスしている場合
    if state.commit_focus_files {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                state.commit_file_next();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                state.commit_file_prev();
            }
            KeyCode::Char('d') => {
                // staged diff を表示
                match state.get_commit_staged_diff() {
                    Ok(diff) => {
                        if diff.is_empty() {
                            state.set_status_message("No diff available");
                        } else {
                            disable_raw_mode()?;
                            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

                            if let Err(e) = pager.display(&diff) {
                                enable_raw_mode()?;
                                execute!(terminal.backend_mut(), EnterAlternateScreen)?;
                                state.set_status_message(&format!("Pager error: {}", e));
                            } else {
                                enable_raw_mode()?;
                                execute!(terminal.backend_mut(), EnterAlternateScreen)?;
                                terminal.clear()?;
                            }
                        }
                    }
                    Err(e) => {
                        state.set_status_message(&format!("Error: {}", e));
                    }
                }
            }
            _ => {}
        }
    } else {
        // メッセージ入力にフォーカスしている場合
        match key.code {
            KeyCode::Enter => {
                state.commit_new_line();
            }
            KeyCode::Backspace => {
                state.commit_delete_char();
            }
            KeyCode::Left => {
                state.commit_cursor_left();
            }
            KeyCode::Right => {
                state.commit_cursor_right();
            }
            KeyCode::Up => {
                state.commit_cursor_up();
            }
            KeyCode::Down => {
                state.commit_cursor_down();
            }
            KeyCode::Char(c) => {
                state.commit_insert_char(c);
            }
            _ => {}
        }
    }

    Ok(())
}

/// Stash View のキー処理
fn handle_stash_view_keys(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut AppState,
    code: KeyCode,
    pager: &Pager,
) -> Result<()> {
    match code {
        // ナビゲーション
        KeyCode::Char('j') | KeyCode::Down => {
            state.select_next();
            state.clear_status_message();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.select_previous();
            state.clear_status_message();
        }
        KeyCode::Char('g') => {
            state.select_first();
            state.clear_status_message();
        }
        KeyCode::Char('G') => {
            state.select_last();
            state.clear_status_message();
        }
        // Stash push（メッセージ入力ダイアログを表示）
        KeyCode::Char('s') => {
            state.start_stash_push();
        }
        // Stash pop
        KeyCode::Char('p') => {
            if let Err(e) = state.stash_pop() {
                state.set_status_message(&format!("Error: {}", e));
            }
        }
        // Stash apply
        KeyCode::Char('a') => {
            if let Err(e) = state.stash_apply() {
                state.set_status_message(&format!("Error: {}", e));
            }
        }
        // Stash drop（確認ダイアログを表示）
        KeyCode::Char('d') => {
            state.show_stash_drop_confirm();
        }
        // Stash show（内容表示）
        KeyCode::Enter => match state.get_stash_show() {
            Ok(content) => {
                if content.is_empty() {
                    state.set_status_message("No stash selected");
                } else {
                    disable_raw_mode()?;
                    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

                    if let Err(e) = pager.display(&content) {
                        enable_raw_mode()?;
                        execute!(terminal.backend_mut(), EnterAlternateScreen)?;
                        state.set_status_message(&format!("Pager error: {}", e));
                    } else {
                        enable_raw_mode()?;
                        execute!(terminal.backend_mut(), EnterAlternateScreen)?;
                        terminal.clear()?;
                    }
                }
            }
            Err(e) => {
                state.set_status_message(&format!("Error: {}", e));
            }
        },
        // 手動リフレッシュ
        KeyCode::Char('R') => {
            if let Err(e) = state.refresh_stash() {
                state.set_status_message(&format!("Error: {}", e));
            } else {
                state.set_status_message("Refreshed");
            }
        }
        _ => {}
    }
    Ok(())
}

/// Stash 入力モードのキー処理
fn handle_stash_input_keys(state: &mut AppState, key: &event::KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Enter => {
            // メッセージを確定して stash push を実行
            if let Err(e) = state.stash_push() {
                state.set_status_message(&format!("Error: {}", e));
            }
        }
        KeyCode::Esc => {
            state.cancel_stash_input();
        }
        KeyCode::Backspace => {
            state.stash_message_pop();
        }
        KeyCode::Char(c) => {
            state.stash_message_push(c);
        }
        _ => {}
    }
    Ok(())
}

/// Branch View のキー処理
fn handle_branch_view_keys(state: &mut AppState, code: KeyCode) -> Result<()> {
    match code {
        // ナビゲーション
        KeyCode::Char('j') | KeyCode::Down => {
            state.select_next();
            state.clear_status_message();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.select_previous();
            state.clear_status_message();
        }
        KeyCode::Char('g') => {
            state.select_first();
            state.clear_status_message();
        }
        KeyCode::Char('G') => {
            state.select_last();
            state.clear_status_message();
        }
        // ブランチ切り替え
        KeyCode::Enter => {
            state.show_branch_checkout_confirm();
        }
        // 検索モード開始
        KeyCode::Char('/') => {
            state.start_branch_search();
        }
        // 検索クリア
        KeyCode::Esc => {
            if !state.branch_search_query.is_empty() {
                state.clear_branch_search();
            }
        }
        // 手動リフレッシュ
        KeyCode::Char('R') => {
            if let Err(e) = state.refresh_branches() {
                state.set_status_message(&format!("Error: {}", e));
            } else {
                state.set_status_message("Refreshed");
            }
        }
        _ => {}
    }
    Ok(())
}

/// Branch 入力モードのキー処理
fn handle_branch_input_keys(state: &mut AppState, key: &event::KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Enter => {
            // 検索を確定
            state.confirm_branch_search();
        }
        KeyCode::Esc => {
            state.cancel_branch_search();
        }
        KeyCode::Backspace => {
            state.branch_search_pop();
        }
        KeyCode::Char(c) => {
            state.branch_search_push(c);
        }
        _ => {}
    }
    Ok(())
}

/// Hunk モードのキー処理
fn handle_hunk_mode_keys(state: &mut AppState, code: KeyCode) -> Result<()> {
    match code {
        // ナビゲーション（1行ずつ移動、ファイルヘッダはスキップ）
        KeyCode::Char('j') | KeyCode::Down => {
            state.hunk_select_next();
            state.clear_status_message();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.hunk_select_previous();
            state.clear_status_message();
        }
        // 選択した hunk / 行 / Visual 選択範囲をステージ
        KeyCode::Char('s') => {
            if let Err(e) = state.stage_selected_hunk() {
                state.set_status_message(&format!("Error: {}", e));
            }
        }
        // Visual モードのトグル
        KeyCode::Char('v') => {
            if state.hunk_visual_mode {
                state.hunk_visual_mode = false;
            } else {
                state.hunk_visual_mode = true;
                state.hunk_visual_anchor = state.hunk_selected_index;
            }
            state.clear_status_message();
        }
        // Esc: Visual モード中は Visual 解除のみ、通常時は hunk モード終了
        KeyCode::Esc => {
            if state.hunk_visual_mode {
                state.hunk_visual_mode = false;
                state.clear_status_message();
            } else {
                state.cancel_hunk_mode();
            }
        }
        // q: 常に hunk モード終了
        KeyCode::Char('q') => {
            state.cancel_hunk_mode();
        }
        _ => {}
    }
    Ok(())
}

/// ヘルプメッセージを表示
fn print_help() {
    println!(
        r#"gaze - A lightweight Git TUI tool

Usage: gaze [OPTIONS]

Options:
    -h, --help          Show this help message
    -V, --version       Show version information
    --check-update      Check for updates
    --update            Update to the latest version

Run 'gaze' without options to start the TUI in the current directory."#
    );
}
