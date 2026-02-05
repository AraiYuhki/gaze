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

use app::{AppState, ConfirmDialog, View};
use config::Settings;
use pager::Pager;

fn main() -> Result<()> {
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
    let current_dir = std::env::current_dir()?;
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
            match state.current_view {
                View::Status => ui::status_view::render(f, state),
                View::Tree => ui::tree_view::render(f, state),
                View::Log => ui::log_view::render(f, state),
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

        // イベント処理
        if let Event::Key(key) = event::read()? {
            // ヘルプ画面表示中は任意のキーで閉じる
            if state.show_help {
                state.show_help = false;
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
                ConfirmDialog::None => {}
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
                KeyCode::Tab => {
                    // Tab で順次切り替え: Status -> Tree -> Log -> Status
                    let next_view = match state.current_view {
                        View::Status => View::Tree,
                        View::Tree => View::Log,
                        View::Log => View::Status,
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
        _ => {}
    }
    Ok(())
}

/// Tree View のキー処理
fn handle_tree_view_keys(state: &mut AppState, code: KeyCode) {
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
        // 展開/折りたたみ
        KeyCode::Enter | KeyCode::Char('l') => {
            state.expand_tree_node();
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
        // チェックアウト
        KeyCode::Char('c') => {
            state.show_checkout_confirm();
        }
        // 手動リフレッシュ
        KeyCode::Char('R') => {
            if let Err(e) = state.refresh_log() {
                state.set_status_message(&format!("Error: {}", e));
            } else {
                state.set_status_message("Refreshed");
            }
        }
        _ => {}
    }
    Ok(())
}
