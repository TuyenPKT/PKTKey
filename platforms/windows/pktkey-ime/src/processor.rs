//! ITfTextInputProcessor — lifecycle of the IME inside the TSF thread manager.
//!
//! Activate: register the key-event sink so we receive keyboard events.
//! Deactivate: unregister and release resources.

use std::cell::RefCell;
use std::rc::Rc;

use pktkey_core::{Engine, MappingConfig, Preset};
use windows::{
    core::{implement, Interface, Result},
    Win32::{
        Foundation::{BOOL, E_INVALIDARG},
        UI::TextServices::{
            ITfKeyEventSink, ITfKeystrokeMgr, ITfTextInputProcessor_Impl, ITfThreadMgr,
        },
    },
};

use crate::{keysink::KeyEventSink, OBJ_COUNT};

// ── Shared engine state ─────────────────────────────────────────────────────

/// All mutable IME state, shared between processor and key sink via Rc<RefCell<>>.
/// Single-threaded: TSF drives an apartment-threaded (STA) IME on one UI thread,
/// so shared-ownership + interior mutability without atomics/locks is correct.
pub struct ImeState {
    pub engine: Engine,
    /// TSF thread manager — kept for Deactivate / UnadviseKeyEventSink.
    pub thread_mgr: Option<ITfThreadMgr>,
    /// TSF client ID assigned to us on Activate.
    pub client_id: u32,
}

impl ImeState {
    pub fn new() -> Self {
        ImeState {
            engine: Engine::new(MappingConfig::from_preset(Preset::Telex)),
            thread_mgr: None,
            client_id: 0,
        }
    }
}

// ── InputProcessor ──────────────────────────────────────────────────────────

#[implement(windows::Win32::UI::TextServices::ITfTextInputProcessor)]
pub struct InputProcessor {
    /// Shared with KeyEventSink.
    state: Rc<RefCell<ImeState>>,
    /// Keep the key-event sink COM object alive while the IME is active.
    /// The sink borrows a clone of `state`.
    key_sink: RefCell<Option<ITfKeyEventSink>>,
}

impl InputProcessor {
    pub fn new(state: Rc<RefCell<ImeState>>) -> Self {
        use std::sync::atomic::Ordering;
        OBJ_COUNT.fetch_add(1, Ordering::SeqCst);
        InputProcessor {
            state,
            key_sink: RefCell::new(None),
        }
    }
}

impl Drop for InputProcessor {
    fn drop(&mut self) {
        use std::sync::atomic::Ordering;
        OBJ_COUNT.fetch_sub(1, Ordering::SeqCst);
    }
}

impl ITfTextInputProcessor_Impl for InputProcessor_Impl {
    /// Called by TSF when the IME is selected for the first time in a thread.
    fn Activate(&self, ptim: Option<&ITfThreadMgr>, tid: u32) -> Result<()> {
        let thread_mgr = ptim.ok_or_else(|| windows::core::Error::from(E_INVALIDARG))?;

        // Register our key-event sink with the keystroke manager.
        let keystroke_mgr: ITfKeystrokeMgr = thread_mgr.cast()?;
        let sink_obj: ITfKeyEventSink =
            KeyEventSink::new(Rc::clone(&self.state)).into();

        unsafe {
            keystroke_mgr.AdviseKeyEventSink(tid, &sink_obj, BOOL::from(true))?;
        }

        // Persist the sink so it isn't dropped prematurely.
        *self.key_sink.borrow_mut() = Some(sink_obj);

        let mut s = self.state.borrow_mut();
        s.thread_mgr = Some(thread_mgr.clone());
        s.client_id = tid;

        Ok(())
    }

    /// Called by TSF when the IME is deselected or the thread exits.
    fn Deactivate(&self) -> Result<()> {
        let s = self.state.borrow();
        if let (Some(tm), cid) = (&s.thread_mgr, s.client_id) {
            unsafe {
                if let Ok(km) = tm.cast::<ITfKeystrokeMgr>() {
                    let _ = km.UnadviseKeyEventSink(cid);
                }
            }
        }
        drop(s);

        // Release the key-event sink COM object.
        *self.key_sink.borrow_mut() = None;

        // Reset engine state so next Activate starts fresh.
        self.state.borrow_mut().engine.reset_buffer();
        Ok(())
    }
}
