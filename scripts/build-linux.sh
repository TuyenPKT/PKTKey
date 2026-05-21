#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# build-linux.sh — Build PKTKey IME for Linux desktop
# Platform: IBus engine (ibus-pktkey)
# Run từ thư mục gốc workspace: ./scripts/build-linux.sh
#
# Yêu cầu:
#   sudo apt install libibus-1.0-dev pkg-config   # Ubuntu/Debian
#   sudo dnf install ibus-devel                   # Fedora/RHEL
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

INSTALL_PREFIX="${INSTALL_PREFIX:-/usr/local}"
IBUS_ENGINE_DIR="${IBUS_ENGINE_DIR:-$INSTALL_PREFIX/lib/ibus-pktkey}"
IBUS_COMPONENT_DIR="${IBUS_COMPONENT_DIR:-$INSTALL_PREFIX/share/ibus/component}"

# ── 1. Build pktkey-capi (.so) ────────────────────────────────────────────
echo "▶ cargo build pktkey-capi (release)..."
cargo build --release -p pktkey-capi
SO="target/release/libpktkey_capi.so"
echo "  ✓ $SO"

# ── 2. Build IBus engine daemon ───────────────────────────────────────────
# Crate: platforms/linux/ibus-pktkey (TODO: chưa implement)
LINUX_DIR="platforms/linux/ibus-pktkey"
if [ -f "$LINUX_DIR/Cargo.toml" ]; then
    echo "▶ cargo build ibus-pktkey (release)..."
    (cd "$LINUX_DIR" && cargo build --release)
    echo "  ✓ $LINUX_DIR/target/release/ibus-pktkey"
else
    echo "  ⚠ platforms/linux/ibus-pktkey chưa có — build .so xong, IBus engine TODO."
    echo "    Dùng libpktkey_capi.so để tích hợp thủ công (Fcitx5, XIM, v.v.)."
    exit 0
fi

# ── 3. Install (cần sudo) ─────────────────────────────────────────────────
if [[ "${1:-}" == "--install" ]]; then
    echo "▶ Install → $IBUS_ENGINE_DIR"
    sudo mkdir -p "$IBUS_ENGINE_DIR"
    sudo cp "$LINUX_DIR/target/release/ibus-pktkey" "$IBUS_ENGINE_DIR/"
    sudo cp "$SO" "$IBUS_ENGINE_DIR/"

    echo "▶ Install component XML → $IBUS_COMPONENT_DIR"
    sudo mkdir -p "$IBUS_COMPONENT_DIR"
    sudo cp "$LINUX_DIR/pktkey.xml" "$IBUS_COMPONENT_DIR/"

    echo "▶ ibus restart"
    ibus restart 2>/dev/null || true

    echo ""
    echo "✅ Done. Chọn PKTKey trong IBus Preferences > Input Method."
    echo "   Toggle Vi/En: Ctrl+Space (hoặc Super+Space tùy DE)"
else
    echo ""
    echo "✅ Build xong."
    echo "   Để cài đặt: ./scripts/build-linux.sh --install   (cần sudo)"
fi
