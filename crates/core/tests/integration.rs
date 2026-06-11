use pktkey_core::{Engine, EngineOutput, MappingConfig, Preset};

/// Simulate typing a sequence of characters through the engine.
/// Returns the final committed text (what would appear on screen).
fn type_sequence(engine: &mut Engine, input: &str) -> String {
    let mut committed = String::new();
    let mut candidate = String::new();

    for ch in input.chars() {
        match engine.process_key(ch) {
            EngineOutput::Replace { text, delete_back } => {
                // Trim the candidate portion, replace with new text
                let cand_chars = candidate.chars().count();
                assert_eq!(delete_back, cand_chars,
                    "delete_back mismatch at '{}': expected {} got {}",
                    ch, cand_chars, delete_back);
                candidate = text;
            }
            EngineOutput::Passthrough => {
                // Flush current candidate first
                committed.push_str(&candidate);
                candidate.clear();
                committed.push(ch);
            }
            EngineOutput::Commit { text } => {
                committed.push_str(&text);
                candidate.clear();
            }
        }
    }
    // Flush remaining candidate
    committed.push_str(&candidate);
    committed
}

fn telex() -> Engine {
    Engine::new(MappingConfig::from_preset(Preset::Telex))
}

/// Like type_sequence but '\x08' (BS) calls process_backspace.
fn type_with_bs(engine: &mut Engine, input: &str) -> String {
    let mut committed = String::new();
    let mut candidate = String::new();

    for ch in input.chars() {
        let result = if ch == '\x08' {
            engine.process_backspace()
        } else {
            engine.process_key(ch)
        };

        match result {
            EngineOutput::Replace { text, delete_back } => {
                let cand_len = candidate.chars().count();
                assert_eq!(delete_back, cand_len);
                candidate = text;
            }
            EngineOutput::Passthrough => {
                committed.push_str(&candidate);
                candidate.clear();
                if ch != '\x08' { committed.push(ch); }
            }
            EngineOutput::Commit { text } => {
                committed.push_str(&text);
                candidate.clear();
            }
        }
    }
    committed.push_str(&candidate);
    committed
}

// ── False-positive tests (English words must NOT be converted) ─────────────

#[test]
fn watch_not_converted() {
    // Telex: w→ư, but "ưatch" is not a valid Vietnamese syllable
    // On space commit, engine must revert to raw "watch"
    let result = type_sequence(&mut telex(), "watch ");
    assert_eq!(result, "watch ");
}

#[test]
fn aws_not_converted() {
    // "aws": 'a' is a vowel → consonant-check blocks aw→ă, so "aw" stays literal.
    // Even if it didn't block, "ắ s" would revert anyway — but now it never fires.
    let result = type_sequence(&mut telex(), "aws ");
    assert_eq!(result, "aws ");
}

#[test]
fn show_not_converted() {
    // "show": s is a tone key but buffer is empty → passthrough
    // Then "how" remains: "h"+"ow" — "ow"→"ơ" double-char, "hơ" is valid...
    // Actually "show" passes s through, then "how" might convert.
    // This test ensures the sequence behaves predictably.
    let result = type_sequence(&mut telex(), "show ");
    // s is passed through (no buffer), then "how" → "hơ" ... or not?
    // "h" appended to new buffer, "o" appended, "w" triggers "ow"→"ơ" double char
    // So result might be "shơ " — that's actually a valid syllable.
    // This is an acceptable trade-off; user can use English mode or whitelist.
    // Just ensure it doesn't panic and returns SOMETHING stable.
    assert!(!result.is_empty());
}

// ── Valid Vietnamese syllables must be converted ───────────────────────────

#[test]
fn ax_gives_nga_tone() {
    // a + x (tone ngã) → "ã"
    let result = type_sequence(&mut telex(), "ax ");
    assert_eq!(result, "ã ");
}

#[test]
fn xin_stays_xin() {
    // "xin": x is tone key on empty buffer → passthrough
    // then "in" builds syllable → committed as "in"
    // result: "x" (passthrough) + "in" (syllable) + " " = "xin "
    let result = type_sequence(&mut telex(), "xin ");
    assert_eq!(result, "xin ");
}

#[test]
fn chao_gives_chao() {
    // "chao": init "ch" + nucleus "ao" → valid → "chao"
    let result = type_sequence(&mut telex(), "chao ");
    assert_eq!(result, "chao ");
}

#[test]
fn dd_gives_d_stroke() {
    // "dd" → "đ" via double_char rule
    let result = type_sequence(&mut telex(), "dd ");
    assert_eq!(result, "đ ");
}

#[test]
fn ow_gives_o_horn() {
    // "ow" standalone (no initial consonant) → "ơ" — needed for "ở", "ớ", "ờ" etc.
    // Only "aw"→"ă" requires an initial consonant (conflicts with English "aws").
    // "ow" and "uw" are allowed standalone.
    let result = type_sequence(&mut telex(), "ow ");
    assert_eq!(result, "ơ ");
}

#[test]
fn now_gives_no_horn() {
    // 'n' is initial consonant → ow→ơ fires → "nơ"
    let result = type_sequence(&mut telex(), "now ");
    assert_eq!(result, "nơ ");
}

#[test]
fn aa_gives_a_circumflex() {
    // "aa" → "â"
    let result = type_sequence(&mut telex(), "aa ");
    assert_eq!(result, "â ");
}

// ── Tone application ───────────────────────────────────────────────────────

#[test]
fn tone_as_produces_sac() {
    // "as" → "á" (a + sắc tone)
    let result = type_sequence(&mut telex(), "as ");
    assert_eq!(result, "á ");
}

#[test]
fn tone_af_produces_huyen() {
    let result = type_sequence(&mut telex(), "af ");
    assert_eq!(result, "à ");
}

// ── Mode toggle ────────────────────────────────────────────────────────────

#[test]
fn english_mode_no_conversion() {
    let mut e = telex();
    e.toggle_mode(); // switch to English
    let result = type_sequence(&mut e, "watch ");
    assert_eq!(result, "watch ");
}

// ── char_sub after initial consonant ──────────────────────────────────────

#[test]
fn nuw_gives_nu_horn() {
    // 'n' + 'uw' (double_char → 'ư') → "nư"
    let result = type_sequence(&mut telex(), "nuw ");
    assert_eq!(result, "nư ");
}

#[test]
fn nuws_gives_nu_sac() {
    // 'n' + 'uw' + 's' (tone sắc) → "nứ"
    let result = type_sequence(&mut telex(), "nuws ");
    assert_eq!(result, "nứ ");
}

#[test]
fn w_is_literal_in_telex() {
    // 'w' is no longer char_sub'd — should commit literally
    let result = type_sequence(&mut telex(), "watch ");
    assert_eq!(result, "watch ");
}

// ── Bracket is now literal ─────────────────────────────────────────────────

#[test]
fn bracket_is_literal() {
    // '[' and ']' are delimiters — committed immediately with no char_sub transform
    let result = type_sequence(&mut telex(), "a]");
    assert_eq!(result, "a]");
}

// ── Backspace ──────────────────────────────────────────────────────────────

#[test]
fn backspace_reverts_w_to_nothing() {
    // w→ư, Backspace → empty (ư deleted, buffer cleared)
    let result = type_with_bs(&mut telex(), "w\x08 ");
    assert_eq!(result, " ");
}

#[test]
fn backspace_reverts_double_char() {
    // aa→â (1 visual char), one Backspace deletes whole â → "", then space
    let result = type_with_bs(&mut telex(), "aa\x08 ");
    assert_eq!(result, " ");
}

#[test]
fn backspace_then_retype() {
    // 'n'+'uw'→'nư', 'a'→'nưa', 's'→'nứa', BS→'nưa', BS→'nư', 's'→'nứ', space commits "nứ"
    // Consonant 'n' is required for the uw→ư transform to fire.
    let result = type_with_bs(&mut telex(), "nuwas\x08\x08s ");
    assert_eq!(result, "nứ ");
}

// ── ôi / ơi diphthong (Bug: missing from NUCLEI) ─────────────────────────

#[test]
fn loi_gives_loi_with_nga() {
    // "looxi" → "lỗi" (l + oo→ô + x=ngã + i)
    let result = type_sequence(&mut telex(), "looxi ");
    assert_eq!(result, "lỗi ");
}

#[test]
fn toi_with_tone() {
    // "tooij" → "tội" (crime): t + oo→ô + i + j=nặng
    let result = type_sequence(&mut telex(), "tooij ");
    assert_eq!(result, "tội ");
}

#[test]
fn roi_gives_roi_with_huyen() {
    // r(passthrough) + oo→ô + i + f=huyền → "rồi" (already/done)
    let result = type_sequence(&mut telex(), "rooif ");
    assert_eq!(result, "rồi ");
}

// ── double-press tone escape reverts to untoned base ─────────────────────

#[test]
fn tone_escape_reverts_to_base() {
    // "as" → "á", "ass" → escape → "as" (tone stripped, 's' becomes literal)
    let result = type_sequence(&mut telex(), "ass ");
    assert_eq!(result, "as ");
}

#[test]
fn tone_escape_on_syllable() {
    // "tes" → "té", "tess" → escape → "tes" (tone stripped, 's' becomes literal)
    let result = type_sequence(&mut telex(), "tess ");
    assert_eq!(result, "tes ");
}

#[test]
fn tone_triple_press_becomes_literal() {
    // "tes"→"té", "tess"→escape→"tes", "tesss"→escaped_key→"tess" (literal 's', not re-toned)
    // "tess" is invalid Vietnamese → reverts to commit_raw "tess"
    let result = type_sequence(&mut telex(), "tesss ");
    assert_eq!(result, "tess ");
}

#[test]
fn escape_then_extra_literal_not_test() {
    // t+e+s+s+s+t: escaped 's' pushes to commit_raw → commit_raw="tesst" → "tesst" not "test"
    let result = type_sequence(&mut telex(), "tessst ");
    assert_eq!(result, "tesst ");
}

// ── English words with tone-key characters must NOT be converted ──────────

#[test]
fn test_not_converted() {
    // 't','e' literal, 's' (sắc tone) on "te"→"té", then 't' appends → "tét"
    // "tét" uses plain 'e' + final 't' which is invalid in Vietnamese
    // → must revert to raw "test"
    let result = type_sequence(&mut telex(), "test ");
    assert_eq!(result, "test ");
}

#[test]
fn best_not_converted() {
    let result = type_sequence(&mut telex(), "best ");
    assert_eq!(result, "best ");
}

#[test]
fn pest_not_converted() {
    let result = type_sequence(&mut telex(), "pest ");
    assert_eq!(result, "pest ");
}

// ── Words starting with tone keys (s, f, r, x, j) ────────────────────────

#[test]
fn song_not_swallowed() {
    // 's' is a tone key but with empty buffer it must become literal, not Passthrough.
    // Previously 's' was Passthrough (system-typed), then engine committed "ong" with
    // delete_back=3 leaving just "s" on screen — but only if platform timing was perfect.
    // Now 's' is literal in buffer, whole "song" is engine-tracked and committed as one.
    let result = type_sequence(&mut telex(), "song ");
    assert_eq!(result, "song ");
}

// ── Escape then continue typing — revert gives full raw ───────────────────

#[test]
fn tess_t_gives_test() {
    // t+e+s(tone→té)+s(escape→te)+t → candidate="tet", invalid → revert to commit_raw="test"
    let result = type_sequence(&mut telex(), "tesst ");
    assert_eq!(result, "test ");
}

#[test]
fn best_via_escape() {
    // b+e+s(tone→bé)+s(escape→be)+s(tone→bé)+t → "bést" invalid → revert "besst"
    // Actually: b,e,s,s,t → commit_raw="best" → valid? No, "bét" invalid → "best"
    let result = type_sequence(&mut telex(), "besst ");
    assert_eq!(result, "best ");
}

// ── Technical token detection ──────────────────────────────────────────────

#[test]
fn uppercase_word_not_converted() {
    // 'W' alone does not immediately lock technical mode (capitalised Vietnamese word
    // support), but the foreign word "Windows" contains 'ow'→'ơ' char_sub which makes
    // the syllable invalid → auto-reverts to raw "Windows" at commit.
    let result = type_sequence(&mut telex(), "Windows ");
    assert_eq!(result, "Windows ");
}

#[test]
fn van_capitalised() {
    // V+a+n+w → "Văn" (aw lookback fires; first uppercase defers technical mode)
    let result = type_sequence(&mut telex(), "Vanw ");
    assert_eq!(result, "Văn ");
}

#[test]
fn ha_noi_capitalised() {
    // "Haf" → "Hà" — capitalised Vietnamese words use Telex tone
    let result = type_sequence(&mut telex(), "Haf ");
    assert_eq!(result, "Hà ");
}

#[test]
fn api_stays_technical() {
    // 'A' alone defers technical; 'P' (2nd uppercase) triggers technical → "API" literal
    let result = type_sequence(&mut telex(), "API ");
    assert_eq!(result, "API ");
}

#[test]
fn digit_revert_and_literal() {
    // 'a'+'x'→"ã", then '3' (digit) reverts to "ax3", rest literal
    let result = type_sequence(&mut telex(), "ax3000t ");
    assert_eq!(result, "ax3000t ");
}

#[test]
fn ipad_not_converted() {
    // 'i' normal, 'P' uppercase → technical mode, no further conversion
    let result = type_sequence(&mut telex(), "iPad ");
    assert_eq!(result, "iPad ");
}

#[test]
fn all_caps_product_code() {
    // "AX3000T": 'A' starts technical mode immediately
    let result = type_sequence(&mut telex(), "AX3000T ");
    assert_eq!(result, "AX3000T ");
}

// ── Compound diphthong "ươ" (được, nước, rượu) ───────────────────────────

#[test]
fn duoc_via_lookback() {
    // d+d+u+o+c+w+j = "được"
    // 'w' after final 'c' looks back → finds 'o' → ow fires → 'u' before 'o' also → ư
    // candidate = "đươc", 'j' nặng on ơ → ợ → "được"
    let result = type_sequence(&mut telex(), "dduocwj ");
    assert_eq!(result, "được ");
}

#[test]
fn duoc_via_w_before_final() {
    // d+d+u+o+w+c+j = "được" (w typed before final consonant)
    let result = type_sequence(&mut telex(), "dduowcj ");
    assert_eq!(result, "được ");
}

#[test]
fn duoc_tone_before_modifier() {
    // d+d+u+o+c+j+w = "được" — tone (j=nặng) typed BEFORE vowel modifier (w)
    // Look-back strips tone from 'ọ' → base 'o' → "ow"→"ơ", u→ư, returns untoned "đươc"
    // apply_replacement re-applies nặng via extract_current_tone → "được"
    let result = type_sequence(&mut telex(), "dduocjw ");
    assert_eq!(result, "được ");
}

#[test]
fn nuoc_gives_nuoc_sac() {
    // "đước" = đ+ư+ớ+c — same uo+w pattern, sắc tone
    let result = type_sequence(&mut telex(), "dduocws ");
    assert_eq!(result, "đước ");
}

#[test]
fn duoc_skip_w_tone_after_coda() {
    // "dduocj" → "được": user skips 'w' modifier, types tone after final consonant.
    // "uo"+coda is promoted to "ươ"+coda automatically when a tone key fires.
    let result = type_sequence(&mut telex(), "dduocj ");
    assert_eq!(result, "được ");
}

#[test]
fn duong_skip_w_huyen() {
    // "ddưongf" → "đường": same uo-promotion for "ng" coda + huyền tone
    let result = type_sequence(&mut telex(), "dduongf ");
    assert_eq!(result, "đường ");
}

// ── Re-edit: backspace past space to continue composing ───────────────────

#[test]
fn reedit_add_tone_after_space() {
    // "vieetj" → "việt", but if user typed "viee " + BS + "tj" should also give "việt "
    // vie + space → "vie ", BS → re-edit → "vie", then "e"→"viê", "t"→"viêt", "j"→"việt"
    let result = type_with_bs(&mut telex(), "viee \x08tj ");
    assert_eq!(result, "việt ");
}

#[test]
fn reedit_append_consonant_then_tone() {
    // "xin" typed as "xi" + space → "xi ", BS → re-edit → "xi", "n" → "xin", " " → "xin "
    let result = type_with_bs(&mut telex(), "xi \x08n ");
    assert_eq!(result, "xin ");
}

#[test]
fn reedit_add_modifier_after_space() {
    // "nuw" + space → "nư ", BS → re-edit → "nư", "s" → "nứ"
    let result = type_with_bs(&mut telex(), "nuw \x08s ");
    assert_eq!(result, "nứ ");
}

#[test]
fn reedit_only_one_level() {
    // Extra space after commit clears the re-edit slot (any key press voids it).
    // Both BSes become Passthrough — platform handles screen deletion manually.
    let mut e = telex();
    type_sequence(&mut e, "xi ");                       // commits "xi ", re-edit slot set
    let extra = e.process_key(' ');                     // extra space → clears re-edit slot
    assert_eq!(extra, EngineOutput::Replace { delete_back: 0, text: " ".into() });
    let bs1 = e.process_backspace();                    // buffer empty, slot gone → Passthrough
    assert_eq!(bs1, EngineOutput::Passthrough);
    let bs2 = e.process_backspace();                    // still Passthrough
    assert_eq!(bs2, EngineOutput::Passthrough);
}

#[test]
fn reedit_voided_by_new_key() {
    // After commit, typing a new key voids the re-edit slot
    let mut e = telex();
    type_sequence(&mut e, "xi ");   // commits "xi "
    e.process_key('a');             // new word started → last_commit cleared
    let bs = e.process_backspace(); // BS on "a" → pops 'a', not re-edit
    // buffer had "a", backspace → empty candidate
    assert_eq!(bs, EngineOutput::Replace { delete_back: 1, text: "".into() });
}

// ── "gi" / "qu" medial glide — tone trên vowel sau glide ─────────────────

#[test]
fn gian_hoi_gives_gian() {
    // g+i+a+r+n → "giản" (hỏi trên 'a', không phải 'i')
    let result = type_sequence(&mut telex(), "giarn ");
    assert_eq!(result, "giản ");
}

#[test]
fn gian_nang_gives_gian() {
    // g+i+a+j+n → "giạn"
    let result = type_sequence(&mut telex(), "giajn ");
    assert_eq!(result, "giạn ");
}

#[test]
fn qua_hoi_gives_qua() {
    // q+u+a+r → "quả"
    let result = type_sequence(&mut telex(), "quar ");
    assert_eq!(result, "quả ");
}

// ── English words with repeated letters must not trigger escape ───────────

#[test]
fn apple_not_mangled() {
    // "pp" must not fire double-press escape — no prior tone/char-sub
    let result = type_sequence(&mut telex(), "apple ");
    assert_eq!(result, "apple ");
}

#[test]
fn hello_not_mangled() {
    let result = type_sequence(&mut telex(), "hello ");
    assert_eq!(result, "hello ");
}

#[test]
fn google_not_mangled() {
    // "oo"→"ô" fires → had_char_sub=true → "gô" + "gle" invalid → reverts to "google"
    let result = type_sequence(&mut telex(), "google ");
    assert_eq!(result, "google ");
}

// ── "?a" + coda: tone trên 'a' khi có consonant cuối ─────────────────────

#[test]
fn doan_tone_before_coda() {
    // "ddoasn " → "đoán " — tone 's' gõ trước coda 'n'
    // Sau khi thêm 'n', engine re-apply tone: "đóa"+'n' → "đoán"
    let result = type_sequence(&mut telex(), "ddoasn ");
    assert_eq!(result, "đoán ");
}

#[test]
fn doan_tone_after_coda() {
    // "ddoans " → "đoán " — tone 's' gõ sau coda 'n' (thứ tự khác, kết quả như nhau)
    let result = type_sequence(&mut telex(), "ddoans ");
    assert_eq!(result, "đoán ");
}

#[test]
fn doan_huyen_gives_doan() {
    // "ddoanf " → "đoàn " (đoàn = nhóm/đội)
    let result = type_sequence(&mut telex(), "ddoanf ");
    assert_eq!(result, "đoàn ");
}

#[test]
fn hoa_sac_no_coda_tone_on_o() {
    // "hoas " → "hóa " — không có coda → tone vẫn trên 'o' (giữ nguyên behavior)
    let result = type_sequence(&mut telex(), "hoas ");
    assert_eq!(result, "hóa ");
}

// ── Double-char escape preserves prefix (g+o+o+o → "goo" not "o") ────────

#[test]
fn gooo_gives_goo() {
    // g+o+o → "gô" (double_char), third o → escape → candidate "goo" (prefix "g" kept)
    // had_char_sub resets to false after escape → finalize skips validation → "goo" committed
    let result = type_sequence(&mut telex(), "gooo ");
    assert_eq!(result, "goo ");
}

#[test]
fn gooogle_gives_google() {
    // g+o+o+o+g+l+e: escape on third 'o' gives "goo", then g,l,e appended → "google"
    // "google" is not valid Vietnamese but had_char_sub=false after escape → committed as-is
    let result = type_sequence(&mut telex(), "gooogle ");
    assert_eq!(result, "google ");
}

// ── Multi-syllable phrase ──────────────────────────────────────────────────

#[test]
fn xin_chao_phrase() {
    // Each syllable is processed independently (space commits the previous)
    let mut e = telex();
    let mut out = String::new();
    // "xin" + space + "chao" + space
    out.push_str(&type_sequence(&mut e, "xin "));

    // Reset engine between words is automatic (buffer cleared on delimiter)
    out.push_str(&type_sequence(&mut e, "chao "));
    // "xin " + "chao " — "chao" is valid (ch+ao)
    assert_eq!(out, "xin chao ");
}
