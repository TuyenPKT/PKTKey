# PKTKey

Bộ gõ tiếng Việt đa nền tảng — Telex Codex engine viết bằng Rust.

---

## Build

### macOS

```bash
./scripts/build-mac.sh
```

Build `pktkey-capi` (Rust release) + xcodebuild macOS app → deploy lên Desktop.  
Lần đầu sau khi mở app: **System Settings → Privacy & Security → Accessibility → bật PKTKeyIME**.

---

### Windows

Chạy **PowerShell as Administrator**:

```powershell
# Chỉ build DLL
.\scripts\build-win.ps1

# Build + tự động cài (copy + regsvr32)
.\scripts\build-win.ps1 -Install

# Tuỳ chỉnh thư mục cài
.\scripts\build-win.ps1 -Install -InstallDir "D:\Apps\PKTKey"
```

Yêu cầu: Rust toolchain MSVC (`rustup target add x86_64-pc-windows-msvc`), Visual Studio Build Tools.  
Sau khi cài: **Settings → Time & Language → Language & Region → thêm Vietnamese → chọn PKTKey Vietnamese IME**.

---

### Linux

```bash
# Build libpktkey_capi.so
./scripts/build-linux.sh

# Build + install (cần sudo, yêu cầu libibus-dev)
./scripts/build-linux.sh --install
```

Yêu cầu: `sudo apt install libibus-1.0-dev` (Ubuntu/Debian) hoặc `sudo dnf install ibus-devel` (Fedora).  
> IBus engine (`platforms/linux/ibus-pktkey`) đang phát triển. Hiện script xuất `libpktkey_capi.so`.

---

## Kiểu gõ (Telex Codex mặc định)

| Tổ hợp | Kết quả |
|--------|---------|
| `aa` | â |
| `oo` | ô |
| `ow` | ơ |
| `uw` | ư |
| `aw` | ă (cần có phụ âm đầu) |
| `dd` | đ |
| `s` | sắc (á) |
| `f` | huyền (à) |
| `r` | hỏi (ả) |
| `x` | ngã (ã) |
| `j` | nặng (ạ) |

**Toggle Vi/En**: `Ctrl+Space` (Windows/Linux) · macOS dùng menu bar.  
**Escape double-press**: gõ phím tone/modifier 2 lần để hoàn tác (vd: `ass` → `as`).

---

## Cấu trúc

```
crates/
  core/        Engine lõi (Rust, không phụ thuộc platform)
  capi/        C API wrapper (cdylib + staticlib)
  wasm/        WASM build cho web
platforms/
  macos/       PKTKeyIME — Swift/AppKit, dùng CGEventTap
  windows/     pktkey-ime — Rust, TSF COM DLL
  linux/       ibus-pktkey — TODO
scripts/
  build-mac.sh
  build-win.ps1
  build-linux.sh
config/
  telex.toml          Telex Codex preset
  telex_original.toml Telex gốc (UniKey-compatible)
```

---

## Phát triển

```bash
# Check toàn workspace
cargo check --workspace --all-targets

# Test
cargo test --workspace

# Lint
cargo clippy --workspace --all-targets -- -D warnings
```
