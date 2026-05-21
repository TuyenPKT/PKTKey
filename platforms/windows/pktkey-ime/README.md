# PKTKey IME — Windows TSF

Bộ gõ tiếng Việt trên Windows, tích hợp qua **Text Services Framework (TSF)**.  
Hỗ trợ toàn bộ tính năng của Telex Codex (engine `pktkey-core`).

---

## Yêu cầu

- Windows 10/11 (64-bit)
- Rust toolchain với target MSVC:
  ```powershell
  rustup target add x86_64-pc-windows-msvc
  ```
- Visual Studio Build Tools (C++ workload) — để link MSVC runtime
- Đã thêm **Vietnamese** vào input languages:
  `Windows Settings > Time & Language > Language & Region > Add a language → Vietnamese`

---

## Build

```powershell
cd platforms\windows\pktkey-ime

# Debug (dev)
cargo build --target x86_64-pc-windows-msvc

# Release
cargo build --release --target x86_64-pc-windows-msvc
```

Output: `target\x86_64-pc-windows-msvc\release\pktkey_ime.dll`

---

## Cài đặt

Chạy **PowerShell as Administrator**:

```powershell
# Copy DLL vào thư mục hệ thống (hoặc bất kỳ đường dẫn cố định nào)
Copy-Item .\target\x86_64-pc-windows-msvc\release\pktkey_ime.dll C:\PKTKey\pktkey_ime.dll

# Đăng ký IME với Windows
regsvr32 C:\PKTKey\pktkey_ime.dll
```

> `DllRegisterServer` sẽ tự ghi registry dựa trên vị trí DLL hiện tại.  
> **Không di chuyển DLL sau khi đăng ký** — nếu cần di chuyển, unregister trước.

---

## Gỡ cài đặt

```powershell
regsvr32 /u C:\PKTKey\pktkey_ime.dll
```

---

## Sử dụng

1. Vào `Settings > Time & Language > Typing > Advanced keyboard settings`
2. Override for Vietnamese → chọn **PKTKey Vietnamese IME**  
   (hoặc dùng Win+Space / Alt+Shift để switch input method)

**Toggle Vi/En**: `Ctrl+Space`

**Preset mặc định**: Telex Codex  
- `aa→â`, `oo→ô`, `uw→ư`, `ow→ơ`, `dd→đ`
- `s=sắc`, `f=huyền`, `r=hỏi`, `x=ngã`, `j=nặng`
- Nhấn đôi để escape (vd: `ass→as`, `gooo→goo`)

---

## Registry entries

`DllRegisterServer` ghi:

```
HKLM\SOFTWARE\Classes\CLSID\{C5B5E23A-8F4D-4B1A-9AC0-1D2F3E4B5C6D}
    InProcServer32 = "C:\PKTKey\pktkey_ime.dll"
    ThreadingModel = "Apartment"

HKLM\SOFTWARE\Microsoft\CTF\TIP\{C5B5E23A-8F4D-4B1A-9AC0-1D2F3E4B5C6D}
    LanguageProfile\0x0000042a\{D6E5F4A3-2B1C-4D3E-8F0A-9B8C7D6E5F4A}
        Enable = 1
        DisplayDescription = "PKTKey"
```

---

## Cấu trúc code

| File | Vai trò |
|------|---------|
| `lib.rs` | DLL entry points, GUIDs, helpers |
| `factory.rs` | `IClassFactory` — tạo `InputProcessor` |
| `processor.rs` | `ITfTextInputProcessor` — lifecycle (Activate/Deactivate) |
| `keysink.rs` | `ITfKeyEventSink` — nhận key events, feed vào engine |
| `editsession.rs` | `ITfEditSession` — xóa N chars + insert text vào document |
| `register.rs` | Ghi/xóa registry entries |

---

## Troubleshooting

**regsvr32 trả về lỗi 0x80004005**  
→ Chạy PowerShell/cmd as Administrator.

**IME không xuất hiện trong language bar**  
→ Kiểm tra Vietnamese đã được thêm vào language list chưa.  
→ Restart `ctfmon.exe`: `taskkill /f /im ctfmon.exe && start ctfmon.exe`  
→ Hoặc log out rồi log in lại.

**Build lỗi "linker not found"**  
→ Cài Visual Studio Build Tools: `winget install Microsoft.VisualStudio.2022.BuildTools`  
→ Chọn workload "Desktop development with C++"
