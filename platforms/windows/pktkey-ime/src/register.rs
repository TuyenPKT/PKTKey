//! DllRegisterServer / DllUnregisterServer — register the IME with TSF.
//!
//! windows 0.58: registration is done through the official TSF COM API
//! (`ITfInputProcessorProfiles` + `ITfCategoryMgr`) instead of writing the
//! `CTF\TIP` registry tree by hand. Hand-written registry entries are fragile
//! and, crucially, omit the category records that Windows 8+/11 require before
//! it will list the IME in "Add a keyboard".
//!
//! The only registry we still write ourselves is the COM `InProcServer32`
//! entry — `CoCreateInstance` needs it to locate this DLL.
//!
//! The IME is registered under Vietnamese (LANGID 0x042A).

use windows::{
    core::{Result, GUID},
    Win32::{
        Foundation::E_FAIL,
        System::{
            Com::{
                CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
                COINIT_APARTMENTTHREADED,
            },
            LibraryLoader::GetModuleFileNameW,
            Registry::{
                RegCreateKeyExW, RegDeleteTreeW, RegSetValueExW, HKEY, HKEY_LOCAL_MACHINE,
                KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ,
            },
        },
        UI::TextServices::{
            ITfCategoryMgr, ITfInputProcessorProfiles, CLSID_TF_CategoryMgr,
            CLSID_TF_InputProcessorProfiles, GUID_TFCAT_TIPCAP_COMLESS,
            GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT, GUID_TFCAT_TIPCAP_INPUTMODECOMPARTMENT,
            GUID_TFCAT_TIPCAP_SECUREMODE, GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT,
            GUID_TFCAT_TIPCAP_UIELEMENTENABLED, GUID_TFCAT_TIP_KEYBOARD,
        },
    },
};

use crate::{dll_module, format_guid, to_wide_null, CLSID_PKTKEY_IME, GUID_PROFILE};

const IME_NAME: &str = "PKTKey Vietnamese IME";

/// LANGID for Vietnamese (vi-VN).
const LANGID_VI: u16 = 0x042A;

/// Categories the IME registers itself into. `TIP_KEYBOARD` makes it a keyboard
/// TIP; the `TIPCAP_*` entries advertise capabilities that Windows 8+/11 needs
/// (notably `IMMERSIVESUPPORT` and `SYSTRAYSUPPORT`) before listing/loading it.
const CATEGORIES: &[GUID] = &[
    GUID_TFCAT_TIP_KEYBOARD,
    GUID_TFCAT_TIPCAP_SECUREMODE,
    GUID_TFCAT_TIPCAP_UIELEMENTENABLED,
    GUID_TFCAT_TIPCAP_COMLESS,
    GUID_TFCAT_TIPCAP_INPUTMODECOMPARTMENT,
    GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT,
    GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT,
];

pub fn register_server() -> Result<()> {
    // ── 1. COM in-proc server (registry — required so COM can find the DLL). ──
    let dll_path = get_dll_path()?;
    let clsid = format_guid(&CLSID_PKTKEY_IME);
    let clsid_root = format!("SOFTWARE\\Classes\\CLSID\\{}", clsid);
    let inproc = format!("{}\\InProcServer32", clsid_root);

    reg_set_string(HKEY_LOCAL_MACHINE, &clsid_root, "", IME_NAME)?;
    reg_set_string(HKEY_LOCAL_MACHINE, &inproc, "", &dll_path)?;
    reg_set_string(HKEY_LOCAL_MACHINE, &inproc, "ThreadingModel", "Apartment")?;

    // ── 2. TSF registration via COM. ─────────────────────────────────────────
    let _com = ComGuard::new();
    unsafe {
        let profiles: ITfInputProcessorProfiles =
            CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)?;
        profiles.Register(&CLSID_PKTKEY_IME)?;

        // Description buffer is null-terminated so TSF writes a clean REG_SZ;
        // we pass cch = text length (excluding the null), per AddLanguageProfile's
        // contract. Icon comes from this DLL (index 0); the path must be a real,
        // null-terminated string — an empty slice gives a dangling pointer.
        let desc_buf: Vec<u16> = IME_NAME.encode_utf16().chain(std::iter::once(0)).collect();
        let icon_buf: Vec<u16> = dll_path.encode_utf16().chain(std::iter::once(0)).collect();
        profiles.AddLanguageProfile(
            &CLSID_PKTKEY_IME,
            LANGID_VI,
            &GUID_PROFILE,
            &desc_buf[..desc_buf.len() - 1],
            &icon_buf[..icon_buf.len() - 1],
            0,
        )?;

        let category_mgr: ITfCategoryMgr =
            CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)?;
        for cat in CATEGORIES {
            category_mgr.RegisterCategory(&CLSID_PKTKEY_IME, cat, &CLSID_PKTKEY_IME)?;
        }
    }

    Ok(())
}

pub fn unregister_server() -> Result<()> {
    // ── 1. Tear down TSF registration (best-effort — ignore per-step errors). ─
    let _com = ComGuard::new();
    unsafe {
        if let Ok(category_mgr) = CoCreateInstance::<_, ITfCategoryMgr>(
            &CLSID_TF_CategoryMgr,
            None,
            CLSCTX_INPROC_SERVER,
        ) {
            for cat in CATEGORIES {
                let _ = category_mgr.UnregisterCategory(&CLSID_PKTKEY_IME, cat, &CLSID_PKTKEY_IME);
            }
        }
        if let Ok(profiles) = CoCreateInstance::<_, ITfInputProcessorProfiles>(
            &CLSID_TF_InputProcessorProfiles,
            None,
            CLSCTX_INPROC_SERVER,
        ) {
            let _ = profiles.RemoveLanguageProfile(&CLSID_PKTKEY_IME, LANGID_VI, &GUID_PROFILE);
            let _ = profiles.Unregister(&CLSID_PKTKEY_IME);
        }
    }

    // ── 2. Remove the COM registry tree. ─────────────────────────────────────
    let clsid = format_guid(&CLSID_PKTKEY_IME);
    let clsid_root = format!("SOFTWARE\\Classes\\CLSID\\{}", clsid);
    reg_delete_tree(HKEY_LOCAL_MACHINE, &clsid_root)?;

    Ok(())
}

// ── COM init guard ───────────────────────────────────────────────────────────

/// RAII COM apartment init. `regsvr32` does not initialize COM before calling
/// `DllRegisterServer`, so we must do it ourselves for `CoCreateInstance`.
struct ComGuard {
    uninit: bool,
}

impl ComGuard {
    fn new() -> Self {
        // S_OK / S_FALSE → we initialized it and must balance with CoUninitialize.
        // Any error (e.g. RPC_E_CHANGED_MODE) → COM is already up in another mode;
        // it is still usable, but we must NOT call CoUninitialize.
        let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        ComGuard { uninit: hr.is_ok() }
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.uninit {
            unsafe { CoUninitialize() };
        }
    }
}

// ── Registry helpers (InProcServer32 only) ───────────────────────────────────

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

    let bytes = unsafe { std::slice::from_raw_parts(dat.as_ptr() as *const u8, dat.len() * 2) };
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

fn reg_delete_tree(root: HKEY, subkey: &str) -> Result<()> {
    let sk = to_wide_null(subkey);
    let status = unsafe { RegDeleteTreeW(root, windows::core::PCWSTR(sk.as_ptr())) };
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
    // windows 0.58: GetModuleFileNameW takes (impl Param<HMODULE>, &mut [u16]).
    let len = unsafe { GetModuleFileNameW(module, &mut buf) };
    if len == 0 {
        return Err(E_FAIL.into());
    }
    Ok(String::from_utf16_lossy(&buf[..len as usize]))
}
