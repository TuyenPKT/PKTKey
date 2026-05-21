//! ITfEditSession — performs atomic text edits inside a TSF document.
//!
//! TSF requires all text modifications to happen inside an edit session callback
//! (DoEditSession). We request a synchronous, read-write session in OnKeyDown,
//! then perform the delete+insert here.

use windows::{
    core::{implement, Result},
    Win32::{
        Foundation::{BOOL, E_FAIL},
        UI::TextServices::{
            ITfContext, ITfEditSession_Impl, TF_AE_NONE, TF_ANCHOR_END, TF_ANCHOR_START,
            TF_DEFAULT_SELECTION, TF_SELECTION, TF_SELECTIONSTYLE,
        },
    },
};

/// Edit session payload: delete `delete_back` chars before the cursor,
/// then insert `text` at the (now-empty) cursor position.
#[implement(windows::Win32::UI::TextServices::ITfEditSession)]
pub struct EditSession {
    context: ITfContext,
    delete_back: usize,
    text: String,
}

impl EditSession {
    pub fn new(context: ITfContext, delete_back: usize, text: String) -> Self {
        EditSession { context, delete_back, text }
    }
}

impl ITfEditSession_Impl for EditSession {
    fn DoEditSession(&self, ec: u32) -> Result<()> {
        unsafe { self.do_edit(ec) }
    }
}

impl EditSession {
    unsafe fn do_edit(&self, ec: u32) -> Result<()> {
        let ctx = &self.context;

        // ── 1. Get the current selection (cursor position). ───────────────────
        let mut sel = TF_SELECTION::default();
        let mut fetched: u32 = 0;
        ctx.GetSelection(ec, TF_DEFAULT_SELECTION, 1, &mut sel, &mut fetched)?;
        if fetched == 0 {
            return Err(E_FAIL.into());
        }
        let range = sel.range.as_ref().ok_or(E_FAIL)?;

        // ── 2. Collapse to the cursor (start == end == caret). ────────────────
        range.Collapse(ec, TF_ANCHOR_END)?;

        // ── 3. Extend the range backwards to cover delete_back characters. ────
        if self.delete_back > 0 {
            let mut actual_shift: i32 = 0;
            range.ShiftStart(
                ec,
                -(self.delete_back as i32),
                &mut actual_shift,
                std::ptr::null(), // no halt condition
            )?;
        }

        // ── 4. Replace range content with the new text (delete + insert). ─────
        //   SetText with an empty string deletes; with text it replaces.
        //   encode_utf16 is correct here — TSF uses UTF-16.
        let wide: Vec<u16> = self.text.encode_utf16().collect();
        range.SetText(ec, 0, windows::core::PCWSTR(wide.as_ptr()), wide.len() as i32)?;

        // ── 5. Move cursor to the end of the inserted text. ───────────────────
        range.Collapse(ec, TF_ANCHOR_END)?;

        let new_sel = TF_SELECTION {
            range: Some(range.clone()),
            style: TF_SELECTIONSTYLE {
                ase: TF_AE_NONE,
                fInterimChar: BOOL(0),
            },
        };
        ctx.SetSelection(ec, 1, &new_sel)?;

        Ok(())
    }
}
