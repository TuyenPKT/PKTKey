use crate::tone::strip_tone;

/// Initial consonants (phụ âm đầu) — longest match first
static INITIALS: &[&str] = &[
    // Trigraph
    "ngh",
    // Digraphs
    "ch", "gh", "gi", "kh", "ng", "nh", "ph", "qu", "th", "tr",
    // Singles
    "b", "c", "d", "đ", "g", "h", "k", "l", "m", "n", "p", "r", "s", "t", "v", "x",
];

/// Final consonants (phụ âm cuối) — longest match first
static FINALS: &[&str] = &["ch", "ng", "nh", "c", "m", "n", "p", "t"];

/// Valid vowel nuclei (nguyên âm / vần) — longest match first
static NUCLEI: &[&str] = &[
    // Triphthongs
    "iêu", "oai", "oao", "oay", "oeo", "uai", "uay", "uây", "ươi", "ươu", "yêu",
    // Diphthongs
    "ai", "ao", "au", "ay", "âu", "ây",
    "ia", "iê", "iu",
    "oa", "oă", "oe", "oi",
    "ua", "uâ", "uê", "ui", "uo", "uô", "uy",
    "ưa", "ươ", "ưi", "ưu",
    "ya", "yê",
    // Single vowels
    "a", "ă", "â", "e", "ê", "i", "o", "ô", "ơ", "u", "ư", "y",
];

/// Returns true if `s` is a structurally valid Vietnamese syllable.
/// Works on both raw (ã) and partially composed (a + ngã mark) forms.
pub fn is_valid_syllable(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    // Normalize: lowercase + strip tone (keep vowel shape modifiers)
    let (base, _) = strip_tone(&s.to_lowercase());

    parse_syllable(&base)
}

fn parse_syllable(base: &str) -> bool {
    // Try with each initial (or no initial)
    for &init in INITIALS.iter().chain(std::iter::once(&"")) {
        if !base.starts_with(init) {
            continue;
        }
        let after_init = &base[init.len()..];
        if try_nucleus_final(after_init) {
            return true;
        }
    }
    false
}

fn try_nucleus_final(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    // Try with each final (or no final)
    for &fin in FINALS.iter().chain(std::iter::once(&"")) {
        if !s.ends_with(fin) {
            continue;
        }
        let nucleus = &s[..s.len() - fin.len()];
        if NUCLEI.contains(&nucleus) {
            return true;
        }
    }
    false
}

/// Returns true if the string contains characters outside the Vietnamese alphabet.
/// Used to fast-reject clearly non-Vietnamese strings.
pub fn has_non_vietnamese_chars(s: &str) -> bool {
    s.chars().any(|c| !is_vietnamese_char(c))
}

fn is_vietnamese_char(c: char) -> bool {
    matches!(c,
        'a'..='z' | 'A'..='Z'
        | 'à'|'á'|'ả'|'ã'|'ạ'
        | 'ă'|'ắ'|'ằ'|'ẳ'|'ẵ'|'ặ'
        | 'â'|'ấ'|'ầ'|'ẩ'|'ẫ'|'ậ'
        | 'è'|'é'|'ẻ'|'ẽ'|'ẹ'
        | 'ê'|'ế'|'ề'|'ể'|'ễ'|'ệ'
        | 'ì'|'í'|'ỉ'|'ĩ'|'ị'
        | 'ò'|'ó'|'ỏ'|'õ'|'ọ'
        | 'ô'|'ố'|'ồ'|'ổ'|'ỗ'|'ộ'
        | 'ơ'|'ớ'|'ờ'|'ở'|'ỡ'|'ợ'
        | 'ù'|'ú'|'ủ'|'ũ'|'ụ'
        | 'ư'|'ứ'|'ừ'|'ử'|'ữ'|'ự'
        | 'ỳ'|'ý'|'ỷ'|'ỹ'|'ỵ'
        | 'đ' | 'Đ'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Valid Vietnamese syllables ──────────────────────────────────────────
    #[test]
    fn valid_single_vowel() {
        assert!(is_valid_syllable("a"));
        assert!(is_valid_syllable("ơ"));
        assert!(is_valid_syllable("ư"));
    }

    #[test]
    fn valid_with_tone() {
        assert!(is_valid_syllable("ã"));   // ngã tone on 'a'
        assert!(is_valid_syllable("ắ"));   // sắc on ă
        assert!(is_valid_syllable("được")); // 'đ' + 'ươ' + 'c'
    }

    #[test]
    fn valid_common_words() {
        assert!(is_valid_syllable("xin"));
        assert!(is_valid_syllable("chào"));
        assert!(is_valid_syllable("việt"));
        assert!(is_valid_syllable("nam"));
        assert!(is_valid_syllable("học"));
        assert!(is_valid_syllable("sinh"));
    }

    // ── Invalid (English words that would be mis-converted in Telex) ────────
    #[test]
    fn invalid_watch() {
        // Telex: w→ư → "ưatch" — must be rejected
        assert!(!is_valid_syllable("ưatch"));
    }

    #[test]
    fn invalid_show() {
        // Telex: s→sắc on nothing → no valid parse
        assert!(!is_valid_syllable("show"));
    }

    #[test]
    fn invalid_random() {
        assert!(!is_valid_syllable("xyz"));
        assert!(!is_valid_syllable("strength"));
        assert!(!is_valid_syllable("ưwx"));
    }

    // ── Edge cases ───────────────────────────────────────────────────────────
    #[test]
    fn empty_is_invalid() {
        assert!(!is_valid_syllable(""));
    }

    #[test]
    fn valid_gi_initial() {
        // "gi" initial + "a" nucleus → "gia"
        assert!(is_valid_syllable("gia"));
        assert!(is_valid_syllable("giá"));
    }

    #[test]
    fn valid_ngh_initial() {
        assert!(is_valid_syllable("nghề"));
        assert!(is_valid_syllable("nghĩ"));
    }
}
