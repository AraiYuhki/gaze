#!/bin/sh
# Gaze インストールスクリプト
# Usage: curl -fsSL https://raw.githubusercontent.com/AraiYuhki/gaze/master/install.sh | sh

set -e

REPO="AraiYuhki/gaze"
BINARY_NAME="gaze"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

# 色付き出力
info() {
    printf '\033[1;34m%s\033[0m\n' "$1"
}

success() {
    printf '\033[1;32m%s\033[0m\n' "$1"
}

error() {
    printf '\033[1;31mError: %s\033[0m\n' "$1" >&2
    exit 1
}

# OS とアーキテクチャを検出
detect_platform() {
    OS="$(uname -s)"
    ARCH="$(uname -m)"

    case "$OS" in
        Linux)
            case "$ARCH" in
                x86_64) PLATFORM="x86_64-unknown-linux-gnu" ;;
                aarch64) PLATFORM="aarch64-unknown-linux-gnu" ;;
                *) error "Unsupported architecture: $ARCH" ;;
            esac
            ;;
        Darwin)
            case "$ARCH" in
                x86_64) PLATFORM="x86_64-apple-darwin" ;;
                arm64) PLATFORM="aarch64-apple-darwin" ;;
                *) error "Unsupported architecture: $ARCH" ;;
            esac
            ;;
        *)
            error "Unsupported OS: $OS (use Scoop for Windows)"
            ;;
    esac
}

# 最新バージョンを取得
get_latest_version() {
    VERSION=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    if [ -z "$VERSION" ]; then
        error "Failed to get latest version"
    fi
}

# ダウンロードして展開
download_and_install() {
    DOWNLOAD_URL="https://github.com/$REPO/releases/download/$VERSION/${BINARY_NAME}-${PLATFORM}.tar.gz"
    
    info "Downloading $BINARY_NAME $VERSION for $PLATFORM..."
    
    TEMP_DIR=$(mktemp -d)
    trap 'rm -rf "$TEMP_DIR"' EXIT
    
    curl -fsSL "$DOWNLOAD_URL" -o "$TEMP_DIR/archive.tar.gz" || error "Failed to download from $DOWNLOAD_URL"
    
    info "Extracting..."
    tar -xzf "$TEMP_DIR/archive.tar.gz" -C "$TEMP_DIR"
    
    # インストールディレクトリを作成
    mkdir -p "$INSTALL_DIR"
    
    # バイナリをコピー
    if [ -f "$TEMP_DIR/$BINARY_NAME" ]; then
        mv "$TEMP_DIR/$BINARY_NAME" "$INSTALL_DIR/"
    elif [ -f "$TEMP_DIR/${BINARY_NAME}-${PLATFORM}/$BINARY_NAME" ]; then
        mv "$TEMP_DIR/${BINARY_NAME}-${PLATFORM}/$BINARY_NAME" "$INSTALL_DIR/"
    else
        error "Binary not found in archive"
    fi
    
    chmod +x "$INSTALL_DIR/$BINARY_NAME"
}

# PATH の確認
check_path() {
    case ":$PATH:" in
        *":$INSTALL_DIR:"*) ;;
        *)
            echo ""
            info "Add the following to your shell profile (.bashrc, .zshrc, etc.):"
            echo ""
            echo "  export PATH=\"\$PATH:$INSTALL_DIR\""
            echo ""
            ;;
    esac
}

main() {
    info "Installing $BINARY_NAME..."
    
    detect_platform
    get_latest_version
    download_and_install
    
    success "Successfully installed $BINARY_NAME $VERSION to $INSTALL_DIR/$BINARY_NAME"
    
    check_path
    
    echo ""
    info "Run '$BINARY_NAME' in a git repository to start!"
}

main
