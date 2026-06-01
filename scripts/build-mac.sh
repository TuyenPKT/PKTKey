#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# build-mac.sh — Build PKTKey IME for macOS
# Run từ thư mục gốc workspace: ./scripts/build-mac.sh
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

APP_NAME="PKTKeyIME"
XCODE_PROJECT="platforms/macos/$APP_NAME/$APP_NAME.xcodeproj"
DERIVED_DATA="platforms/macos/$APP_NAME/build"
RELEASE_APP="$DERIVED_DATA/Build/Products/Release/$APP_NAME.app"
# Cài vào /Applications để SMAppService (launch at login) hoạt động đúng
DEST_APP="/Applications/$APP_NAME.app"

# ── 1. Build Rust C API (capi sẽ được embed vào Xcode target) ─────────────
echo "▶ cargo build pktkey-capi (release)..."
cargo build --release -p pktkey-capi
echo "  ✓ target/release/libpktkey_capi.a"

# ── 2. Build macOS app ────────────────────────────────────────────────────
echo "▶ xcodebuild $APP_NAME Release..."
xcodebuild \
  -project  "$XCODE_PROJECT" \
  -scheme   "$APP_NAME" \
  -configuration Release \
  -derivedDataPath "$DERIVED_DATA" \
  -quiet

echo "  ✓ $RELEASE_APP"

# ── 3. Deploy to Desktop ──────────────────────────────────────────────────
echo "▶ Deploy → $DEST_APP"
pkill -f "$APP_NAME" 2>/dev/null && sleep 0.5 || true
rm -rf "$DEST_APP"
cp -R "$RELEASE_APP" "$DEST_APP"
open "$DEST_APP"

echo ""
echo "✅ Done. PKTKeyIME installed to /Applications."
echo "   Nếu hỏi Accessibility permission → System Settings > Privacy & Security > Accessibility → bật."
echo "   Launch at login: bật trong Settings window của app."
