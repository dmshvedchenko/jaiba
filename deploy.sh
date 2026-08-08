#!/usr/bin/env bash
#
# deploy.sh - build Linux release artifacts for jaiba (.deb, .rpm, .AppImage, AUR PKGBUILD)
#
# Usage:
#   ./deploy.sh              # build everything available for this host
#   ./deploy.sh deb          # just the .deb
#   ./deploy.sh rpm          # just the .rpm
#   ./deploy.sh appimage     # just the .AppImage
#   ./deploy.sh aur          # sync + (if on Arch) build the AUR package
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
#   aur:      needs a pushed `vX.Y.Z` git tag on GitHub to compute
#             sha256sums; needs `makepkg` (i.e. an actual Arch machine or
#             container) to build/check the package. Either way, the synced
#             PKGBUILD (and .SRCINFO if built) is always copied into dist/
#             so it ships as a release asset.

set -eu

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT_DIR"

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
ARCH="$(uname -m)"
DIST_DIR="$ROOT_DIR/dist"
TOOLS_DIR="$ROOT_DIR/.deploy-tools"
APPDIR="$ROOT_DIR/target/AppDir"

BIN_NAME="jaiba"
ICON_SIZES="16 22 24 32 48 64 72 96 128 192 256 512"

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
    log "deb done -> $DIST_DIR"
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
    log "rpm done -> $DIST_DIR"
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
    OUT="$DIST_DIR/Jaiba-${VERSION}-${ARCH}.AppImage"
    ARCH="$ARCH" "$APPIMAGETOOL" "$APPDIR" "$OUT"
    chmod 755 "$OUT"
    log "AppImage done -> $OUT"
}

# ---------------------------------------------------------------------------
# AUR - sync PKGBUILD's pkgver/sha256sums with this repo, then (optionally)
# build it locally with makepkg if run on an Arch machine.
# ---------------------------------------------------------------------------
build_aur() {
    PKGBUILD="$ROOT_DIR/packaging/aur/PKGBUILD"
    [ -f "$PKGBUILD" ] || die "missing $PKGBUILD"

    log "Syncing PKGBUILD pkgver to $VERSION..."
    sed -i "s/^pkgver=.*/pkgver=$VERSION/" "$PKGBUILD"
    sed -i "s/^pkgrel=.*/pkgrel=1/" "$PKGBUILD"

    TARBALL_URL="https://github.com/pomboverso/jaiba/archive/v${VERSION}.tar.gz"
    log "Fetching release tarball to compute sha256sum..."
    log "  $TARBALL_URL"
    TMP_TARBALL="$(mktemp)"
    if command -v curl >/dev/null 2>&1; then
        HTTP_CODE=$(curl -fsSL -w '%{http_code}' -o "$TMP_TARBALL" "$TARBALL_URL" || echo "000")
    elif command -v wget >/dev/null 2>&1; then
        wget -q -O "$TMP_TARBALL" "$TARBALL_URL" && HTTP_CODE=200 || HTTP_CODE=000
    else
        die "need curl or wget"
    fi

    if [ "$HTTP_CODE" = "200" ] && [ -s "$TMP_TARBALL" ]; then
        SUM="$(sha256sum "$TMP_TARBALL" | cut -d' ' -f1)"
        sed -i "s/^sha256sums=.*/sha256sums=('$SUM')/" "$PKGBUILD"
        log "sha256sums updated: $SUM"
    else
        warn "couldn't fetch v$VERSION tarball from GitHub (not tagged/pushed yet?)."
        warn "Leaving sha256sums=('SKIP') - push+tag the release first, then re-run 'deploy.sh aur'."
    fi
    rm -f "$TMP_TARBALL"

    if command -v makepkg >/dev/null 2>&1; then
        log "makepkg found, building + checking the package..."
        ( cd "$ROOT_DIR/packaging/aur" && makepkg --printsrcinfo > .SRCINFO && makepkg -sf --noconfirm )
        mkdir -p "$DIST_DIR"
        cp "$ROOT_DIR"/packaging/aur/*.pkg.tar.* "$DIST_DIR/" 2>/dev/null || true
        log "AUR package built -> $DIST_DIR"
    else
        warn "makepkg not found (this isn't Arch) - PKGBUILD synced, but not built/checked."
        warn "Run this target on an Arch machine/container to also produce & verify a .pkg.tar.zst."
    fi

    # Always publish the PKGBUILD itself (and .SRCINFO, if we made one) into
    # dist/, so it ships as a release asset alongside the deb/rpm/AppImage -
    # not just something buried in packaging/aur/.
    mkdir -p "$DIST_DIR"
    cp "$PKGBUILD" "$DIST_DIR/PKGBUILD"
    [ -f "$ROOT_DIR/packaging/aur/.SRCINFO" ] && cp "$ROOT_DIR/packaging/aur/.SRCINFO" "$DIST_DIR/.SRCINFO"
    log "PKGBUILD published -> $DIST_DIR/PKGBUILD"
}

# ---------------------------------------------------------------------------
main() {
    TARGET="${1:-all}"

    build_release_binary

    case "$TARGET" in
        deb)      build_deb ;;
        rpm)      build_rpm ;;
        appimage) build_appimage ;;
        aur)      build_aur ;;
        all)
            build_deb      || warn "deb build failed, continuing"
            build_rpm      || warn "rpm build failed, continuing"
            build_appimage || warn "AppImage build failed, continuing"
            build_aur      || warn "AUR sync/build failed, continuing"
            ;;
        *)
            die "unknown target '$TARGET' (use: deb, rpm, appimage, aur, all)"
            ;;
    esac

    log "Artifacts in $DIST_DIR:"
    ls -lh "$DIST_DIR" 2>/dev/null || true
}

main "$@"
