#!/usr/bin/env bash
#
# deploy.sh - build Linux release artifacts for jaiba (.deb, .rpm, .AppImage)
#
# Usage:
#   ./deploy.sh              # build everything available for this host
#   ./deploy.sh deb          # just the .deb
#   ./deploy.sh rpm          # just the .rpm
#   ./deploy.sh appimage     # just the .AppImage
#   ./deploy.sh all          # same as no args
#
# Output lands in ./dist/
#
# Requirements (only needed for the target you're building):
#   deb:      cargo install cargo-deb
#   rpm:      cargo install cargo-generate-rpm
#   appimage: appimagetool on PATH, or let this script download it
#             (needs `wget` or `curl`, and FUSE or --appimage-extract-and-run
#             support on the build machine)

set -eu

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT_DIR"

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
SHORT_VERSION="$(printf '%s' "$VERSION" | cut -d. -f1,2)"
ARCH="$(uname -m)"
DIST_DIR="$ROOT_DIR/dist"
TOOLS_DIR="$ROOT_DIR/.deploy-tools"
APPDIR="$ROOT_DIR/target/AppDir"

BIN_NAME="jaiba"
ICON_SIZES="16 22 24 32 48 64 72 96 128 192 256 512"

# Output filename pattern: jaiba_<major.minor>_<arch>.<ext>
# e.g. jaiba_2026.1_amd64.deb, jaiba_2026.1_x86_64.rpm, jaiba_2026.1_x86_64.AppImage
# lowercase, underscores only (extension keeps AppImage's conventional casing).
dist_name() {
    # $1 = arch, $2 = extension
    printf '%s_%s_%s.%s' "$BIN_NAME" "$SHORT_VERSION" "$1" "$2"
}

# Rename the most recently modified file matching a glob in $DIST_DIR to $2.
rename_latest() {
    # $1 = glob (e.g. "*.deb"), $2 = target filename
    local built target
    built="$(ls -t "$DIST_DIR"/$1 2>/dev/null | head -1)"
    [ -n "$built" ] || { warn "no file matching '$1' found in $DIST_DIR to rename"; return 1; }

    target="$DIST_DIR/$2"
    if [ "$built" != "$target" ]; then
        mv -f "$built" "$target"
    fi
    printf '%s' "$target"
}

log()  { printf '\033[1;34m==>\033[0m %s\n' "$1"; }
warn() { printf '\033[1;33m!! \033[0m%s\n' "$1" >&2; }
die()  { printf '\033[1;31merror:\033[0m %s\n' "$1" >&2; exit 1; }

command -v cargo >/dev/null 2>&1 || die "cargo not found. Install a Rust toolchain (rustup.rs)."

mkdir -p "$DIST_DIR"

build_release_binary() {
    log "Building release binary ($VERSION, $ARCH)..."
    cargo build --release
}

# ---------------------------------------------------------------------------
# .deb via cargo-deb
# ---------------------------------------------------------------------------
build_deb() {
    command -v cargo-deb >/dev/null 2>&1 || {
        warn "cargo-deb not found, installing (cargo install cargo-deb)..."
        cargo install cargo-deb
    }

    log "Building .deb package..."
    cargo deb --no-build --output "$DIST_DIR"

    deb_arch="$(command -v dpkg >/dev/null 2>&1 && dpkg --print-architecture 2>/dev/null || echo amd64)"
    out="$(rename_latest '*.deb' "$(dist_name "$deb_arch" deb)")"
    log "deb done -> $out"
}

# ---------------------------------------------------------------------------
# .rpm via cargo-generate-rpm
# ---------------------------------------------------------------------------
build_rpm() {
    command -v cargo-generate-rpm >/dev/null 2>&1 || {
        warn "cargo-generate-rpm not found, installing (cargo install cargo-generate-rpm)..."
        cargo install cargo-generate-rpm
    }

    log "Building .rpm package..."
    cargo generate-rpm --output "$DIST_DIR/"

    out="$(rename_latest '*.rpm' "$(dist_name "$ARCH" rpm)")"
    log "rpm done -> $out"
}

# ---------------------------------------------------------------------------
# .AppImage - assembled by hand, packaged with appimagetool
# ---------------------------------------------------------------------------
ensure_appimagetool() {
    if command -v appimagetool >/dev/null 2>&1; then
        APPIMAGETOOL="appimagetool"
        return
    fi

    mkdir -p "$TOOLS_DIR"
    APPIMAGETOOL="$TOOLS_DIR/appimagetool-$ARCH.AppImage"
    if [ ! -x "$APPIMAGETOOL" ]; then
        warn "appimagetool not found, downloading a local copy into .deploy-tools/..."
        URL="https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-${ARCH}.AppImage"
        if command -v curl >/dev/null 2>&1; then
            curl -fL "$URL" -o "$APPIMAGETOOL"
        elif command -v wget >/dev/null 2>&1; then
            wget -O "$APPIMAGETOOL" "$URL"
        else
            die "need curl or wget to download appimagetool"
        fi
        chmod +x "$APPIMAGETOOL"
    fi
}

build_appimage() {
    ensure_appimagetool

    log "Assembling AppDir..."
    rm -rf "$APPDIR"
    mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/applications"

    install -m 755 "target/release/$BIN_NAME" "$APPDIR/usr/bin/$BIN_NAME"
    install -m 644 "assets/jaiba.desktop" "$APPDIR/usr/share/applications/jaiba.desktop"
    install -m 644 "assets/jaiba.desktop" "$APPDIR/jaiba.desktop"

    mkdir -p "$APPDIR/usr/share/icons/hicolor/scalable/apps"
    install -m 644 "assets/jaiba.svg" "$APPDIR/usr/share/icons/hicolor/scalable/apps/jaiba.svg"

    for s in $ICON_SIZES; do
        src="assets/icon_${s}x${s}.png"
        [ -f "$src" ] || continue
        dest_dir="$APPDIR/usr/share/icons/hicolor/${s}x${s}/apps"
        mkdir -p "$dest_dir"
        install -m 644 "$src" "$dest_dir/jaiba.png"
    done

    # Top-level icon: AppImage/appimagetool wants a .DirIcon and a
    # top-level <name>.png/svg next to AppRun. Use the 256px raster.
    install -m 644 "assets/icon_256x256.png" "$APPDIR/jaiba.png"
    cp "$APPDIR/jaiba.png" "$APPDIR/.DirIcon"

    cat > "$APPDIR/AppRun" <<'EOF'
#!/usr/bin/env sh
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/jaiba" "$@"
EOF
    chmod 755 "$APPDIR/AppRun"

    log "Running appimagetool..."
    OUT="$DIST_DIR/$(dist_name "$ARCH" AppImage)"
    ARCH="$ARCH" "$APPIMAGETOOL" "$APPDIR" "$OUT"
    chmod 755 "$OUT"
    log "AppImage done -> $OUT"
}

# ---------------------------------------------------------------------------
main() {
    TARGET="${1:-all}"

    build_release_binary

    case "$TARGET" in
        deb)      build_deb ;;
        rpm)      build_rpm ;;
        appimage) build_appimage ;;
        all)
            build_deb      || warn "deb build failed, continuing"
            build_rpm      || warn "rpm build failed, continuing"
            build_appimage || warn "AppImage build failed, continuing"
            ;;
        *)
            die "unknown target '$TARGET' (use: deb, rpm, appimage, all)"
            ;;
    esac

    log "Artifacts in $DIST_DIR:"
    ls -lh "$DIST_DIR" 2>/dev/null || true
}

main "$@"