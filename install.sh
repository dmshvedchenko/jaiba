#!/usr/bin/env sh

set -eu

PREFIX="${PREFIX:-$HOME/.local}"
BIN_DIR="$PREFIX/bin"
BIN_NAME="jaiba"
ICON_DIR="$PREFIX/share/pixmaps"
ICON_NAME="jaiba.png"
DESKTOP_DIR="$PREFIX/share/applications"
DESKTOP_NAME="jaiba.desktop"

if [ "${1:-}" = "--uninstall" ]; then
    if [ -f "$BIN_DIR/$BIN_NAME" ]; then
        rm -f "$BIN_DIR/$BIN_NAME"
        echo "Removed $BIN_DIR/$BIN_NAME"
    else
        echo "Nothing installed at $BIN_DIR/$BIN_NAME"
    fi

    if [ -f "$DESKTOP_DIR/$DESKTOP_NAME" ]; then
        rm -f "$DESKTOP_DIR/$DESKTOP_NAME"
        echo "Removed $DESKTOP_DIR/$DESKTOP_NAME"
    fi

    if [ -f "$ICON_DIR/$ICON_NAME" ]; then
        rm -f "$ICON_DIR/$ICON_NAME"
        echo "Removed $ICON_DIR/$ICON_NAME"
    fi

    command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$DESKTOP_DIR" >/dev/null 2>&1 || true

    echo "Note: ~/.config/jaiba (your config, vault path, and themes) was left in place."
    exit 0
fi

command -v cargo >/dev/null 2>&1 || {
    echo "error: cargo not found. Install a Rust toolchain (rustup.rs), Rust 1.85+ required." >&2
    exit 1
}

echo "Building jaiba (release)..."
cargo build --release

mkdir -p "$BIN_DIR"
install -m 755 "target/release/$BIN_NAME" "$BIN_DIR/$BIN_NAME"
echo "Installed to $BIN_DIR/$BIN_NAME"

mkdir -p "$ICON_DIR" "$DESKTOP_DIR"
install -m 644 "assets/icon.png" "$ICON_DIR/$ICON_NAME"
install -m 644 "assets/jaiba.desktop" "$DESKTOP_DIR/$DESKTOP_NAME"
echo "Installed icon to $ICON_DIR/$ICON_NAME"
echo "Installed launcher entry to $DESKTOP_DIR/$DESKTOP_NAME"

command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$DESKTOP_DIR" >/dev/null 2>&1 || true
command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -q "$PREFIX/share/icons/hicolor" >/dev/null 2>&1 || true

case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *)
        echo ""
        echo "warning: $BIN_DIR is not on your PATH."
        echo "Add this to your shell profile:"
        echo "  export PATH=\"$BIN_DIR:\$PATH\""
        ;;
esac

echo ""
echo "Run 'jaiba' to get started. On first launch it creates:"
echo "  ~/.config/jaiba/config.toml"
echo "  ~/.config/jaiba/themes/ (seeded with the built-in themes)"
