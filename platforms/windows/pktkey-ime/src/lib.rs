//! PKTKey Vietnamese IME — Windows TSF Input Processor DLL
//!
//! Exports: DllMain, DllGetClassObject, DllCanUnloadNow,
//!          DllRegisterServer, DllUnregisterServer

mod factory;
mod processor;
mod keysink;
mod editsession;
mod register;

use std::sync::atomic::{AtomicI32, Ordering};
use windows::{
    core::{Interface, GUID, HRESULT},
    Win32::{
        Foundation::{E_POINTER, HINSTANCE, S_FALSE, S_OK},
        System::Com::IClassFactory,
    },
};

// 0x80040111 — not re-exported in windows 0.58 Win32::System::Com
const CLASS_E_CLASSNOTAVAILABLE: HRESULT = HRESULT(0x80040111_u32 as i32);

// ── GUIDs ──────────────────────────────────────────────────────────────────

/// CLSID for the PKTKey IME COM class.
/// Generated randomly — do NOT reuse in other projects.
pub const CLSID_PKTKEY_IME: GUID = GUID {
    data1: 0xC5B5_E23A,
    data2: 0x8F4D,
    data3: 0x4B1A,
    data4: [0x9A, 0xC0, 0x1D, 0x2F, 0x3E, 0x4B, 0x5C, 0x6D],
};

/// GUID for the Vietnamese language profile.
pub const GUID_PROFILE: GUID = GUID {
    data1: 0xD6E5_F4A3,
    data2: 0x2B1C,
    data3: 0x4D3E,
    data4: [0x8F, 0x0A, 0x9B, 0x8C, 0x7D, 0x6E, 0x5F, 0x4A],
};

// ── Global module handle & ref counts ─────────────────────────────────────

struct SyncHINSTANCE(HINSTANCE);
// SAFETY: HINSTANCE is a process-global constant set once in DllMain before
// any other thread can call DllGetClassObject. Never mutated after that.
unsafe impl Sync for SyncHINSTANCE {}
unsafe impl Send for SyncHINSTANCE {}

static DLL_MODULE: std::sync::OnceLock<SyncHINSTANCE> = std::sync::OnceLock::new();
/// Number of active COM objects created by this DLL.
pub static OBJ_COUNT: AtomicI32 = AtomicI32::new(0);
/// Number of LockServer(TRUE) calls outstanding.
pub static LOCK_COUNT: AtomicI32 = AtomicI32::new(0);

pub fn dll_module() -> HINSTANCE {
    DLL_MODULE.get().expect("DLL module handle not set").0
}

// ── DLL entry points ───────────────────────────────────────────────────────

#[no_mangle]
extern "system" fn DllMain(
    hinstance: HINSTANCE,
    reason: u32,
    _reserved: *mut core::ffi::c_void,
) -> bool {
    const DLL_PROCESS_ATTACH: u32 = 1;
    if reason == DLL_PROCESS_ATTACH {
        let _ = DLL_MODULE.set(SyncHINSTANCE(hinstance));
    }
    true
}

/// COM object factory entry point (called by the system when activating the IME).
#[no_mangle]
extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut core::ffi::c_void,
) -> HRESULT {
    unsafe {
        if ppv.is_null() {
            return E_POINTER;
        }
        *ppv = core::ptr::null_mut();

        if *rclsid != CLSID_PKTKEY_IME {
            return CLASS_E_CLASSNOTAVAILABLE;
        }

        let factory: IClassFactory = factory::ClassFactory::new().into();
        // Writes the requested interface pointer into *ppv.
        factory.query(riid, ppv)
    }
}

/// Returns S_OK if the DLL has no outstanding objects and can be unloaded.
#[no_mangle]
extern "system" fn DllCanUnloadNow() -> HRESULT {
    if OBJ_COUNT.load(Ordering::SeqCst) == 0 && LOCK_COUNT.load(Ordering::SeqCst) == 0 {
        S_OK
    } else {
        S_FALSE
    }
}

/// Called by `regsvr32 pktkey_ime.dll`. Writes CLSID + TSF TIP entries to registry.
#[no_mangle]
extern "system" fn DllRegisterServer() -> HRESULT {
    match register::register_server() {
        Ok(()) => S_OK,
        Err(e) => e.code(),
    }
}

/// Called by `regsvr32 /u pktkey_ime.dll`. Removes registry entries.
#[no_mangle]
extern "system" fn DllUnregisterServer() -> HRESULT {
    match register::unregister_server() {
        Ok(()) => S_OK,
        Err(e) => e.code(),
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────

/// Format a GUID as the registry-standard `{XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}` string.
pub fn format_guid(g: &GUID) -> String {
    format!(
        "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        g.data1,
        g.data2,
        g.data3,
        g.data4[0],
        g.data4[1],
        g.data4[2],
        g.data4[3],
        g.data4[4],
        g.data4[5],
        g.data4[6],
        g.data4[7],
    )
}

/// Encode a &str as a null-terminated UTF-16 Vec for use with PCWSTR.
pub fn to_wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(core::iter::once(0)).collect()
}
