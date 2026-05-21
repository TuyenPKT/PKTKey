/// Map a Vietnamese character (with tone + vowel modifier) to its plain ASCII equivalent.
pub fn strip_viet_char(c: char) -> char {
    match c {
        'à'|'á'|'ả'|'ã'|'ạ'|'â'|'ấ'|'ầ'|'ẩ'|'ẫ'|'ậ'|'ă'|'ắ'|'ằ'|'ẳ'|'ẵ'|'ặ' => 'a',
        'À'|'Á'|'Ả'|'Ã'|'Ạ'|'Â'|'Ấ'|'Ầ'|'Ẩ'|'Ẫ'|'Ậ'|'Ă'|'Ắ'|'Ằ'|'Ẳ'|'Ẵ'|'Ặ' => 'A',
        'è'|'é'|'ẻ'|'ẽ'|'ẹ'|'ê'|'ế'|'ề'|'ể'|'ễ'|'ệ' => 'e',
        'È'|'É'|'Ẻ'|'Ẽ'|'Ẹ'|'Ê'|'Ế'|'Ề'|'Ể'|'Ễ'|'Ệ' => 'E',
        'ì'|'í'|'ỉ'|'ĩ'|'ị' => 'i',
        'Ì'|'Í'|'Ỉ'|'Ĩ'|'Ị' => 'I',
        'ò'|'ó'|'ỏ'|'õ'|'ọ'|'ô'|'ố'|'ồ'|'ổ'|'ỗ'|'ộ'|'ơ'|'ớ'|'ờ'|'ở'|'ỡ'|'ợ' => 'o',
        'Ò'|'Ó'|'Ỏ'|'Õ'|'Ọ'|'Ô'|'Ố'|'Ồ'|'Ổ'|'Ỗ'|'Ộ'|'Ơ'|'Ớ'|'Ờ'|'Ở'|'Ỡ'|'Ợ' => 'O',
        'ù'|'ú'|'ủ'|'ũ'|'ụ'|'ư'|'ứ'|'ừ'|'ử'|'ữ'|'ự' => 'u',
        'Ù'|'Ú'|'Ủ'|'Ũ'|'Ụ'|'Ư'|'Ứ'|'Ừ'|'Ử'|'Ữ'|'Ự' => 'U',
        'ỳ'|'ý'|'ỷ'|'ỹ'|'ỵ' => 'y',
        'Ỳ'|'Ý'|'Ỷ'|'Ỹ'|'Ỵ' => 'Y',
        'đ' => 'd',
        'Đ' => 'D',
        _ => c,
    }
}

/// Strip all Vietnamese diacritics from a string, returning plain ASCII.
pub fn strip_viet(s: &str) -> String {
    s.chars().map(strip_viet_char).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_duoc() {
        assert_eq!(strip_viet("được"), "duoc");
    }

    #[test]
    fn strip_nguoi() {
        assert_eq!(strip_viet("người"), "nguoi");
    }

    #[test]
    fn strip_nuoc() {
        assert_eq!(strip_viet("nước"), "nuoc");
    }

    #[test]
    fn strip_toi() {
        assert_eq!(strip_viet("tôi"), "toi");
    }

    #[test]
    fn plain_ascii_unchanged() {
        assert_eq!(strip_viet("hello"), "hello");
    }
}
