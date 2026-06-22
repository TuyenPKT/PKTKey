//! IClassFactory — creates InputProcessor instances on demand.

use std::cell::RefCell;
use std::rc::Rc;

use windows::{
    core::{implement, IUnknown, Interface, Result},
    Win32::{
        Foundation::{CLASS_E_NOAGGREGATION, E_NOINTERFACE},
        System::Com::IClassFactory_Impl,
    },
};

use crate::{processor, LOCK_COUNT};

/// COM class factory for the PKTKey IME input processor.
#[implement(windows::Win32::System::Com::IClassFactory)]
pub struct ClassFactory;

impl ClassFactory {
    pub fn new() -> Self {
        ClassFactory
    }
}

impl IClassFactory_Impl for ClassFactory_Impl {
    fn CreateInstance(
        &self,
        punkouter: Option<&IUnknown>,
        riid: *const windows::core::GUID,
        ppvobject: *mut *mut core::ffi::c_void,
    ) -> Result<()> {
        // Aggregation is not supported.
        if punkouter.is_some() {
            return Err(CLASS_E_NOAGGREGATION.into());
        }
        if ppvobject.is_null() {
            return Err(E_NOINTERFACE.into());
        }

        // Build the input processor and return the requested interface.
        let state = Rc::new(RefCell::new(processor::ImeState::new()));
        let proc = processor::InputProcessor::new(state);
        let itf: windows::Win32::UI::TextServices::ITfTextInputProcessor = proc.into();

        // QueryInterface into the caller's requested riid.
        unsafe { itf.query(riid, ppvobject).ok() }
    }

    fn LockServer(&self, flock: windows::Win32::Foundation::BOOL) -> Result<()> {
        use std::sync::atomic::Ordering;
        if flock.as_bool() {
            LOCK_COUNT.fetch_add(1, Ordering::SeqCst);
        } else {
            LOCK_COUNT.fetch_sub(1, Ordering::SeqCst);
        }
        Ok(())
    }
}
