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

use app::{AppState, ConfirmDialog};
use pager::Pager;

fn main() -> Result<()> {
    // パニック時にターミナルを復帰させる
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic);
    }));

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

    let result = run_app(&mut terminal, &mut state);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut AppState,
) -> Result<()> {
    let pager = Pager::new();

    loop {
        // 描画
        terminal.draw(|f| {
            ui::status_view::render(f, state);
        })?;

        // 終了フラグのチェック
        if state.should_quit {
            return Ok(());
        }

        // イベント処理
        if let Event::Key(key) = event::read()? {
            // 確認ダイアログ表示中の場合
            if matches!(state.confirm_dialog, ConfirmDialog::DiscardChanges { .. }) {
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

            // 通常のキー処理
            match key.code {
                // 終了
                KeyCode::Char('q') => {
                    state.should_quit = true;
                }
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
                // Ctrl+C でも終了
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    state.should_quit = true;
                }
                _ => {}
            }
        }
    }
}
