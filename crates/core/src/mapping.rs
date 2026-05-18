use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Built-in input method presets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    Telex,
    Vni,
    Viqr,
    Custom,
}

/// A mapping rule result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleAction {
    /// Replace the current buffer content entirely
    ReplaceBuffer(String),
    /// Append a vowel shape modifier (ă, â, ơ, ư, đ, ê, ô)
    ApplyVowelMod(VowelMod),
    /// Apply a tone mark to the current buffer's main vowel
    ApplyTone(crate::tone::Tone),
    /// Append the character literally (no conversion)
    Literal(char),
    /// Double-press: revert last conversion and output raw char
    Escape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VowelMod {
    Breve,      // ă  (Telex: aw)
    Circumflex, // â  (Telex: aa)
    Horn,       // ơ/ư (Telex: ow/uw)
    Stroke,     // đ  (Telex: dd)
}

/// The full mapping configuration for an input session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingConfig {
    pub name: String,
    /// Single-char substitutions applied at word start or standalone (e.g. w→ư)
    pub char_sub: HashMap<String, String>,
    /// Double-char shortcuts (e.g. dd→đ, aa→â)
    pub double_char: HashMap<String, String>,
    /// Tone key mappings (key → tone name)
    pub tone: HashMap<String, String>,
    /// Vowel modifier key mappings (e.g. ow→ơ, uw→ư)
    pub vowel_mod: HashMap<String, String>,
    /// Double-press a key to escape conversion (output raw char)
    #[serde(default = "default_true")]
    pub double_press_escape: bool,
    /// Words to never convert (English whitelist)
    #[serde(default)]
    pub protected_words: Vec<String>,
}

fn default_true() -> bool {
    true
}

impl MappingConfig {
    pub fn telex() -> Self {
        let mut char_sub = HashMap::new();
        char_sub.insert("w".into(), "ư".into());
        char_sub.insert("[".into(), "ơ".into());
        char_sub.insert("]".into(), "ư".into());

        let mut double_char = HashMap::new();
        double_char.insert("dd".into(), "đ".into());
        double_char.insert("aa".into(), "â".into());
        double_char.insert("ee".into(), "ê".into());
        double_char.insert("oo".into(), "ô".into());
        double_char.insert("aw".into(), "ă".into());
        double_char.insert("ow".into(), "ơ".into());
        double_char.insert("uw".into(), "ư".into());

        let mut tone = HashMap::new();
        tone.insert("s".into(), "sac".into());
        tone.insert("f".into(), "huyen".into());
        tone.insert("r".into(), "hoi".into());
        tone.insert("x".into(), "nga".into());
        tone.insert("j".into(), "nang".into());

        MappingConfig {
            name: "Telex".into(),
            char_sub,
            double_char,
            tone,
            vowel_mod: HashMap::new(),
            double_press_escape: true,
            protected_words: vec![],
        }
    }

    pub fn vni() -> Self {
        let mut double_char = HashMap::new();
        double_char.insert("a6".into(), "â".into());
        double_char.insert("a8".into(), "ă".into());
        double_char.insert("e6".into(), "ê".into());
        double_char.insert("o6".into(), "ô".into());
        double_char.insert("o7".into(), "ơ".into());
        double_char.insert("u7".into(), "ư".into());
        double_char.insert("d9".into(), "đ".into());

        let mut tone = HashMap::new();
        tone.insert("1".into(), "sac".into());
        tone.insert("2".into(), "huyen".into());
        tone.insert("3".into(), "hoi".into());
        tone.insert("4".into(), "nga".into());
        tone.insert("5".into(), "nang".into());
        tone.insert("0".into(), "flat".into());

        MappingConfig {
            name: "VNI".into(),
            char_sub: HashMap::new(),
            double_char,
            tone,
            vowel_mod: HashMap::new(),
            double_press_escape: true,
            protected_words: vec![],
        }
    }

    pub fn from_preset(preset: Preset) -> Self {
        match preset {
            Preset::Telex  => Self::telex(),
            Preset::Vni    => Self::vni(),
            Preset::Viqr   => Self::vni(), // TODO: VIQR preset
            Preset::Custom => MappingConfig {
                name: "Custom".into(),
                char_sub: HashMap::new(),
                double_char: HashMap::new(),
                tone: HashMap::new(),
                vowel_mod: HashMap::new(),
                double_press_escape: true,
                protected_words: vec![],
            },
        }
    }

    /// Merge another config on top of this one (for "base + override" pattern)
    pub fn merge(&mut self, override_cfg: &MappingConfig) {
        for (k, v) in &override_cfg.char_sub {
            self.char_sub.insert(k.clone(), v.clone());
        }
        for (k, v) in &override_cfg.double_char {
            self.double_char.insert(k.clone(), v.clone());
        }
        for (k, v) in &override_cfg.tone {
            self.tone.insert(k.clone(), v.clone());
        }
        self.protected_words.extend(override_cfg.protected_words.iter().cloned());
    }

    pub fn is_protected(&self, word: &str) -> bool {
        let lower = word.to_lowercase();
        self.protected_words.iter().any(|w| w.to_lowercase() == lower)
    }

    pub fn tone_for_key(&self, key: &str) -> Option<crate::tone::Tone> {
        self.tone.get(key).and_then(|name| parse_tone_name(name))
    }
}

fn parse_tone_name(name: &str) -> Option<crate::tone::Tone> {
    use crate::tone::Tone;
    match name {
        "sac"   | "acute"  => Some(Tone::Sac),
        "huyen" | "grave"  => Some(Tone::Huyen),
        "hoi"   | "hook"   => Some(Tone::Hoi),
        "nga"   | "tilde"  => Some(Tone::Nga),
        "nang"  | "dot"    => Some(Tone::Nang),
        "flat"  | "none"   => Some(Tone::Flat),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telex_has_w_sub() {
        let cfg = MappingConfig::telex();
        assert_eq!(cfg.char_sub.get("w").map(String::as_str), Some("ư"));
    }

    #[test]
    fn telex_tone_s_is_sac() {
        let cfg = MappingConfig::telex();
        assert_eq!(cfg.tone_for_key("s"), Some(crate::tone::Tone::Sac));
    }

    #[test]
    fn vni_6_gives_circumflex() {
        let cfg = MappingConfig::vni();
        assert_eq!(cfg.double_char.get("a6").map(String::as_str), Some("â"));
    }
}
