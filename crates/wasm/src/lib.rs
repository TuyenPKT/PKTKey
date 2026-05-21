use pktkey_core::{Engine, InputMode, MappingConfig, Preset};
use wasm_bindgen::prelude::*;

#[cfg(feature = "console_error_panic_hook")]
fn set_panic_hook() {
    console_error_panic_hook::set_once();
}

#[cfg(not(feature = "console_error_panic_hook"))]
fn set_panic_hook() {}

/// JS-facing wrapper around the Rust Engine
#[wasm_bindgen]
pub struct WasmEngine {
    inner: Engine,
}

#[wasm_bindgen]
impl WasmEngine {
    /// Create a new engine.
    /// `preset`: "telex" | "vni" | "custom"
    #[wasm_bindgen(constructor)]
    pub fn new(preset: &str) -> WasmEngine {
        set_panic_hook();
        let p = match preset.to_lowercase().as_str() {
            "vni"    => Preset::Vni,
            "custom" => Preset::Custom,
            _        => Preset::Telex,
        };
        WasmEngine { inner: Engine::new(MappingConfig::from_preset(p)) }
    }

    /// Process one key press. Returns a JS object:
    /// ```text
    /// { type: "Replace", deleteBefore: number, text: string }
    /// { type: "Passthrough" }
    /// { type: "Commit", text: string }
    /// ```
    #[wasm_bindgen(js_name = processKey)]
    pub fn process_key(&mut self, key: &str) -> JsValue {
        let ch = match key.chars().next() {
            Some(c) => c,
            None    => return JsValue::NULL,
        };
        let output = self.inner.process_key(ch);
        serde_wasm_bindgen::to_value(&output).unwrap_or(JsValue::NULL)
    }

    /// Handle Backspace. Returns the same object shape as processKey.
    #[wasm_bindgen(js_name = processBackspace)]
    pub fn process_backspace(&mut self) -> JsValue {
        let output = self.inner.process_backspace();
        serde_wasm_bindgen::to_value(&output).unwrap_or(JsValue::NULL)
    }

    /// Toggle between Vietnamese and English input mode.
    #[wasm_bindgen(js_name = toggleMode)]
    pub fn toggle_mode(&mut self) {
        self.inner.toggle_mode();
    }

    /// Returns "vi" or "en"
    #[wasm_bindgen(js_name = getMode)]
    pub fn get_mode(&self) -> String {
        match self.inner.mode {
            InputMode::Vietnamese => "vi".into(),
            InputMode::English    => "en".into(),
        }
    }
}
