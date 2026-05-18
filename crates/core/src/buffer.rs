/// Tracks the syllable currently being composed.
/// Stores both the raw keystrokes and the converted candidate.
#[derive(Debug, Default, Clone)]
pub struct SyllableBuffer {
    /// Keys typed by the user, in order
    pub raw: String,
    /// Current converted candidate (may differ from raw)
    pub candidate: String,
    /// Set when char_sub fired (e.g. w→ư). Enables syllable-validity check at commit
    /// to catch false positives like "ưatch" from "watch".
    pub had_char_sub: bool,
    /// Number of times the last conversion key was pressed consecutively
    /// (used for double-press escape detection)
    pub last_key_count: u32,
    pub last_key: Option<char>,
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

    /// True if the given key has been pressed consecutively ≥ n times
    pub fn last_key_repeated(&self, key: char, n: u32) -> bool {
        self.last_key == Some(key) && self.last_key_count >= n
    }
}
