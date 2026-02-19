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
mod keys;
mod pager;
mod ui;
mod update;

use app::{AppState, BranchInputMode, CommitMode, ConfirmDialog, StashInputMode, View};
use config::Settings;
use keys::{
    handle_branch_input_keys, handle_branch_view_keys, handle_commit_mode_keys,
    handle_hunk_mode_keys, handle_log_view_keys, handle_stash_input_keys, handle_stash_view_keys,
    handle_status_view_keys, handle_tree_view_keys,
};
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
            if handle_confirm_dialog(state, key.code) {
                continue;
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

/// 確認ダイアログのキー処理
///
/// ダイアログが表示中であれば処理して true を返す。
/// 表示されていなければ false を返す。
fn handle_confirm_dialog(state: &mut AppState, code: KeyCode) -> bool {
    // ConfirmDialog::None の場合は何もしない
    if matches!(state.confirm_dialog, ConfirmDialog::None) {
        return false;
    }

    match code {
        // 選択切り替え
        KeyCode::Left | KeyCode::Char('h') => {
            state.confirm_selected_yes = true;
        }
        KeyCode::Right | KeyCode::Char('l') => {
            state.confirm_selected_yes = false;
        }
        // 決定
        KeyCode::Enter => {
            if state.confirm_selected_yes {
                execute_confirm_action(state);
            } else {
                state.cancel_confirm();
            }
        }
        // y キーで即実行
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            execute_confirm_action(state);
        }
        // n キーまたは Esc でキャンセル
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            state.cancel_confirm();
        }
        _ => {}
    }
    true
}

/// 確認ダイアログで Yes が選択された場合のアクション実行
fn execute_confirm_action(state: &mut AppState) {
    // Amend は特殊処理（cancel_confirm → start_amend_mode）
    if matches!(state.confirm_dialog, ConfirmDialog::Amend) {
        state.cancel_confirm();
        if let Err(e) = state.start_amend_mode() {
            state.set_status_message(&format!("Error: {}", e));
        }
        return;
    }

    // Checkout は成功メッセージにハッシュを含める
    let success_message = if let ConfirmDialog::Checkout { ref commit_hash } = state.confirm_dialog
    {
        Some(format!("Checked out {}", commit_hash))
    } else {
        None
    };

    let result = match &state.confirm_dialog {
        ConfirmDialog::DiscardChanges { .. } => {
            state.discard_changes().map(|_| "Changes discarded")
        }
        ConfirmDialog::Checkout { .. } => state.checkout_commit().map(|_| ""),
        ConfirmDialog::DropStash { .. } => state.stash_drop().map(|_| ""),
        ConfirmDialog::CheckoutBranch { .. } => state.checkout_branch().map(|_| ""),
        ConfirmDialog::Push => state.execute_push().map(|_| ""),
        ConfirmDialog::Pull => state.execute_pull().map(|_| ""),
        ConfirmDialog::Amend | ConfirmDialog::None => unreachable!(),
    };

    match result {
        Err(e) => state.set_status_message(&format!("Error: {}", e)),
        Ok(msg) => {
            if let Some(custom_msg) = success_message {
                state.set_status_message(&custom_msg);
            } else if !msg.is_empty() {
                state.set_status_message(msg);
            }
        }
    }
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
