//! 自己更新機能
//!
//! GitHub Releases API を使用して最新バージョンを確認し、
//! 必要に応じてバイナリを更新する。

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

/// GitHub リポジトリ情報
const REPO_OWNER: &str = "AraiYuhki";
const REPO_NAME: &str = "gaze";

/// 現在のバージョン
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// GitHub Release API のレスポンス
#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

/// Release アセット
#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

/// バージョン情報を表示する
pub fn print_version() {
    println!("gaze {}", VERSION);
}

/// 最新バージョンを確認する
pub fn check_update() -> Result<()> {
    println!("Checking for updates...");

    let latest = fetch_latest_release()?;
    let latest_version = latest.tag_name.trim_start_matches('v');

    if is_newer_version(latest_version, VERSION) {
        println!(
            "New version available: v{} (current: v{})",
            latest_version, VERSION
        );
        println!("Run 'gaze --update' to update.");
    } else {
        println!("You are using the latest version (v{}).", VERSION);
    }

    Ok(())
}

/// 最新バージョンに更新する
pub fn update() -> Result<()> {
    println!("Checking for updates...");

    let latest = fetch_latest_release()?;
    let latest_version = latest.tag_name.trim_start_matches('v');

    if !is_newer_version(latest_version, VERSION) {
        println!("You are already using the latest version (v{}).", VERSION);
        return Ok(());
    }

    println!(
        "New version available: v{} (current: v{})",
        latest_version, VERSION
    );

    // 確認プロンプト
    print!("Do you want to update? [y/N]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if !input.trim().eq_ignore_ascii_case("y") {
        println!("Update cancelled.");
        return Ok(());
    }

    // プラットフォームに応じたアセット名を取得
    let asset_name = get_asset_name()?;

    // ダウンロード URL を取得
    let asset = latest
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .context(format!("Asset not found: {}", asset_name))?;

    println!("Downloading {}...", asset.name);

    // ダウンロード
    let response =
        reqwest::blocking::get(&asset.browser_download_url).context("Failed to download update")?;

    if !response.status().is_success() {
        bail!("Failed to download: HTTP {}", response.status());
    }

    let bytes = response.bytes().context("Failed to read response body")?;

    // 一時ファイルに保存
    let temp_dir = env::temp_dir();
    let archive_path = temp_dir.join(&asset.name);
    fs::write(&archive_path, &bytes).context("Failed to write temporary file")?;

    // 現在の実行ファイルのパスを取得
    let current_exe = env::current_exe().context("Failed to get current executable path")?;

    // アーカイブを展開してインストール
    install_from_archive(&archive_path, &current_exe)?;

    // 一時ファイルを削除
    let _ = fs::remove_file(&archive_path);

    println!("Successfully updated to v{}!", latest_version);

    Ok(())
}

/// GitHub Releases API から最新リリースを取得
fn fetch_latest_release() -> Result<Release> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        REPO_OWNER, REPO_NAME
    );

    let client = reqwest::blocking::Client::new();
    let response = client
        .get(&url)
        .header("User-Agent", format!("gaze/{}", VERSION))
        .send()
        .context("Failed to connect to GitHub API")?;

    if !response.status().is_success() {
        bail!("GitHub API error: HTTP {}", response.status());
    }

    response
        .json::<Release>()
        .context("Failed to parse GitHub API response")
}

/// バージョン比較（semver）
fn is_newer_version(new: &str, current: &str) -> bool {
    let parse = |v: &str| -> Option<(u32, u32, u32)> {
        let parts: Vec<&str> = v.split('.').collect();
        if parts.len() >= 3 {
            Some((
                parts[0].parse().ok()?,
                parts[1].parse().ok()?,
                parts[2].parse().ok()?,
            ))
        } else if parts.len() == 2 {
            Some((parts[0].parse().ok()?, parts[1].parse().ok()?, 0))
        } else {
            None
        }
    };

    match (parse(new), parse(current)) {
        (Some(new_v), Some(cur_v)) => new_v > cur_v,
        _ => false,
    }
}

/// プラットフォームに応じたアセット名を返す
fn get_asset_name() -> Result<String> {
    let target = if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        "x86_64-unknown-linux-gnu"
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "x86_64") {
        "x86_64-apple-darwin"
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        "aarch64-apple-darwin"
    } else if cfg!(target_os = "windows") && cfg!(target_arch = "x86_64") {
        "x86_64-pc-windows-msvc"
    } else {
        bail!("Unsupported platform");
    };

    let ext = if cfg!(target_os = "windows") {
        "zip"
    } else {
        "tar.gz"
    };

    Ok(format!("gaze-{}.{}", target, ext))
}

/// アーカイブからインストール
fn install_from_archive(archive_path: &Path, target_path: &Path) -> Result<()> {
    let temp_dir = env::temp_dir();

    if cfg!(target_os = "windows") {
        // Windows: zip を展開
        install_from_zip(archive_path, target_path, &temp_dir)?;
    } else {
        // Unix: tar.gz を展開
        install_from_tar_gz(archive_path, target_path, &temp_dir)?;
    }

    Ok(())
}

/// tar.gz からインストール（Unix）
#[cfg(not(target_os = "windows"))]
fn install_from_tar_gz(archive_path: &Path, target_path: &Path, temp_dir: &Path) -> Result<()> {
    use std::process::Command;

    // 展開先
    let extract_dir = temp_dir.join("gaze-update");
    let _ = fs::remove_dir_all(&extract_dir);
    fs::create_dir_all(&extract_dir)?;

    // tar で展開
    let status = Command::new("tar")
        .args([
            "-xzf",
            archive_path.to_str().unwrap(),
            "-C",
            extract_dir.to_str().unwrap(),
        ])
        .status()
        .context("Failed to run tar")?;

    if !status.success() {
        bail!("tar extraction failed");
    }

    // バイナリを見つける
    let binary_path = extract_dir.join("gaze");
    if !binary_path.exists() {
        bail!("Binary not found in archive");
    }

    // 既存のバイナリをバックアップ
    let backup_path = target_path.with_extension("old");
    if target_path.exists() {
        fs::rename(target_path, &backup_path).context("Failed to backup current binary")?;
    }

    // 新しいバイナリをインストール
    if let Err(e) = fs::copy(&binary_path, target_path) {
        // 失敗した場合はバックアップから復元
        if backup_path.exists() {
            let _ = fs::rename(&backup_path, target_path);
        }
        return Err(e).context("Failed to install new binary");
    }

    // 実行権限を設定
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(target_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(target_path, perms)?;
    }

    // バックアップを削除
    let _ = fs::remove_file(&backup_path);

    // 一時ディレクトリを削除
    let _ = fs::remove_dir_all(&extract_dir);

    Ok(())
}

#[cfg(target_os = "windows")]
fn install_from_tar_gz(_archive_path: &Path, _target_path: &Path, _temp_dir: &Path) -> Result<()> {
    bail!("tar.gz is not supported on Windows");
}

/// zip からインストール（Windows）
#[cfg(target_os = "windows")]
fn install_from_zip(archive_path: &Path, target_path: &Path, temp_dir: &Path) -> Result<()> {
    use std::process::Command;

    // 展開先
    let extract_dir = temp_dir.join("gaze-update");
    let _ = fs::remove_dir_all(&extract_dir);
    fs::create_dir_all(&extract_dir)?;

    // PowerShell で展開
    let status = Command::new("powershell")
        .args([
            "-Command",
            &format!(
                "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                archive_path.display(),
                extract_dir.display()
            ),
        ])
        .status()
        .context("Failed to run PowerShell")?;

    if !status.success() {
        bail!("zip extraction failed");
    }

    // バイナリを見つける
    let binary_path = extract_dir.join("gaze.exe");
    if !binary_path.exists() {
        bail!("Binary not found in archive");
    }

    // Windows では実行中のバイナリを直接置き換えられないため、
    // バッチファイルを使って更新を行う
    let batch_path = temp_dir.join("gaze-update.bat");
    let batch_content = format!(
        r#"@echo off
timeout /t 1 /nobreak >nul
copy /y "{}" "{}"
del "{}"
del "%~f0"
"#,
        binary_path.display(),
        target_path.display(),
        binary_path.display()
    );

    fs::write(&batch_path, batch_content)?;

    // バッチファイルを実行
    Command::new("cmd")
        .args(["/C", "start", "/b", batch_path.to_str().unwrap()])
        .spawn()
        .context("Failed to start update batch")?;

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn install_from_zip(_archive_path: &Path, _target_path: &Path, _temp_dir: &Path) -> Result<()> {
    bail!("zip is not supported on this platform");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer_version() {
        assert!(is_newer_version("0.2.0", "0.1.0"));
        assert!(is_newer_version("1.0.0", "0.9.9"));
        assert!(is_newer_version("0.2.1", "0.2.0"));
        assert!(!is_newer_version("0.1.0", "0.2.0"));
        assert!(!is_newer_version("0.2.0", "0.2.0"));
    }
}
