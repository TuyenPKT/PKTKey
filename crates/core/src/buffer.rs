/// Tracks the syllable currently being composed.
/// Stores both the raw keystrokes and the converted candidate.
#[derive(Debug, Default, Clone)]
pub struct SyllableBuffer {
    /// Keys typed by the user for backspace replay.
    /// Escape trims the escaped tone key so BS replays cleanly.
    pub raw: String,
    /// Keys typed by the user for commit-time revert.
    /// Never trimmed on escape, so revert after "tesst" gives "test" not "tet".
    pub commit_raw: String,
    /// Current converted candidate (may differ from raw)
    pub candidate: String,
    /// Set when char_sub fired (e.g. w→ư).
    pub had_char_sub: bool,
    /// Set when a tone key successfully converted the candidate (e.g. "te"→"té").
    /// Enables validity check at commit to catch false positives like "test"→"tét".
    pub had_tone_applied: bool,
    /// Number of times the last conversion key was pressed consecutively
    /// (used for double-press escape detection)
    pub last_key_count: u32,
    pub last_key: Option<char>,
    /// Set when an uppercase letter or digit entered the buffer.
    /// In technical mode all subsequent keys are appended literally — no tone/double_char/char_sub.
    pub is_technical: bool,
    /// Set by double-press escape. The next press of this key is treated as a literal character
    /// rather than re-applying the tone. Clears after use or on buffer clear.
    pub escaped_key: Option<char>,
}

impl SyllableBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }

    /// Push a raw key and update the candidate.
    /// Returns the previous candidate so the caller can diff.
    pub fn push_raw(&mut self, key: char, candidate: String) -> String {
        let prev = self.candidate.clone();
        self.raw.push(key);
        self.commit_raw.push(key);
        if Some(key) == self.last_key {
            self.last_key_count += 1;
        } else {
            self.last_key = Some(key);
            self.last_key_count = 1;
        }
        self.candidate = candidate;
        prev
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Like push_raw but does NOT update commit_raw.
    /// Used when the escaped tone key is pressed as literal: commit_raw already
    /// accounts for that key from its original tone-key role.
    pub fn push_raw_keep_commit(&mut self, key: char, candidate: String) {
        self.raw.push(key);
        if Some(key) == self.last_key {
            self.last_key_count += 1;
        } else {
            self.last_key = Some(key);
            self.last_key_count = 1;
        }
        self.candidate = candidate;
    }

    /// True if the given key has been pressed consecutively ≥ n times
    pub fn last_key_repeated(&self, key: char, n: u32) -> bool {
        self.last_key == Some(key) && self.last_key_count >= n
    }
}
