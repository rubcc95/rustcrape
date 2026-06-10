#!/bin/bash
set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "==> Building sidecar binary..."
cd "$PROJECT_DIR/sidecar"
bun run build

case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*) EXT=".exe" ;;
    *) EXT="" ;;
esac

TARGET=$(rustc -vV 2>/dev/null | grep "^host:" | awk '{print $2}')
if [ -n "$TARGET" ]; then
    cp -f "$PROJECT_DIR/src-tauri/binaries/scraper-sidecar$EXT" "$PROJECT_DIR/src-tauri/binaries/scraper-sidecar-${TARGET}${EXT}"
    echo "==> Sidecar built for target: $TARGET"
else
    echo "==> WARNING: could not detect target triple. sidecar binary may not work with 'cargo tauri build'"
fi
