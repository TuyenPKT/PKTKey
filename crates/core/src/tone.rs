/// Unicode combining tone marks used in Vietnamese (NFD form)
const TONE_MARKS: &[char] = &[
    '\u{0300}', // grave    → huyền (à)
    '\u{0301}', // acute    → sắc   (á)
    '\u{0303}', // tilde    → ngã   (ã)
    '\u{0309}', // hook     → hỏi   (ả)
    '\u{0323}', // dot below→ nặng  (ạ)
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Flat,   // bằng (không dấu)
    Huyen,  // huyền à
    Sac,    // sắc  á
    Nga,    // ngã  ã
    Hoi,    // hỏi  ả
    Nang,   // nặng ạ
}

impl Tone {
    pub fn combining_char(self) -> Option<char> {
        match self {
            Tone::Flat  => None,
            Tone::Huyen => Some('\u{0300}'),
            Tone::Sac   => Some('\u{0301}'),
            Tone::Nga   => Some('\u{0303}'),
            Tone::Hoi   => Some('\u{0309}'),
            Tone::Nang  => Some('\u{0323}'),
        }
    }
}

/// Strip tone marks from a string, returning (base_string, detected_tone).
/// Vowel shape modifiers (circumflex, breve, horn) are kept.
pub fn strip_tone(s: &str) -> (String, Tone) {
    use unicode_normalization::UnicodeNormalization;

    let nfd: String = s.nfd().collect();
    let mut tone = Tone::Flat;
    let base: String = nfd
        .chars()
        .filter(|&c| {
            if TONE_MARKS.contains(&c) {
                tone = char_to_tone(c);
                false
            } else {
                true
            }
        })
        .collect();

    let nfc: String = base.nfc().collect();
    (nfc, tone)
}

fn char_to_tone(c: char) -> Tone {
    match c {
        '\u{0300}' => Tone::Huyen,
        '\u{0301}' => Tone::Sac,
        '\u{0303}' => Tone::Nga,
        '\u{0309}' => Tone::Hoi,
        '\u{0323}' => Tone::Nang,
        _          => Tone::Flat,
    }
}

/// Apply a tone mark to the main vowel in a string.
/// Returns None if no suitable vowel is found.
pub fn apply_tone(s: &str, tone: Tone) -> Option<String> {
    use unicode_normalization::UnicodeNormalization;

    // Find the position of the main vowel to place the tone mark.
    // Rule: prefer ê/ô over other vowels; otherwise use the last vowel
    // before a final consonant.
    let vowel_idx = find_main_vowel_index(s)?;

    let chars: Vec<char> = s.chars().collect();
    let mut result = String::new();

    for (i, &c) in chars.iter().enumerate() {
        result.push(c);
        if i == vowel_idx {
            if let Some(mark) = tone.combining_char() {
                result.push(mark);
            }
        }
    }

    // NFC normalize so each character is a single code point
    Some(result.nfc().collect())
}

fn find_main_vowel_index(s: &str) -> Option<usize> {
    const VOWEL_PRIORITY: &[char] = &['ê', 'ô', 'ă', 'â', 'ơ', 'ư', 'a', 'e', 'o', 'u', 'i', 'y'];
    let chars: Vec<char> = s.chars().collect();

    // Priority vowel wins
    for &priority in VOWEL_PRIORITY {
        if let Some(idx) = chars.iter().position(|&c| c == priority) {
            return Some(idx);
        }
    }
    // Fallback: last vowel
    const PLAIN_VOWELS: &[char] = &['a', 'e', 'i', 'o', 'u', 'y'];
    chars.iter().rposition(|c| PLAIN_VOWELS.contains(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_tone_sac() {
        let (base, tone) = strip_tone("á");
        assert_eq!(base, "a");
        assert_eq!(tone, Tone::Sac);
    }

    #[test]
    fn strip_tone_complex() {
        let (base, tone) = strip_tone("ắ");
        assert_eq!(base, "ă");
        assert_eq!(tone, Tone::Sac);
    }

    #[test]
    fn strip_tone_no_tone() {
        let (base, tone) = strip_tone("watch");
        assert_eq!(base, "watch");
        assert_eq!(tone, Tone::Flat);
    }
}
