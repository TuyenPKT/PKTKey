use pktkey_core::{Engine, EngineOutput, MappingConfig, Preset};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

// Output type constants
pub const PKTKEY_REPLACE:      c_int = 0;
pub const PKTKEY_PASSTHROUGH:  c_int = 1;
pub const PKTKEY_COMMIT:       c_int = 2;

/// Output written by process_key / process_backspace.
/// `text` is a heap-allocated C string; caller must free it with `pktkey_free_string`.
#[repr(C)]
pub struct PKTKeyOutput {
    pub output_type:  c_int,
    pub delete_back:  usize,
    /// Only valid when output_type == PKTKEY_REPLACE or PKTKEY_COMMIT.
    /// Null-terminated UTF-8. Caller owns this memory — free with `pktkey_free_string`.
    pub text: *mut c_char,
}

// ── Lifecycle ────────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn pktkey_engine_new(preset: *const c_char) -> *mut Engine {
    let preset_str = if preset.is_null() {
        "telex"
    } else {
        // SAFETY: caller guarantees a valid, null-terminated UTF-8 string
        match unsafe { CStr::from_ptr(preset) }.to_str() {
            Ok(s) => s,
            Err(_) => "telex",
        }
    };
    let p = match preset_str.to_lowercase().as_str() {
        "telex_original" => Preset::TelexOriginal,
        "vni"            => Preset::Vni,
        "viqr"           => Preset::Viqr,
        "direct"         => Preset::Direct,
        "custom"         => Preset::Custom,
        _                => Preset::Telex,
    };
    Box::into_raw(Box::new(Engine::new(MappingConfig::from_preset(p))))
}

#[no_mangle]
pub extern "C" fn pktkey_engine_free(engine: *mut Engine) {
    if engine.is_null() { return; }
    // SAFETY: pointer was created by pktkey_engine_new and not yet freed
    drop(unsafe { Box::from_raw(engine) });
}

// ── Key processing ───────────────────────────────────────────────────────────

/// Process one keypress. Writes result into `out`.
/// Returns 1 on success, 0 if engine is null.
#[no_mangle]
pub extern "C" fn pktkey_process_key(
    engine: *mut Engine,
    key:    *const c_char,
    out:    *mut PKTKeyOutput,
) -> c_int {
    if engine.is_null() || key.is_null() || out.is_null() { return 0; }
    // SAFETY: pointers are non-null and valid for the duration of this call
    let engine = unsafe { &mut *engine };
    let key_str = match unsafe { CStr::from_ptr(key) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };
    let ch = match key_str.chars().next() {
        Some(c) => c,
        None    => return 0,
    };
    write_output(unsafe { &mut *out }, engine.process_key(ch));
    1
}

/// Process a Backspace. Writes result into `out`.
#[no_mangle]
pub extern "C" fn pktkey_process_backspace(
    engine: *mut Engine,
    out:    *mut PKTKeyOutput,
) -> c_int {
    if engine.is_null() || out.is_null() { return 0; }
    // SAFETY: pointers are non-null and valid for the duration of this call
    let engine = unsafe { &mut *engine };
    write_output(unsafe { &mut *out }, engine.process_backspace());
    1
}

/// Returns how many characters the current candidate occupies on screen.
#[no_mangle]
pub extern "C" fn pktkey_candidate_len(engine: *const Engine) -> usize {
    if engine.is_null() { return 0; }
    // SAFETY: pointer is non-null and valid
    unsafe { &*engine }.candidate_len()
}

/// Returns suggestions for the current buffer as a heap-allocated array of C strings.
/// `count_out` is set to the number of suggestions. Returns NULL if no suggestions.
/// Caller must free with `pktkey_free_suggestions(arr, count)`.
#[no_mangle]
pub extern "C" fn pktkey_get_suggestions(
    engine:    *const Engine,
    count_out: *mut usize,
) -> *mut *mut c_char {
    if count_out.is_null() { return std::ptr::null_mut(); }
    if engine.is_null() {
        // SAFETY: count_out is non-null
        unsafe { *count_out = 0; }
        return std::ptr::null_mut();
    }
    // SAFETY: pointer is non-null and valid
    let suggestions = unsafe { &*engine }.get_suggestions();
    let count = suggestions.len();
    // SAFETY: count_out is non-null
    unsafe { *count_out = count; }
    if count == 0 {
        return std::ptr::null_mut();
    }
    let mut ptrs: Vec<*mut c_char> = suggestions
        .iter()
        .map(|s| CString::new(s.as_str()).unwrap_or_default().into_raw())
        .collect();
    let raw = ptrs.as_mut_ptr();
    std::mem::forget(ptrs);
    raw
}

/// Free a suggestions array returned by `pktkey_get_suggestions`.
#[no_mangle]
pub extern "C" fn pktkey_free_suggestions(arr: *mut *mut c_char, count: usize) {
    if arr.is_null() || count == 0 { return; }
    // SAFETY: arr was allocated by pktkey_get_suggestions with exactly `count` elements
    unsafe {
        for i in 0..count {
            let p = *arr.add(i);
            if !p.is_null() {
                drop(CString::from_raw(p));
            }
        }
        drop(Vec::from_raw_parts(arr, count, count));
    }
}

/// Reset the syllable buffer (call when cursor moves, focus changes, etc.)
#[no_mangle]
pub extern "C" fn pktkey_reset_buffer(engine: *mut Engine) {
    if engine.is_null() { return; }
    // SAFETY: pointer is non-null and valid
    unsafe { &mut *engine }.reset_buffer();
}

// ── Mode ─────────────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn pktkey_toggle_mode(engine: *mut Engine) {
    if engine.is_null() { return; }
    // SAFETY: pointer is non-null and valid
    unsafe { &mut *engine }.toggle_mode();
}

/// Returns "vi" or "en". Caller must free with `pktkey_free_string`.
#[no_mangle]
pub extern "C" fn pktkey_get_mode(engine: *const Engine) -> *mut c_char {
    if engine.is_null() {
        return alloc_cstring("vi");
    }
    // SAFETY: pointer is non-null and valid
    let mode = unsafe { &*engine }.mode;
    use pktkey_core::InputMode;
    let s = match mode {
        InputMode::Vietnamese => "vi",
        InputMode::English    => "en",
    };
    alloc_cstring(s)
}

// ── Memory ───────────────────────────────────────────────────────────────────

/// Free a C string returned by this library (e.g. from `pktkey_get_mode`
/// or the `text` field of `PKTKeyOutput`).
#[no_mangle]
pub extern "C" fn pktkey_free_string(s: *mut c_char) {
    if s.is_null() { return; }
    // SAFETY: string was created by CString::into_raw inside this library
    drop(unsafe { CString::from_raw(s) });
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn alloc_cstring(s: &str) -> *mut c_char {
    // Null bytes in s would cause panic; engine output is guaranteed ASCII/UTF-8 without nulls
    CString::new(s).unwrap().into_raw()
}

fn write_output(out: &mut PKTKeyOutput, result: EngineOutput) {
    match result {
        EngineOutput::Replace { delete_back, text } => {
            out.output_type = PKTKEY_REPLACE;
            out.delete_back = delete_back;
            out.text = alloc_cstring(&text);
        }
        EngineOutput::Passthrough => {
            out.output_type = PKTKEY_PASSTHROUGH;
            out.delete_back = 0;
            out.text = std::ptr::null_mut();
        }
        EngineOutput::Commit { text } => {
            out.output_type = PKTKEY_COMMIT;
            out.delete_back = 0;
            out.text = alloc_cstring(&text);
        }
    }
}
