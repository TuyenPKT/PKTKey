use crate::{
    buffer::SyllableBuffer,
    mapping::MappingConfig,
    tone::{apply_tone, strip_tone, Tone},
    validator::is_valid_syllable,
};

/// What the engine tells the caller to do after processing a keystroke.
/// Serialized with `type` tag so JS can do `if (output.type === "Replace") ...`
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "type")]
pub enum EngineOutput {
    /// Replace the last `delete_back` characters with `text`.
    /// delete_back=0 means pure insertion.
    Replace { delete_back: usize, text: String },
    /// Pass the key through unchanged (engine is disabled or key is unhandled)
    Passthrough,
    /// Commit the current syllable (reserved for future platform use)
    Commit { text: String },
}

/// Whether Vietnamese input is active
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Vietnamese,
    English,
}

pub struct Engine {
    pub config: MappingConfig,
    pub mode: InputMode,
    buffer: SyllableBuffer,
}

impl Engine {
    pub fn new(config: MappingConfig) -> Self {
        Self {
            config,
            mode: InputMode::Vietnamese,
            buffer: SyllableBuffer::new(),
        }
    }

    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            InputMode::Vietnamese => InputMode::English,
            InputMode::English    => InputMode::Vietnamese,
        };
        self.buffer.clear();
    }

    /// Process one key press. Returns the action the platform layer should take.
    pub fn process_key(&mut self, key: char) -> EngineOutput {
        if self.mode == InputMode::English {
            return EngineOutput::Passthrough;
        }

        // Delimiter: commit current syllable and pass key through
        if is_delimiter(key) {
            return self.commit_and_passthrough(key);
        }

        let key_str = key.to_string();

        // ── Try double-char rule first (e.g. "aa"→â, "ow"→ơ) ───────────────
        // Must come before the escape check so "aa" → "â" not "a" (escape).
        if let Some(replacement) = self.try_double_char_rule(&key_str) {
            return self.apply_replacement(key, replacement);
        }

        // ── Double-press escape ────────────────────────────────────────────
        // Typing the same conversion key twice reverts the last char_sub.
        // e.g. 'w' → 'ư', then 'w' again → 'w' (no double-char rule matched)
        if self.config.double_press_escape {
            if self.buffer.last_key_repeated(key, 1) {
                if let Some(escaped) = self.try_double_press_escape(key) {
                    return escaped;
                }
            }
        }

        // ── Try tone key ──────────────────────────────────────────────────
        if let Some(tone) = self.config.clone().tone_for_key(&key_str) {
            return self.apply_tone_key(key, tone);
        }

        // ── Try char substitution (e.g. w→ư at buffer start) ─────────────
        if let Some(sub) = self.config.char_sub.get(&key_str).cloned() {
            return self.apply_char_sub(key, sub);
        }

        // ── Plain character ────────────────────────────────────────────────
        self.append_literal(key)
    }

    /// Handle Backspace.
    /// - Buffer empty → Passthrough (platform deletes previous committed char)
    /// - Buffer non-empty → pop the last raw keystroke and recompute candidate
    pub fn process_backspace(&mut self) -> EngineOutput {
        if self.mode == InputMode::English || self.buffer.is_empty() {
            return EngineOutput::Passthrough;
        }

        let prev_candidate_len = self.buffer.candidate.chars().count();

        // Collect all raw keys except the last one
        let raw_chars: Vec<char> = self.buffer.raw.chars().collect();
        let replay: Vec<char> = raw_chars[..raw_chars.len() - 1].to_vec();

        // Reset buffer and replay remaining keys to rebuild candidate
        self.buffer.clear();
        let mut last_text = String::new();
        for ch in replay {
            if let EngineOutput::Replace { text, .. } = self.process_key(ch) {
                last_text = text;
            }
        }

        EngineOutput::Replace {
            delete_back: prev_candidate_len,
            text: last_text,
        }
    }

    // ── Internal helpers ──────────────────────────────────────────────────

    fn commit_and_passthrough(&mut self, delimiter: char) -> EngineOutput {
        let delete_back = self.buffer.candidate.chars().count();
        let committed = self.finalize_buffer();
        self.buffer.clear();
        let text = format!("{}{}", committed, delimiter);
        EngineOutput::Replace { delete_back, text }
    }

    /// Return the best candidate for the current buffer, reverting to raw if invalid.
    fn finalize_buffer(&self) -> String {
        let candidate = &self.buffer.candidate;
        if candidate.is_empty() {
            return self.buffer.raw.clone();
        }
        if self.config.is_protected(candidate) {
            return self.buffer.raw.clone();
        }
        // Only run the syllable validator when char_sub was used (w→ư, [→ơ, etc.).
        // Double-char / tone-only conversions are always committed as-is because
        // they are explicit user intent (e.g. "dd"→"đ", "as"→"á").
        if self.buffer.had_char_sub && !is_valid_syllable(candidate) {
            return self.buffer.raw.clone();
        }
        candidate.clone()
    }

    fn apply_replacement(&mut self, key: char, new_base: String) -> EngineOutput {
        // The new_base replaces the whole candidate (e.g. "oa" → "oa" with â modifier)
        let prev_len = self.buffer.candidate.chars().count();
        let (tone, _) = self.extract_current_tone();
        let with_tone = if tone != Tone::Flat {
            apply_tone(&new_base, tone).unwrap_or(new_base.clone())
        } else {
            new_base.clone()
        };

        self.buffer.push_raw(key, with_tone.clone());
        EngineOutput::Replace {
            delete_back: prev_len,
            text: with_tone,
        }
    }

    fn apply_tone_key(&mut self, key: char, tone: Tone) -> EngineOutput {
        if self.buffer.is_empty() {
            // No vowel to attach tone to — output literal
            return EngineOutput::Passthrough;
        }
        let (_, base_candidate) = self.extract_current_tone();
        let with_tone = apply_tone(&base_candidate, tone)
            .unwrap_or_else(|| base_candidate.clone());

        if !is_valid_syllable(&with_tone) {
            // Tone would make syllable invalid — output literal
            return EngineOutput::Passthrough;
        }

        let prev_len = self.buffer.candidate.chars().count();
        self.buffer.push_raw(key, with_tone.clone());
        EngineOutput::Replace {
            delete_back: prev_len,
            text: with_tone,
        }
    }

    fn apply_char_sub(&mut self, key: char, sub: String) -> EngineOutput {
        // Apply char_sub when buffer is empty OR has only initial consonants (no vowel yet).
        // Example: 'n'+'w' → "nư" because 'n' is an initial and 'w' supplies the nucleus.
        if !self.buffer.is_empty() && candidate_has_vowel(&self.buffer.candidate) {
            return self.append_literal(key);
        }
        let prev_len = self.buffer.candidate.chars().count();
        // Preserve any initial consonants already in the candidate
        let new_candidate = format!("{}{}", self.buffer.candidate, sub);
        self.buffer.had_char_sub = true;
        self.buffer.push_raw(key, new_candidate.clone());
        EngineOutput::Replace { delete_back: prev_len, text: new_candidate }
    }

    fn append_literal(&mut self, key: char) -> EngineOutput {
        let new_candidate = format!("{}{}", self.buffer.candidate, key);
        let prev_len = self.buffer.candidate.chars().count();
        self.buffer.push_raw(key, new_candidate.clone());
        EngineOutput::Replace {
            delete_back: prev_len,
            text: new_candidate,
        }
    }

    fn try_double_char_rule(&self, key_str: &str) -> Option<String> {
        let candidate = &self.buffer.candidate;
        // Build the 2-char sequence: last char of candidate + new key
        if let Some(last) = candidate.chars().last() {
            let two = format!("{}{}", last, key_str);
            if let Some(result) = self.config.double_char.get(&two) {
                return Some(format!("{}{}", &candidate[..candidate.len() - last.len_utf8()], result));
            }
        }
        None
    }

    fn try_double_press_escape(&mut self, key: char) -> Option<EngineOutput> {
        let raw_last = self.buffer.raw.chars().last()?;
        if raw_last != key {
            return None;
        }
        let prev_len = self.buffer.candidate.chars().count();
        let escaped = key.to_string();
        self.buffer.push_raw(key, escaped.clone());
        // User explicitly escaped — finalize_buffer must NOT revert to raw.
        self.buffer.had_char_sub = false;
        Some(EngineOutput::Replace {
            delete_back: prev_len,
            text: escaped,
        })
    }

    fn extract_current_tone(&self) -> (Tone, String) {
        let (base, tone) = strip_tone(&self.buffer.candidate);
        (tone, base)
    }
}

fn is_delimiter(c: char) -> bool {
    matches!(c, ' ' | '\n' | '\r' | '\t' | '.' | ',' | '!' | '?' | ';' | ':')
}

/// Returns true if `s` contains at least one Vietnamese vowel (base or toned).
fn candidate_has_vowel(s: &str) -> bool {
    s.chars().any(|c| matches!(c,
        'a'|'ă'|'â'|'e'|'ê'|'i'|'o'|'ô'|'ơ'|'u'|'ư'|'y'
        |'á'|'à'|'ả'|'ã'|'ạ'
        |'ắ'|'ằ'|'ẳ'|'ẵ'|'ặ'
        |'ấ'|'ầ'|'ẩ'|'ẫ'|'ậ'
        |'é'|'è'|'ẻ'|'ẽ'|'ẹ'
        |'ế'|'ề'|'ể'|'ễ'|'ệ'
        |'í'|'ì'|'ỉ'|'ĩ'|'ị'
        |'ó'|'ò'|'ỏ'|'õ'|'ọ'
        |'ố'|'ồ'|'ổ'|'ỗ'|'ộ'
        |'ớ'|'ờ'|'ở'|'ỡ'|'ợ'
        |'ú'|'ù'|'ủ'|'ũ'|'ụ'
        |'ứ'|'ừ'|'ử'|'ữ'|'ự'
        |'ý'|'ỳ'|'ỷ'|'ỹ'|'ỵ'
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mapping::Preset;

    fn telex_engine() -> Engine {
        Engine::new(MappingConfig::from_preset(Preset::Telex))
    }

    #[test]
    fn english_mode_passthrough() {
        let mut e = telex_engine();
        e.mode = InputMode::English;
        assert_eq!(e.process_key('w'), EngineOutput::Passthrough);
    }

    #[test]
    fn char_sub_w_gives_ư() {
        let mut e = telex_engine();
        let out = e.process_key('w');
        assert_eq!(out, EngineOutput::Replace { delete_back: 0, text: "ư".into() });
    }

    #[test]
    fn double_char_aa_gives_â() {
        let mut e = telex_engine();
        e.process_key('a');
        let out = e.process_key('a');
        match out {
            EngineOutput::Replace { text, .. } => assert!(text.contains('â')),
            other => panic!("unexpected: {:?}", other),
        }
    }

    #[test]
    fn tone_s_applies_sac() {
        let mut e = telex_engine();
        e.process_key('a');
        let out = e.process_key('s');
        match out {
            EngineOutput::Replace { text, .. } => assert_eq!(text, "á"),
            other => panic!("unexpected: {:?}", other),
        }
    }

    #[test]
    fn tone_on_empty_buffer_passthrough() {
        let mut e = telex_engine();
        assert_eq!(e.process_key('s'), EngineOutput::Passthrough);
    }

    #[test]
    fn backspace_on_empty_is_passthrough() {
        let mut e = telex_engine();
        assert_eq!(e.process_backspace(), EngineOutput::Passthrough);
    }

    #[test]
    fn backspace_pops_single_char() {
        // "w" → "ư"; Backspace → delete "ư", nothing left
        let mut e = telex_engine();
        e.process_key('w');
        let out = e.process_backspace();
        assert_eq!(out, EngineOutput::Replace { delete_back: 1, text: "".into() });
    }

    #[test]
    fn backspace_pops_to_previous_candidate() {
        // "a" → "a"; "a" → "â"; Backspace → back to "a"
        let mut e = telex_engine();
        e.process_key('a');
        e.process_key('a'); // now "â"
        let out = e.process_backspace();
        match out {
            EngineOutput::Replace { text, .. } => assert_eq!(text, "a"),
            other => panic!("expected Replace, got {:?}", other),
        }
    }
}
