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

/// Remembers the last committed syllable so the user can backspace past the
/// delimiter (space) and continue editing without retyping from scratch.
struct LastCommit {
    /// Raw keystrokes to replay (e.g. "viet" for "việt").
    raw: String,
    /// Number of Unicode chars that went to screen: committed text + delimiter.
    /// Used as delete_back when re-entering edit mode.
    display_len: usize,
}

pub struct Engine {
    pub config: MappingConfig,
    pub mode: InputMode,
    buffer: SyllableBuffer,
    /// One-shot re-edit slot: set on every space-commit, consumed by the next
    /// Backspace-on-empty-buffer, cleared by any other key or reset.
    last_commit: Option<LastCommit>,
}

impl Engine {
    pub fn new(config: MappingConfig) -> Self {
        Self {
            config,
            mode: InputMode::Vietnamese,
            buffer: SyllableBuffer::new(),
            last_commit: None,
        }
    }

    /// How many characters the current candidate occupies on screen.
    pub fn candidate_len(&self) -> usize {
        self.buffer.candidate.chars().count()
    }

    /// Words in the dictionary whose diacritic-stripped form matches the current candidate.
    /// Returns empty vec if candidate already has Vietnamese diacritics (nothing to improve)
    /// or if no dictionary match exists.
    /// Results are sorted by Vietnamese word frequency — most common first.
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
        let mut suggestions: Vec<String> = crate::dict::lookup(&phonetic)
            .iter()
            .copied()
            .map(String::from)
            .collect();
        // Sort by frequency rank: most common word first
        suggestions.sort_by_key(|w| crate::freq::vi_rank(w));
        suggestions
    }

    pub fn reset_buffer(&mut self) {
        self.buffer.clear();
        self.last_commit = None;
    }

    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            InputMode::Vietnamese => InputMode::English,
            InputMode::English    => InputMode::Vietnamese,
        };
        self.buffer.clear();
        self.last_commit = None;
    }

    /// Process one key press. Returns the action the platform layer should take.
    pub fn process_key(&mut self, key: char) -> EngineOutput {
        self.process_key_inner(key)
    }

    fn process_key_inner(&mut self, key: char) -> EngineOutput {
        if self.mode == InputMode::English {
            return EngineOutput::Passthrough;
        }

        // Any keystroke (other than the BS path in process_backspace) voids the
        // re-edit slot — once you start a new syllable there's nothing to re-edit.
        // During internal replay, last_commit is already None (was .take()'d).
        self.last_commit = None;

        // Delimiter: commit current syllable and pass key through
        if is_delimiter(key) {
            return self.commit_and_passthrough(key);
        }

        // Technical token: uppercase letter or ASCII digit bypasses all Vietnamese processing.
        // Once is_technical is set the whole word stays literal (no tone/char_sub/double_char).
        if self.buffer.is_technical {
            return self.append_literal(key);
        }
        if key.is_ascii_digit() {
            return self.start_technical_token(key);
        }
        if key.is_uppercase() {
            // A leading uppercase on an empty buffer is a capitalised Vietnamese word
            // (Văn, Hà, Nguyễn…) — defer technical mode until the SECOND uppercase
            // letter arrives (which signals an acronym: API, HTTP, …).
            if self.buffer.is_empty() {
                return self.append_literal(key);
            }
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
    /// - Buffer empty + last_commit Some → re-edit: delete committed word+space,
    ///   replay raw keystrokes, return to composing state.
    /// - Buffer empty otherwise → Passthrough (platform deletes previous char)
    /// - Buffer non-empty → pop the last raw keystroke and recompute candidate
    pub fn process_backspace(&mut self) -> EngineOutput {
        if self.mode == InputMode::English {
            return EngineOutput::Passthrough;
        }

        if self.buffer.is_empty() {
            // Re-edit: restore last committed syllable into the buffer.
            if let Some(last) = self.last_commit.take() {
                let display_len = last.display_len;
                // Replay raw keystrokes to rebuild buffer state (candidate, tone flags, etc.)
                for ch in last.raw.chars() {
                    self.process_key(ch);
                }
                let text = self.buffer.candidate.clone();
                return EngineOutput::Replace { delete_back: display_len, text };
            }
            return EngineOutput::Passthrough;
        }

        let prev_candidate_len = self.buffer.candidate.chars().count();
        let raw_chars: Vec<char> = self.buffer.raw.chars().collect();

        // Pop raw keystrokes one at a time until the visual character count drops.
        // For single-visual-char buffers (á, ể, ư, â, đ…) this means we keep
        // popping until the candidate is empty — one Backspace deletes the whole
        // diacritic character.  For multi-char buffers we stop after one pop
        // (Unikey-style: removes the last modifier/tone key, allowing partial undo).
        let mut drop = 1usize;
        loop {
            let keep = raw_chars.len().saturating_sub(drop);
            let replay = &raw_chars[..keep];
            self.buffer.clear();
            let mut last_text = String::new();
            for &ch in replay {
                if let EngineOutput::Replace { text, .. } = self.process_key(ch) {
                    last_text = text;
                }
            }
            let new_len = self.buffer.candidate.chars().count();
            if new_len < prev_candidate_len || replay.is_empty() || prev_candidate_len > 1 {
                return EngineOutput::Replace { delete_back: prev_candidate_len, text: last_text };
            }
            drop += 1;
        }
    }

    // ── Internal helpers ──────────────────────────────────────────────────

    fn commit_and_passthrough(&mut self, delimiter: char) -> EngineOutput {
        let delete_back = self.buffer.candidate.chars().count();
        let committed = self.finalize_buffer();
        // Save re-edit slot: raw keystrokes + how many chars land on screen
        // (committed text + the delimiter itself, always 1 Unicode scalar).
        let raw = self.buffer.commit_raw.clone();
        self.last_commit = if raw.is_empty() {
            None
        } else {
            Some(LastCommit { raw, display_len: committed.chars().count() + 1 })
        };
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
        // EN_DEV safety net: if Telex conversion accidentally produced a known
        // English dev term, revert to raw. Covers edge cases where a vowel modifier
        // or tone fired and the result happens to be a dev word.
        if (self.buffer.had_char_sub || self.buffer.had_tone_applied)
            && crate::freq::is_en_dev(candidate)
        {
            return self.buffer.commit_raw.clone();
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

        // Mark as char_sub when the replacement modified a vowel (e.g. ow→ơ, aa→â).
        // This enables commit-time validity checking so invalid results like "ơl"
        // revert to the raw form "owl".
        // Consonant-only transforms like dd→đ must NOT set this flag: "đ" alone fails
        // the syllable validator but is a perfectly valid word-initial consonant.
        if candidate_has_vowel(&new_base) {
            self.buffer.had_char_sub = true;
        }

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
        // Buffer has consonants only — tone cannot apply without a vowel nucleus.
        // Treat as literal so "ssh" stays "ssh" instead of clearing the buffer.
        if !candidate_has_vowel(&self.buffer.candidate) {
            return self.append_literal(key);
        }
        let (_, base_candidate) = self.extract_current_tone();
        // In Vietnamese, "uo" in a closed syllable is always spelled "ươ".
        // Promote "uo"+coda → "ươ"+coda so that e.g. "duocj" → "dược" instead
        // of "duọc".  Only fires when candidate has plain ASCII "uo" before a
        // final consonant; already-promoted "ươ" passes through unchanged.
        let toned_candidate = promote_uo_closed(&base_candidate)
            .unwrap_or_else(|| base_candidate.clone());
        let with_tone = apply_tone(&toned_candidate, tone)
            .or_else(|| apply_tone(&base_candidate, tone))
            .unwrap_or_else(|| base_candidate.clone());

        if !is_valid_syllable(&with_tone) {
            // Toned result is not a valid Vietnamese syllable (e.g. foreign word
            // like "Windơ"+'s' → "Windớ").  Append the tone key as a literal so
            // the buffer stays intact and auto-revert at commit restores raw text.
            return self.append_literal(key);
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
        let prev_len = self.buffer.candidate.chars().count();

        // Re-seat the tone when a final consonant is appended after a tone was applied.
        // This fixes cases like "đóa"+'n' → "đoán" (tone moves from 'o' to 'a' because
        // the final consonant makes 'a' the nucleus, not the trailing vowel).
        let new_candidate = if self.buffer.had_tone_applied {
            let (base, tone) = strip_tone(&self.buffer.candidate);
            let new_base = format!("{}{}", base, key);
            apply_tone(&new_base, tone)
                .unwrap_or_else(|| format!("{}{}", self.buffer.candidate, key))
        } else {
            format!("{}{}", self.buffer.candidate, key)
        };

        self.buffer.push_raw(key, new_candidate.clone());
        EngineOutput::Replace { delete_back: prev_len, text: new_candidate }
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
                // 'aw'→'ă' only fires when there's an initial consonant in the buffer.
                // Prevents "aws"→"ắ" (English acronym conflict).
                // 'ow'→'ơ' and 'uw'→'ư' are allowed without initial consonant so that
                // standalone Vietnamese syllables like "ở" (owr), "ừ" (uwf) can be typed.
                let two_str = two.as_str();
                if key_str == "w"
                    && two_str == "aw"
                    && self.config.aw_requires_consonant
                    && !candidate_starts_with_consonant(candidate)
                {
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

        // ── Look-back: 'w' after a final consonant, or after a tone key ──────────
        // Scans backward through the final consonant to find the last vowel.
        // If the vowel is already toned (e.g. 'ọ' from a previous 'j'), strips its tone,
        // tries the base+w match, and returns the UNTONED result — apply_replacement
        // re-applies the current tone via extract_current_tone, placing it on the new vowel.
        //
        // Enables both orderings:
        //   "dduocwj" → "được"  (w before j — primary block handles this)
        //   "dduocjw" → "được"  (j before w — this look-back handles it)
        if key_str == "w" && candidate_starts_with_consonant(candidate) {
            let chars: Vec<(usize, char)> = candidate.char_indices().collect();
            for &(byte_pos, ch) in chars.iter().rev() {
                if is_vowel_char(ch) {
                    // Try direct match first (vowel has no tone yet).
                    let two_direct = format!("{}w", ch);
                    let (result, base_is_o) =
                        if let Some(r) = self.config.double_char.get(&two_direct).cloned() {
                            (Some(r), ch == 'o')
                        } else {
                            // Strip tone and retry: allows 'w' to fire after a tone key.
                            // Returns UNTONED transform so apply_replacement can re-tone
                            // consistently (tone moves from old vowel to new vowel).
                            let (base_str, _) = strip_tone(&ch.to_string());
                            let base_ch = base_str.chars().next().unwrap_or(ch);
                            // Also reverse shape modifiers: 'ơ' came from "ow", 'ư' from
                            // "uw", 'ă' from "aw" — use the ASCII base for the map lookup.
                            // This lets the look-back fire even when the vowel was already
                            // promoted (e.g. candidate "được" has 'ợ' → needs "ow" lookup).
                            let lookup_ch = match base_ch {
                                'ơ' => 'o', 'ư' => 'u', 'ă' => 'a', c => c,
                            };
                            let two_base = format!("{}w", lookup_ch);
                            (self.config.double_char.get(&two_base).cloned(), lookup_ch == 'o')
                        };

                    if let Some(result) = result {
                        let before = &candidate[..byte_pos];
                        let after  = &candidate[byte_pos + ch.len_utf8()..];
                        // Compound diphthong: u + ow → ươ
                        if base_is_o && before.ends_with('u') {
                            let uw_before = &before[..before.len() - 1];
                            return Some(format!("{}ư{}{}", uw_before, result, after));
                        }
                        return Some(format!("{}{}{}", before, result, after));
                    }
                    break; // found a vowel but no match — stop
                }
                // non-vowel (final consonant) — keep scanning backwards
            }
        }

        None
    }

    fn try_double_press_escape(&mut self, key: char) -> Option<EngineOutput> {
        // Only escape when there is an actual conversion to undo.
        // Without a prior tone or char-sub/double-char, repeated letters are plain
        // literals — "pp" in "apple", "ll" in "hello", etc. must NOT trigger escape.
        if !self.buffer.had_tone_applied && !self.buffer.had_char_sub {
            return None;
        }
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
            // Double-char/char-sub escape: undo the transform and restore raw keys.
            // Trim the triggering key from raw/commit_raw, then rebuild candidate as
            // (remaining commit_raw) + (escape key) so the prefix is preserved.
            //
            // Example: "gô" (from g+o+o) + escape 'o':
            //   commit_raw "goo" → truncated to "go"
            //   new_candidate = "go" + "o" = "goo"    ← prefix "g" kept ✓
            //
            // Without this, new_candidate was just "o" and the prefix "g" was lost,
            // causing "g+o+o+o+g+l+e" → "ogle" instead of "google".
            if let Some((last_byte, _)) = self.buffer.raw.char_indices().next_back() {
                self.buffer.raw.truncate(last_byte);
            }
            if let Some((last_byte, _)) = self.buffer.commit_raw.char_indices().next_back() {
                self.buffer.commit_raw.truncate(last_byte);
            }
            format!("{}{}", self.buffer.commit_raw, key)
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
    matches!(
        c,
        ' ' | '\n' | '\r' | '\t'
        | '.' | ',' | '!' | '?' | ';' | ':'
        | '/' | '-' | '(' | ')' | '[' | ']' | '{' | '}'
        | '"' | '\'' | '`'
        | '@' | '#' | '$' | '%' | '^' | '&' | '*'
        | '+' | '=' | '<' | '>' | '|' | '\\'
    )
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

/// In Vietnamese, "uo" in a CLOSED syllable (before a final consonant) is
/// always spelled "ươ" (both vowels carry the horn modifier).  When a tone
/// key fires on a candidate like "duoc", replace "uo" → "ươ" so that
/// "duocj" produces "dược" rather than "duọc".
///
/// Returns None when no "uo"+coda pattern is found (no change needed).
fn promote_uo_closed(candidate: &str) -> Option<String> {
    const CODA: &[&str] = &["ch", "ng", "nh", "c", "m", "n", "p", "t"];
    for fin in CODA {
        if candidate.ends_with(fin) {
            let stem = &candidate[..candidate.len() - fin.len()];
            if stem.ends_with("uo") {
                // 'u' and 'o' are single ASCII bytes — safe to slice by len
                let before = &stem[..stem.len() - 2];
                return Some(format!("{}ươ{}", before, fin));
            }
            break; // matched a coda but no "uo" stem → nothing to promote
        }
    }
    None
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
        // 'f'+'i'+'x': 'x' (ngã) on "fi" → "fĩ" invalid → append 'x' as literal → "fix"
        // The invalid tone key becomes a literal so auto-revert at commit handles it.
        let mut e = telex_engine();
        e.process_key('f');
        e.process_key('i');
        let out = e.process_key('x');
        assert_eq!(out, EngineOutput::Replace { delete_back: 2, text: "fix".to_string() });
    }

    #[test]
    fn tone_key_after_consonant_is_literal() {
        // 't' + 'r' (hỏi tone key) with no vowel yet → 'r' is treated as literal,
        // giving consonant cluster "tr". Then 'o' correctly gives "tro".
        let mut e = telex_engine();
        e.process_key('t');
        let tone_out = e.process_key('r');
        assert_eq!(tone_out, EngineOutput::Replace { delete_back: 1, text: "tr".to_string() });
        let o_out = e.process_key('o');
        match o_out {
            EngineOutput::Replace { delete_back, text } => {
                assert_eq!(delete_back, 2);
                assert_eq!(text, "tro");
            }
            other => panic!("expected Replace, got {:?}", other),
        }
    }

    #[test]
    fn ssh_stays_ssh() {
        // "ss" in Telex must NOT erase one 's' when followed by 'h'.
        // Previously, the second 's' (sắc tone) cleared the buffer → "ssh" became "sh".
        let mut e = telex_engine();
        e.process_key('s');
        let s2 = e.process_key('s');
        // second 's': buffer "s" has no vowel → literal → "ss"
        assert_eq!(s2, EngineOutput::Replace { delete_back: 1, text: "ss".to_string() });
        let h = e.process_key('h');
        assert_eq!(h, EngineOutput::Replace { delete_back: 2, text: "ssh".to_string() });
    }

    #[test]
    fn uo_closed_promotes_on_tone() {
        // "duoc" + 'j' (nặng): "uo" before coda 'c' → promote to "ươ" → "dược"
        let mut e = telex_engine();
        e.process_key('d');
        e.process_key('u');
        e.process_key('o');
        e.process_key('c');
        let out = e.process_key('j');
        match out {
            EngineOutput::Replace { text, .. } => assert_eq!(text, "dược"),
            other => panic!("expected Replace, got {:?}", other),
        }
    }

    #[test]
    fn backspace_deletes_single_diacritic() {
        // "aa" → "â" (1 visual char); one Backspace deletes the whole thing → ""
        let mut e = telex_engine();
        e.process_key('a');
        e.process_key('a'); // now "â"
        let out = e.process_backspace();
        assert_eq!(out, EngineOutput::Replace { delete_back: 1, text: "".into() });
    }
}
