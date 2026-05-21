//! ITfKeyEventSink — intercepts all keyboard events and feeds them to the engine.
//!
//! OnTestKeyDown: preview — return TRUE to claim keys we may want to handle.
//! OnKeyDown:     actually process the key; return TRUE = consumed, FALSE = pass through.

use std::sync::{Arc, Mutex};

use pktkey_core::{EngineOutput, InputMode};
use windows::{
    core::{implement, Result, GUID},
    Win32::{
        Foundation::{BOOL, E_FAIL, S_OK},
        UI::{
            Input::KeyboardAndMouse::{
                GetKeyState, VK_BACK, VK_CAPITAL, VK_CONTROL, VK_SHIFT, VK_SPACE,
            },
            TextServices::{
                ITfContext, ITfEditSession, ITfKeyEventSink_Impl, TF_ES_READWRITE, TF_ES_SYNC,
            },
        },
    },
};

use crate::{editsession::EditSession, processor::ImeState};

// ── KeyEventSink ────────────────────────────────────────────────────────────

#[implement(windows::Win32::UI::TextServices::ITfKeyEventSink)]
pub struct KeyEventSink {
    state: Arc<Mutex<ImeState>>,
}

impl KeyEventSink {
    pub fn new(state: Arc<Mutex<ImeState>>) -> Self {
        KeyEventSink { state }
    }
}

impl ITfKeyEventSink_Impl for KeyEventSink {
    /// Called when input focus changes. Clear composition on focus loss.
    fn OnSetFocus(&self, fforeground: BOOL) -> Result<()> {
        if !fforeground.as_bool() {
            self.state.lock().unwrap().engine.reset_buffer();
        }
        Ok(())
    }

    /// Preview: return TRUE for keys we might consume so OnKeyDown is called next.
    fn OnTestKeyDown(
        &self,
        _pic: Option<&ITfContext>,
        wparam: windows::Win32::Foundation::WPARAM,
        _lparam: windows::Win32::Foundation::LPARAM,
    ) -> Result<BOOL> {
        let vk = wparam.0 as u32;

        // Never intercept pure modifier key presses.
        if is_modifier_vk(vk) {
            return Ok(BOOL(0));
        }

        // In English mode pass everything through.
        if self.state.lock().unwrap().engine.mode == InputMode::English {
            // Except Ctrl+Space which we use to toggle mode.
            let ctrl = unsafe { GetKeyState(VK_CONTROL.0 as i32) as u16 } & 0x8000 != 0;
            if vk == VK_SPACE.0 as u32 && ctrl {
                return Ok(BOOL(1));
            }
            return Ok(BOOL(0));
        }

        // Vietnamese mode: claim printable ASCII, space, and backspace.
        Ok(BOOL(wants_key(vk) as i32))
    }

    fn OnTestKeyUp(
        &self,
        _pic: Option<&ITfContext>,
        _wparam: windows::Win32::Foundation::WPARAM,
        _lparam: windows::Win32::Foundation::LPARAM,
    ) -> Result<BOOL> {
        Ok(BOOL(0))
    }

    /// Main processing: call engine, then modify text in context via edit session.
    fn OnKeyDown(
        &self,
        pic: Option<&ITfContext>,
        wparam: windows::Win32::Foundation::WPARAM,
        _lparam: windows::Win32::Foundation::LPARAM,
    ) -> Result<BOOL> {
        let vk = wparam.0 as u32;
        let ctrl = unsafe { GetKeyState(VK_CONTROL.0 as i32) as u16 } & 0x8000 != 0;

        // Ctrl+Space: toggle Vi/En mode.
        if vk == VK_SPACE.0 as u32 && ctrl {
            self.state.lock().unwrap().engine.toggle_mode();
            return Ok(BOOL(1)); // consumed
        }

        // Convert VK code to a character for the engine.
        let output = {
            let mut s = self.state.lock().unwrap();
            if vk == VK_BACK.0 as u32 {
                s.engine.process_backspace()
            } else if let Some(ch) = vk_to_char(vk) {
                s.engine.process_key(ch)
            } else {
                return Ok(BOOL(0)); // not our key
            }
        };

        match output {
            // Engine did nothing — let the system handle the key.
            EngineOutput::Passthrough => Ok(BOOL(0)),

            // Replace: delete N chars then insert converted text.
            EngineOutput::Replace { delete_back, text } => {
                submit_edit(pic, &self.state, delete_back, text)
            }

            // Commit: pure insertion (e.g. platform-triggered commit with no space).
            EngineOutput::Commit { text } => {
                submit_edit(pic, &self.state, 0, text)
            }
        }
    }

    fn OnKeyUp(
        &self,
        _pic: Option<&ITfContext>,
        _wparam: windows::Win32::Foundation::WPARAM,
        _lparam: windows::Win32::Foundation::LPARAM,
    ) -> Result<BOOL> {
        Ok(BOOL(0))
    }

    fn OnPreservedKey(
        &self,
        _pic: Option<&ITfContext>,
        _rguid: *const GUID,
    ) -> Result<BOOL> {
        Ok(BOOL(0))
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Request a synchronous read-write edit session and perform delete+insert.
fn submit_edit(
    pic: Option<&ITfContext>,
    state: &std::sync::Mutex<crate::processor::ImeState>,
    delete_back: usize,
    text: String,
) -> Result<BOOL> {
    let context = pic.ok_or(E_FAIL)?;
    let client_id = state.lock().unwrap().client_id;

    let session: ITfEditSession =
        EditSession::new(context.clone(), delete_back, text).into();

    let mut hr = S_OK;
    unsafe {
        context.RequestEditSession(
            client_id,
            &session,
            TF_ES_SYNC | TF_ES_READWRITE,
            &mut hr,
        )?;
    }
    hr.ok()?;
    Ok(BOOL(1)) // consumed
}

/// True if the engine should be offered this VK code (Vietnamese mode).
fn wants_key(vk: u32) -> bool {
    matches!(vk,
        0x08        // VK_BACK
        | 0x20      // VK_SPACE
        | 0x41..=0x5A  // A-Z
        | 0x30..=0x39  // 0-9
        | 0xBD | 0xBB | 0xBC | 0xBE | 0xBF  // - = , . /
        | 0xBA | 0xDE | 0xDB | 0xDD | 0xDC | 0xC0  // ; ' [ ] \ `
    )
}

/// True if VK is a pure modifier key (Shift, Ctrl, Alt, Win, CapsLock).
fn is_modifier_vk(vk: u32) -> bool {
    matches!(vk, 0x10 | 0x11 | 0x12 | 0x14 | 0x5B | 0x5C)
}

/// Convert a Virtual-Key code to a `char` using the current keyboard state.
/// Handles standard US-QWERTY layout. Vietnamese input typically uses US layout.
fn vk_to_char(vk: u32) -> Option<char> {
    let shift = unsafe { GetKeyState(VK_SHIFT.0 as i32) as u16 } & 0x8000 != 0;
    let caps  = unsafe { GetKeyState(VK_CAPITAL.0 as i32) as u16 } & 0x0001 != 0;
    let upper = shift ^ caps;

    match vk {
        // Letters A–Z (VK_A = 0x41)
        0x41..=0x5A => {
            let c = (b'a' + (vk - 0x41) as u8) as char;
            Some(if upper { c.to_ascii_uppercase() } else { c })
        }
        // Digits 0–9 (VK_0 = 0x30)
        0x30 => Some(if shift { ')' } else { '0' }),
        0x31 => Some(if shift { '!' } else { '1' }),
        0x32 => Some(if shift { '@' } else { '2' }),
        0x33 => Some(if shift { '#' } else { '3' }),
        0x34 => Some(if shift { '$' } else { '4' }),
        0x35 => Some(if shift { '%' } else { '5' }),
        0x36 => Some(if shift { '^' } else { '6' }),
        0x37 => Some(if shift { '&' } else { '7' }),
        0x38 => Some(if shift { '*' } else { '8' }),
        0x39 => Some(if shift { '(' } else { '9' }),
        // Space
        0x20 => Some(' '),
        // OEM keys (US layout)
        0xBD => Some(if shift { '_' } else { '-' }),  // VK_OEM_MINUS
        0xBB => Some(if shift { '+' } else { '=' }),  // VK_OEM_PLUS
        0xBC => Some(if shift { '<' } else { ',' }),  // VK_OEM_COMMA
        0xBE => Some(if shift { '>' } else { '.' }),  // VK_OEM_PERIOD
        0xBF => Some(if shift { '?' } else { '/' }),  // VK_OEM_2
        0xBA => Some(if shift { ':' } else { ';' }),  // VK_OEM_1
        0xDE => Some(if shift { '"' } else { '\'' }), // VK_OEM_7
        0xDB => Some(if shift { '{' } else { '[' }),  // VK_OEM_4
        0xDD => Some(if shift { '}' } else { ']' }),  // VK_OEM_6
        0xDC => Some(if shift { '|' } else { '\\' }), // VK_OEM_5
        0xC0 => Some(if shift { '~' } else { '`' }),  // VK_OEM_3
        _ => None,
    }
}
