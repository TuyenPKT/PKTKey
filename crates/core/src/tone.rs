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
    let chars: Vec<char> = s.chars().collect();

    // Step 1: modified vowels (ê, ô, ơ, ư, ă, â) always take the tone mark.
    // These are unambiguous nuclei — wherever they appear they own the diacritic.
    const MODIFIED: &[char] = &['ê', 'ô', 'ă', 'â', 'ơ', 'ư'];
    for &mv in MODIFIED {
        if let Some(idx) = chars.iter().position(|&c| c == mv) {
            return Some(idx);
        }
    }

    // Step 2: two-vowel cluster rules for plain vowels.
    //
    // Modern Vietnamese orthography:
    //   "gi" glide — 'i' after 'g' is a medial glide (part of the "gi" onset),
    //          not the syllable nucleus. Tone skips it and lands on the next vowel.
    //          "gia"→'a' (giả ✓), "giao"→'a' (giào ✓).
    //          Same for "qu" glide: 'u' after 'q' is a medial glide.
    //          "qua"→'a' (quả ✓), "quê"→'ê' (caught by Step 1).
    //   "?a" — 'a' as coda after another vowel → preceding vowel gets tone
    //          oa→o  (hòa ✓, not hoà), ia→i (bìa ✓), ua→u (múa ✓)
    //   "uo" — base form of the "ươ" compound nucleus → 'o' is the main element
    //          (đuoc+j → đuọc, then +w → được; 'u' is the medial glide here)
    //   default → leftmost plain vowel
    //          ao→a (chào ✓), ai→a, oi→o, ui→u
    const PLAIN: &[char] = &['a', 'e', 'i', 'o', 'u', 'y'];
    let vowels: Vec<(usize, char)> = chars
        .iter()
        .enumerate()
        .filter(|(_, c)| PLAIN.contains(c))
        .map(|(i, &c)| (i, c))
        .collect();

    if vowels.len() >= 2 {
        let (i1, v1) = vowels[0];
        let (i2, v2) = vowels[1];

        // Medial glide rule: 'i' after 'g' (gi-onset) or 'u' after 'q' (qu-onset)
        // is part of the initial consonant cluster, not a vowel nucleus.
        // Skip it — tone belongs to the following vowel.
        let prev = if i1 > 0 { Some(chars[i1 - 1]) } else { None };
        let is_medial_glide = (v1 == 'i' && prev == Some('g'))
                           || (v1 == 'u' && prev == Some('q'));
        if is_medial_glide {
            // Apply cluster rules to the vowels *after* the glide
            if vowels.len() >= 3 {
                let (_, ev1) = vowels[1];
                let (ei3, ev2) = vowels[2];
                if ev2 == 'a' && ev1 != 'a' { return Some(i2); }
                if ev1 == 'u' && ev2 == 'o'  { return Some(ei3); }
            }
            return Some(i2); // default: first non-glide vowel
        }

        // Rule 1: 'a' as trailing vowel (no coda) → preceding vowel wins.
        //         But when a final consonant follows 'a', 'a' IS the nucleus → tone on 'a'.
        //         "hoa"→o (hòa ✓)   "bia"→i (bìa ✓)   "mua"→u (múa ✓)
        //         "oan"→a (đoán ✓)  "ian"→a             "uan"→a (đoàn ✓)
        if v2 == 'a' && v1 != 'a' {
            let has_coda = chars.get(i2 + 1)
                .map_or(false, |&c| !PLAIN.contains(&c) && !MODIFIED.contains(&c));
            return Some(if has_coda { i2 } else { i1 });
        }
        // Rule 2: "uo" cluster — 'o' (second) is the nucleus of the "ươ" compound
        if v1 == 'u' && v2 == 'o' {
            return Some(i2);
        }
    }
    // Default: leftmost plain vowel
    vowels.first().map(|(i, _)| *i)
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

    // ── Tone placement: modern Vietnamese orthography ──────────────────────

    #[test]
    fn hoa_huyen_on_o() {
        // "hòa" — huyền trên 'o', không phải 'a' (chuẩn hiện đại)
        let result = apply_tone("hoa", Tone::Huyen).unwrap();
        assert_eq!(result, "hòa", "dấu huyền phải trên o, không phải a");
    }

    #[test]
    fn hoa_sac_on_o() {
        // "hóa" (chemistry) — sắc trên 'o'
        let result = apply_tone("hoa", Tone::Sac).unwrap();
        assert_eq!(result, "hóa");
    }

    #[test]
    fn bia_huyen_on_i() {
        // "bìa" (cover/page) — huyền trên 'i'
        let result = apply_tone("bia", Tone::Huyen).unwrap();
        assert_eq!(result, "bìa");
    }

    #[test]
    fn mua_sac_on_u() {
        // "múa" (dance) — sắc trên 'u'
        let result = apply_tone("mua", Tone::Sac).unwrap();
        assert_eq!(result, "múa");
    }

    #[test]
    fn chao_huyen_on_a() {
        // "chào" (greeting) — huyền trên 'a', 'a' vẫn đúng vì 'a' đứng trước 'o'
        let result = apply_tone("chao", Tone::Huyen).unwrap();
        assert_eq!(result, "chào");
    }

    #[test]
    fn roi_huyen_on_o() {
        // Plain "roi": 'o' is first plain vowel → huyền on 'o' → "ròi"
        // (The real word "rồi" uses "ô" from Telex "oo"→"ô"; that goes through Step 1.)
        let result = apply_tone("roi", Tone::Huyen).unwrap();
        assert_eq!(result, "ròi");
    }

    #[test]
    fn gia_hoi_on_a() {
        // "giả" — hỏi trên 'a', không phải 'i' (i là glide của "gi")
        let result = apply_tone("gia", Tone::Hoi).unwrap();
        assert_eq!(result, "giả", "'i' sau 'g' là medial glide, tone phải trên 'a'");
    }

    #[test]
    fn giao_huyen_on_a() {
        // "giào" — huyền trên 'a' (glide 'i' bị bỏ qua)
        let result = apply_tone("giao", Tone::Huyen).unwrap();
        assert_eq!(result, "giào");
    }

    #[test]
    fn qua_hoi_on_a() {
        // "quả" — hỏi trên 'a', không phải 'u' (u là glide của "qu")
        let result = apply_tone("qua", Tone::Hoi).unwrap();
        assert_eq!(result, "quả", "'u' sau 'q' là medial glide, tone phải trên 'a'");
    }

    // ── "?a" + final consonant: tone phải trên 'a' ──────────────────────────

    #[test]
    fn oan_sac_on_a() {
        // "oán" — 'a' là nucleus vì có coda 'n' sau nó
        let result = apply_tone("oan", Tone::Sac).unwrap();
        assert_eq!(result, "oán", "coda 'n' → tone phải trên 'a', không phải 'o'");
    }

    #[test]
    fn doan_huyen_on_a() {
        // "đoàn" — 'a' là nucleus, 'n' là coda
        let result = apply_tone("đoan", Tone::Huyen).unwrap();
        assert_eq!(result, "đoàn");
    }

    #[test]
    fn hoa_no_coda_tone_on_o() {
        // "hóa" — không có coda → 'a' là trailing → tone trên 'o' (giữ nguyên behavior)
        let result = apply_tone("hoa", Tone::Sac).unwrap();
        assert_eq!(result, "hóa", "không có coda → tone trên 'o'");
    }

    #[test]
    fn roi_via_modified_o() {
        // "rồi" is typed as "rooif": "oo"→"ô" first, then f=huyền on "rôi"
        // Step 1 catches 'ô' → "rồi" (ồ = ô+huyền)
        let result = apply_tone("rôi", Tone::Huyen).unwrap();
        assert_eq!(result, "rồi");
    }
}
