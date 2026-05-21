//! DllRegisterServer / DllUnregisterServer — write/delete Windows registry entries.
//!
//! Registration writes two trees:
//!   HKLM\SOFTWARE\Classes\CLSID\{CLSID}            — COM InProcServer32
//!   HKLM\SOFTWARE\Microsoft\CTF\TIP\{CLSID}        — TSF TIP Language Profile
//!
//! The IME is registered under Vietnamese (LANGID 0x042A).
//! Users must add "Vietnamese" as an input language in Windows Settings > Time & Language.

use windows::{
    core::Result,
    Win32::{
        Foundation::E_FAIL,
        System::Registry::{
            RegCreateKeyExW, RegDeleteTreeW, RegSetValueExW, HKEY, HKEY_LOCAL_MACHINE,
            KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ, REG_DWORD,
        },
        System::LibraryLoader::GetModuleFileNameW,
    },
};

use crate::{dll_module, format_guid, to_wide_null, CLSID_PKTKEY_IME, GUID_PROFILE};

const IME_NAME: &str = "PKTKey Vietnamese IME";
const IME_SHORT: &str = "PKTKey";

/// LANGID for Vietnamese (vi-VN).
const LANGID_VI: u32 = 0x0000_042A;

pub fn register_server() -> Result<()> {
    let dll_path = get_dll_path()?;
    let clsid = format_guid(&CLSID_PKTKEY_IME);
    let profile = format_guid(&GUID_PROFILE);

    // ── COM CLSID ──────────────────────────────────────────────────────────
    let clsid_root = format!("SOFTWARE\\Classes\\CLSID\\{}", clsid);
    let inproc = format!("{}\\InProcServer32", clsid_root);

    reg_set_string(HKEY_LOCAL_MACHINE, &clsid_root, "", IME_NAME)?;
    reg_set_string(HKEY_LOCAL_MACHINE, &inproc, "", &dll_path)?;
    reg_set_string(HKEY_LOCAL_MACHINE, &inproc, "ThreadingModel", "Apartment")?;

    // ── TSF TIP Language Profile ───────────────────────────────────────────
    let tip_root = format!("SOFTWARE\\Microsoft\\CTF\\TIP\\{}", clsid);
    let lang_key = format!(
        "{}\\LanguageProfile\\{:#010x}\\{}",
        tip_root, LANGID_VI, profile
    );

    reg_set_string(HKEY_LOCAL_MACHINE, &tip_root, "Description", IME_NAME)?;
    reg_set_dword(HKEY_LOCAL_MACHINE, &lang_key, "Enable", 1)?;
    reg_set_dword(HKEY_LOCAL_MACHINE, &lang_key, "BitmapIndex", 0)?;
    reg_set_string(HKEY_LOCAL_MACHINE, &lang_key, "DisplayDescription", IME_SHORT)?;

    Ok(())
}

pub fn unregister_server() -> Result<()> {
    let clsid = format_guid(&CLSID_PKTKEY_IME);

    let clsid_root = format!("SOFTWARE\\Classes\\CLSID\\{}", clsid);
    let tip_root = format!("SOFTWARE\\Microsoft\\CTF\\TIP\\{}", clsid);

    reg_delete_tree(HKEY_LOCAL_MACHINE, &clsid_root)?;
    reg_delete_tree(HKEY_LOCAL_MACHINE, &tip_root)?;

    Ok(())
}

// ── Registry helpers ────────────────────────────────────────────────────────

fn reg_set_string(root: HKEY, subkey: &str, value: &str, data: &str) -> Result<()> {
    let sk = to_wide_null(subkey);
    let val = to_wide_null(value);
    let dat = to_wide_null(data);

    let mut hkey = HKEY::default();
    let status = unsafe {
        RegCreateKeyExW(
            root,
            windows::core::PCWSTR(sk.as_ptr()),
            0,
            windows::core::PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut hkey,
            None,
        )
    };
    status.ok()?;

    let bytes = unsafe {
        std::slice::from_raw_parts(dat.as_ptr() as *const u8, dat.len() * 2)
    };
    let status = unsafe {
        RegSetValueExW(
            hkey,
            windows::core::PCWSTR(val.as_ptr()),
            0,
            REG_SZ,
            Some(bytes),
        )
    };
    status.ok()?;
    Ok(())
}

fn reg_set_dword(root: HKEY, subkey: &str, value: &str, data: u32) -> Result<()> {
    let sk = to_wide_null(subkey);
    let val = to_wide_null(value);

    let mut hkey = HKEY::default();
    let status = unsafe {
        RegCreateKeyExW(
            root,
            windows::core::PCWSTR(sk.as_ptr()),
            0,
            windows::core::PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut hkey,
            None,
        )
    };
    status.ok()?;

    let bytes = data.to_ne_bytes();
    let status = unsafe {
        RegSetValueExW(
            hkey,
            windows::core::PCWSTR(val.as_ptr()),
            0,
            REG_DWORD,
            Some(&bytes),
        )
    };
    status.ok()?;
    Ok(())
}

fn reg_delete_tree(root: HKEY, subkey: &str) -> Result<()> {
    let sk = to_wide_null(subkey);
    let status = unsafe {
        RegDeleteTreeW(root, windows::core::PCWSTR(sk.as_ptr()))
    };
    // Ignore "key not found" (2 = ERROR_FILE_NOT_FOUND): already unregistered.
    if status.0 != 0 && status.0 != 2 {
        status.ok()?;
    }
    Ok(())
}

/// Get the full path of this DLL (used in InProcServer32 registration).
fn get_dll_path() -> Result<String> {
    let module = dll_module();
    let mut buf = vec![0u16; 520];
    let len = unsafe {
        GetModuleFileNameW(
            Some(module),
            windows::core::PWSTR(buf.as_mut_ptr()),
            buf.len() as u32,
        )
    };
    if len == 0 {
        return Err(E_FAIL.into());
    }
    Ok(String::from_utf16_lossy(&buf[..len as usize]))
}
