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

    /// How many characters the current candidate occupies on screen.
    pub fn candidate_len(&self) -> usize {
        self.buffer.candidate.chars().count()
    }

    /// Words in the dictionary whose diacritic-stripped form matches the current candidate.
    /// Returns empty vec if candidate already has Vietnamese diacritics (nothing to improve)
    /// or if no dictionary match exists.
    pub fn get_suggestions(&self) -> Vec<String> {
        let cand = &self.buffer.candidate;
        if cand.is_empty() {
            return vec![];
        }
        let phonetic = crate::phonetic::strip_viet(cand);
        // Skip if the candidate already IS a diacritic-containing Vietnamese word
        if phonetic != *cand {
            return vec![];
        }
        crate::dict::lookup(&phonetic)
            .iter()
            .copied()
            .map(String::from)
            .collect()
    }

    pub fn reset_buffer(&mut self) {
        self.buffer.clear();
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
        self.process_key_inner(key)
    }

    fn process_key_inner(&mut self, key: char) -> EngineOutput {
        if self.mode == InputMode::English {
            return EngineOutput::Passthrough;
        }

        // Delimiter: commit current syllable and pass key through
        if is_delimiter(key) {
            return self.commit_and_passthrough(key);
        }

        // Technical token: uppercase letter or ASCII digit bypasses all Vietnamese processing.
        // Once is_technical is set the whole word stays literal (no tone/char_sub/double_char).
        if self.buffer.is_technical {
            return self.append_literal(key);
        }
        if key.is_uppercase() || key.is_ascii_digit() {
            return self.start_technical_token(key);
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

        // ── Escaped key: pressed as literal after a double-press escape ───
        // e.g. t+e+s+s → "tes", then s again → "tess" (not "té").
        if self.buffer.escaped_key == Some(key) {
            self.buffer.escaped_key = None;
            return self.append_literal(key);
        }
        self.buffer.escaped_key = None; // any other key clears the escape marker

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
        // Validate when a tone or char_sub changed the candidate from what was typed.
        // Pure literal and double-char-only buffers skip validation (e.g. "đ" from "dd").
        if (self.buffer.had_char_sub || self.buffer.had_tone_applied)
            && !is_valid_syllable(candidate)
        {
            // Use commit_raw (not raw): escape trims raw for clean BS replay, but
            // commit_raw retains the tone key so "tesst" reverts to "test" not "tet".
            return self.buffer.commit_raw.clone();
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
        // Tone key with no buffer content → treat as a literal character.
        // This keeps "song", "fix", etc. intact: 's','f','r','x','j' at word start
        // go into the buffer and participate in validity checking at commit.
        if self.buffer.is_empty() {
            return self.append_literal(key);
        }
        let (_, base_candidate) = self.extract_current_tone();
        let with_tone = apply_tone(&base_candidate, tone)
            .unwrap_or_else(|| base_candidate.clone());

        if !is_valid_syllable(&with_tone) {
            // Clear buffer so screen and engine stay in sync (prevents "ttong" bug).
            self.buffer.clear();
            return EngineOutput::Passthrough;
        }

        let prev_len = self.buffer.candidate.chars().count();
        self.buffer.had_tone_applied = true;
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

    /// Called when an uppercase letter or digit is typed.
    /// Marks the buffer as technical so no further Vietnamese processing occurs.
    /// If a tone or char_sub had already been applied, reverts the candidate back to raw first.
    fn start_technical_token(&mut self, key: char) -> EngineOutput {
        self.buffer.is_technical = true;
        if !self.buffer.is_empty() && (self.buffer.had_tone_applied || self.buffer.had_char_sub) {
            // Revert: replace the converted candidate with raw + new key
            let prev_len = self.buffer.candidate.chars().count();
            let new_text = format!("{}{}", self.buffer.commit_raw, key);
            self.buffer.raw = new_text.clone();
            self.buffer.commit_raw = new_text.clone();
            self.buffer.candidate = new_text.clone();
            self.buffer.had_tone_applied = false;
            self.buffer.had_char_sub = false;
            self.buffer.last_key = Some(key);
            self.buffer.last_key_count = 1;
            EngineOutput::Replace { delete_back: prev_len, text: new_text }
        } else {
            self.append_literal(key)
        }
    }

    fn try_double_char_rule(&self, key_str: &str) -> Option<String> {
        let candidate = &self.buffer.candidate;

        // ── Primary: last char + key forms a double-char pair ─────────────────
        if let Some(last) = candidate.chars().last() {
            let two = format!("{}{}", last, key_str);
            if let Some(result) = self.config.double_char.get(&two) {
                // w-based transforms only fire when buffer starts with an initial consonant.
                // Prevents "aws"→"ắ" while "nắng" still works.
                if key_str == "w" && !candidate_starts_with_consonant(candidate) {
                    return None;
                }
                let prefix = &candidate[..candidate.len() - last.len_utf8()];
                // Compound diphthong: "uo"+'w' → "ươ"
                // "đuo"+'w' → ow fires on 'o', and 'u' immediately before also becomes 'ư'.
                // Enables typing "dduowcj" → "được".
                if key_str == "w" && last == 'o' && prefix.ends_with('u') {
                    let uw_before = &prefix[..prefix.len() - 1]; // 'u' is ASCII, 1 byte
                    return Some(format!("{}ư{}", uw_before, result));
                }
                return Some(format!("{}{}", prefix, result));
            }
        }

        // ── Look-back: 'w' typed after a final consonant ───────────────────────
        // Scans backward through the final consonant to find the last vowel that can
        // pair with 'w'. Enables "dduocwj" → "được" (c is the final, o is the target vowel).
        if key_str == "w" && candidate_starts_with_consonant(candidate) {
            let chars: Vec<(usize, char)> = candidate.char_indices().collect();
            for &(byte_pos, ch) in chars.iter().rev() {
                if is_vowel_char(ch) {
                    let two = format!("{}w", ch);
                    if let Some(result) = self.config.double_char.get(&two) {
                        let before = &candidate[..byte_pos];
                        let after  = &candidate[byte_pos + ch.len_utf8()..];
                        // Compound: u + ow (over final) → ươ
                        if ch == 'o' && before.ends_with('u') {
                            let uw_before = &before[..before.len() - 1];
                            return Some(format!("{}ư{}{}", uw_before, result, after));
                        }
                        return Some(format!("{}{}{}", before, result, after));
                    }
                    break; // found a vowel but no 'w' match — stop looking further
                }
                // non-vowel (final consonant) — keep scanning backwards
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

        let new_candidate = if self.buffer.had_tone_applied {
            // Tone escape: strip tone and keep key as visible literal.
            // "té"+'s' → "tes" (not "te"). raw is NOT trimmed so BS pops the literal 's'.
            let (base, _) = strip_tone(&self.buffer.candidate);
            format!("{}{}", base, key)
        } else {
            // Double-char/char-sub escape: revert to just the key, trim raw for clean BS.
            if let Some((last_byte, _)) = self.buffer.raw.char_indices().next_back() {
                self.buffer.raw.truncate(last_byte);
            }
            if let Some((last_byte, _)) = self.buffer.commit_raw.char_indices().next_back() {
                self.buffer.commit_raw.truncate(last_byte);
            }
            key.to_string()
        };

        self.buffer.candidate = new_candidate.clone();
        self.buffer.had_char_sub = false;
        // Next press of the same key is literal (commit_raw already has it from the tone role).
        self.buffer.escaped_key = Some(key);
        self.buffer.last_key = None;
        self.buffer.last_key_count = 0;
        Some(EngineOutput::Replace {
            delete_back: prev_len,
            text: new_candidate,
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

/// Returns true if the candidate's first character is a Vietnamese initial consonant
/// (not a vowel, not empty). Used by the w-based double-char guard to prevent
/// "aws"→"ắs" while still allowing "nắng" (n is a consonant).
fn candidate_starts_with_consonant(s: &str) -> bool {
    match s.chars().next() {
        None => false,
        Some(c) => !matches!(c,
            'a'|'ă'|'â'|'e'|'ê'|'i'|'o'|'ô'|'ơ'|'u'|'ư'|'y'
            // toned vowels — should not start a syllable that benefits from w-transform
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
        ),
    }
}

/// Returns true if `c` is a Vietnamese vowel (base or toned).
fn is_vowel_char(c: char) -> bool {
    matches!(c,
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
    )
}

/// Returns true if `s` contains at least one Vietnamese vowel (base or toned).
fn candidate_has_vowel(s: &str) -> bool {
    s.chars().any(is_vowel_char)
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
    fn w_is_literal() {
        // 'w' is no longer in char_sub — it should append literally
        let mut e = telex_engine();
        let out = e.process_key('w');
        assert_eq!(out, EngineOutput::Replace { delete_back: 0, text: "w".into() });
    }

    #[test]
    fn bracket_is_literal() {
        // '[' and ']' are no longer char_sub — type literally
        let mut e = telex_engine();
        let out = e.process_key('[');
        assert_eq!(out, EngineOutput::Replace { delete_back: 0, text: "[".into() });
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
    fn tone_on_empty_buffer_becomes_literal() {
        // Tone key on empty buffer → treated as literal, not Passthrough.
        // Fixes "song" → "ong" regression (s was passthrough, ong committed separately).
        let mut e = telex_engine();
        assert_eq!(e.process_key('s'), EngineOutput::Replace { delete_back: 0, text: "s".into() });
    }

    #[test]
    fn backspace_on_empty_is_passthrough() {
        let mut e = telex_engine();
        assert_eq!(e.process_backspace(), EngineOutput::Passthrough);
    }

    #[test]
    fn backspace_pops_single_char() {
        // "w" is literal; Backspace → delete it, nothing left
        let mut e = telex_engine();
        e.process_key('w');
        let out = e.process_backspace();
        assert_eq!(out, EngineOutput::Replace { delete_back: 1, text: "".into() });
    }

    #[test]
    fn fix_stays_fix() {
        // 'f' → literal "f"; 'i' → "fi"; 'x' (ngã) on "fi" → is_valid("fĩ") false
        // ('f' is not a Vietnamese initial consonant) → buffer clears, Passthrough
        let mut e = telex_engine();
        e.process_key('f');
        e.process_key('i');
        let out = e.process_key('x');
        assert_eq!(out, EngineOutput::Passthrough);
    }

    #[test]
    fn trong_tone_on_consonant_clears_buffer() {
        // 't' then 'r' (hỏi) fails — buffer must clear so 'o' starts with delete_back=0
        let mut e = telex_engine();
        e.process_key('t');
        let tone_out = e.process_key('r');
        assert_eq!(tone_out, EngineOutput::Passthrough);
        let o_out = e.process_key('o');
        match o_out {
            EngineOutput::Replace { delete_back, .. } => assert_eq!(delete_back, 0),
            other => panic!("expected Replace, got {:?}", other),
        }
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
