use std::collections::HashMap;
use std::sync::OnceLock;
use crate::phonetic::strip_viet;

// Common Vietnamese monosyllabic words, ordered roughly by frequency.
// Each word is stored in its canonical toned form; the phonetic (diacritic-stripped)
// form is computed once at startup and used for lookup.
static WORDS: &[&str] = &[
    // ── Ultra-high frequency function words ──────────────────────────────
    "không", "có", "và", "của", "được", "cho", "với", "trong", "đã",
    "là", "sẽ", "đang", "vừa", "vẫn", "còn", "cũng", "cả", "đều",
    "rất", "khá", "quá", "hơi", "cực",
    // ── Personal pronouns / address words ────────────────────────────────
    "tôi", "tao", "mày", "bạn", "anh", "chị", "em", "họ", "mình",
    "ông", "bà", "cha", "mẹ", "bố", "má", "con", "cô", "dì", "chú",
    "bác", "cậu", "thầy",
    // ── Demonstratives / question words ──────────────────────────────────
    "này", "đó", "đây", "kia", "ấy", "nọ",
    "gì", "ai", "nào", "đâu", "sao", "tại",
    // ── Conjunctions / prepositions ───────────────────────────────────────
    "nếu", "khi", "thì", "mà", "vì", "để", "bởi", "do", "hay", "hoặc",
    "nhưng", "song", "tuy", "dù", "dầu", "theo", "giữa", "trên", "dưới",
    "trước", "sau", "bên", "ngoài", "trong", "qua", "từ", "đến", "về",
    // ── Direction / movement verbs ────────────────────────────────────────
    "đi", "về", "ra", "vào", "lên", "xuống", "sang", "qua", "lại", "đến",
    // ── Common action verbs ───────────────────────────────────────────────
    "làm", "nói", "thấy", "nghe", "biết", "hiểu", "nghĩ", "nhớ",
    "muốn", "cần", "phải", "nên", "được", "có thể",
    "mua", "bán", "ăn", "uống", "ngủ", "dậy",
    "đọc", "viết", "học", "dạy", "hỏi", "trả", "kể",
    "mở", "đóng", "bật", "tắt", "sửa", "xây",
    "gặp", "gọi", "nhìn", "xem", "thử", "chọn", "giúp",
    "yêu", "ghét", "thích", "sợ", "vui", "buồn", "giận",
    "chạy", "ngồi", "đứng", "nằm", "bước", "đưa", "lấy",
    // ── Common adjectives ─────────────────────────────────────────────────
    "tốt", "xấu", "đẹp", "hay", "khó", "dễ", "mới", "cũ",
    "lớn", "nhỏ", "cao", "thấp", "nhanh", "chậm",
    "đúng", "sai", "thật", "giả", "rõ", "mờ",
    "đủ", "thiếu", "nhiều", "ít",
    "nặng", "nhẹ", "dài", "ngắn", "rộng", "hẹp",
    "sạch", "bẩn", "nóng", "lạnh", "ấm", "mát",
    "mạnh", "yếu", "già", "trẻ", "sống", "chết",
    // ── Numbers ───────────────────────────────────────────────────────────
    "một", "hai", "ba", "bốn", "năm", "sáu", "bảy", "tám", "chín", "mười",
    "trăm", "nghìn", "triệu",
    // ── Time words ────────────────────────────────────────────────────────
    "ngày", "đêm", "sáng", "chiều", "tối", "trưa",
    "tuần", "tháng", "năm", "giờ", "phút", "giây",
    "hôm", "nay", "mai", "hôm qua", "bây giờ",
    // ── Common nouns ──────────────────────────────────────────────────────
    "người", "nhà", "nước", "việc", "chuyện", "điều",
    "tiếng", "tên", "lúc", "lần", "thứ",
    "gia", "đình", "vợ", "chồng",
    "đường", "phố", "nơi", "chỗ",
    "tiền", "giá", "hàng", "xe",
    "sách", "bài", "lớp", "trường",
    "bác sĩ", "giáo viên",
    // ── Monosyllabic words by phonetic group ─────────────────────────────
    // -a group
    "à", "á", "ả",
    "già", "giả", "giá",
    "kẻ", "lá", "má",
    // -ai group
    "bài", "cài", "dài", "gái", "gai", "lại", "lái",
    "mãi", "mai", "ngoài", "nhai", "sai", "tai", "tài", "tại",
    "vai", "vài",
    // -an group
    "bàn", "bán", "bản", "cần", "dân", "gần",
    "hạn", "lần", "mạnh", "nhân", "nhận", "nhẫn",
    "tân", "tản",
    // -ang group
    "bảng", "băng", "bằng", "làng", "tăng", "tặng",
    "thẳng",
    // -ao group
    "áo", "bao", "báo", "cáo", "cao", "chào", "cháo",
    "dao", "giao", "lao", "màu", "sao", "sáo", "tao", "táo",
    // -at group
    "bắt", "bật", "bát", "đặt", "đạt", "đất", "mắt", "mất",
    "tất", "tắt",
    // -ay group
    "bay", "bảy", "dạy", "dậy", "đây", "đấy", "đầy",
    "lấy", "mây", "ngay", "ngày", "tay", "tây",
    // -eo group
    "bèo", "chèo", "đèo", "kèo", "mèo", "rèo", "theo",
    // -e/ê group
    "chè", "chế", "đề", "kể", "lề", "lễ", "nề", "nễ",
    "quê", "thế", "về",
    // -en group
    "đen", "đèn", "khen", "lên", "lén", "tên",
    // -i group
    "bị", "chí", "chỉ", "dì", "gì", "kỳ", "lý", "lì",
    "mỉ", "nghĩ", "nghỉ", "nghề", "nhỉ", "sĩ", "thì", "thi",
    "tý", "vì", "vị",
    // -iên group
    "biển", "biến", "điền", "điện", "hiển", "liền", "liên",
    "miền", "miễn", "nghiên", "tiền", "tiện", "viền",
    // -iêt/iet group
    "biết", "chiết", "diệt", "kiết", "liệt", "miệt",
    "thiết", "triệt", "viết", "việt",
    // -in group
    "kín", "lịch", "nhìn", "nhịn", "tín",
    // -inh group
    "bình", "chính", "định", "hình", "lịch", "linh", "minh",
    "sinh", "thịnh", "tinh", "tính",
    // -o/ô group
    "bố", "bổ", "bộ", "bò", "bọ",
    "chổi", "cổ", "có", "cũ",
    "đổ", "đo", "đó",
    "ho", "hổ", "hồ",
    "lổ", "lộ", "lo",
    "mổ", "mộ", "mỡ", "mờ", "mở",
    "nổ", "nộ", "nỗ",
    "ổn", "ộc",
    // -oi group
    "bồi", "chơi", "đổi", "đời", "đội", "gọi", "giỏi", "giới",
    "lỗi", "lời", "mới", "môi", "mời", "ngồi",
    "nổi", "nơi", "nội",
    "ơi", "rồi", "tôi", "tội", "tối", "với", "vội",
    // -ong group
    "bóng", "bổng", "công", "còng", "đồng", "đóng",
    "không", "lòng", "lộng", "phòng", "sống", "tổng", "trong",
    // -ot group
    "bột", "gốc", "học", "lốc", "một", "nhọt",
    "quốc", "rốc", "sốc", "tốc",
    // -ơ group
    "bơ", "cơ", "hờ", "lờ", "mơ", "ngờ",
    "như", "nhờ", "sơ", "tơ", "thờ",
    // -on group
    "bốn", "bon", "con", "còn", "đón", "đơn",
    "hơn", "hồn", "lớn", "lồng", "mòn", "nhơn", "nhờn",
    "sơn", "thơn", "tốt", "trơn", "từng",
    // -u/ư group
    "bù", "cứ", "củ", "cụ", "du", "dù", "đủ",
    "hủ", "hư", "lũ", "mũ", "mù",
    "nhụ", "nư", "như",
    "phụ", "phú", "rủ", "rụ",
    "tự", "từ", "tứ", "tư",
    "ừ", "ứ",
    // -ua/ươ group
    "bừa", "chua", "dừa", "dưa", "đưa", "lửa", "lựa",
    "mưa", "nữa", "sữa", "sửa", "tưa", "vừa",
    "mướn", "thuê", "được",
    // -ui group
    "bụi", "cười", "cuối", "cưỡi", "gửi",
    "mui", "mùi", "ngủ", "nuôi", "rủi", "tủi", "vui",
    // -un group
    "buôn", "buồn", "bún", "căn", "gần", "luôn",
    "muôn", "muốn", "muộn", "mượn", "nuốt",
    "thuộc", "tuần", "tuyên",
    // -ung group
    "bụng", "chung", "chúng", "chừng", "cung", "cùng",
    "đứng", "dứng", "lưng", "mừng", "rừng",
    "sung", "tung", "thung", "trung", "vùng", "vững",
    // -uoc group — critical for "được"
    "buộc", "chuộc", "được", "đuốc", "luộc",
    "thuốc", "tuổi", "uống", "ước", "vướng",
    // -uong group
    "buồng", "chuồng", "đường", "dương", "dưỡng",
    "gương", "hướng", "lương", "nương", "phương",
    "sương", "thương", "trường", "tương", "vương",
    // -uy group
    "duy", "huy", "khuya", "quý", "thủy", "tuỳ", "tùy",
    // -uyên group
    "chuyên", "chuyến", "khuyên", "nguyên", "nguyện",
    "quyền", "tuyên", "tuyến", "xuyên",
    // -y group
    "ý", "ỷ", "ỵ", "yếu", "yêu",
    // -ach group
    "bạch", "cách", "đạch", "mạch", "sạch", "tách",
    // -anh group
    "bánh", "cảnh", "cạnh", "canh", "hanh", "hành",
    "lành", "mạnh", "nhanh", "nành", "ranh", "rảnh",
    "sành", "thành", "thanh", "xanh",
    // -ich group
    "địch", "dịch", "ích", "kịch", "lịch",
    "nghịch", "phịch", "tích", "thích", "vịch",
    // -ênh group
    "bệnh", "kênh", "lệnh", "mệnh", "rênh",
    "thênh", "tệnh",
    // -ơng group
    "bường", "cường", "đường", "gương", "hường",
    "lường", "mường", "nường", "phường", "thường",
    "tướng", "vường", "xường",
    // -ươi group
    "bướm", "cười", "mười", "người", "tươi",
    // -ngh- initial
    "nghề", "nghe", "nghĩ", "nghỉ", "nghiêm",
    // Misc high-frequency
    "xe", "xem", "xin", "xong", "xinh",
    "thế", "thật", "thêm", "thì", "thứ",
    "phải", "phố", "phở",
];

static MAP: OnceLock<HashMap<String, Vec<&'static str>>> = OnceLock::new();

fn get_map() -> &'static HashMap<String, Vec<&'static str>> {
    MAP.get_or_init(|| {
        let mut m: HashMap<String, Vec<&'static str>> = HashMap::new();
        for &w in WORDS {
            let key = strip_viet(w).to_lowercase();
            m.entry(key).or_default().push(w);
        }
        m
    })
}

/// Return all dictionary words whose diacritic-stripped form equals `phonetic`.
/// Results are in the order they appear in WORDS (roughly frequency-sorted).
pub fn lookup(phonetic: &str) -> &'static [&'static str] {
    match get_map().get(&phonetic.to_lowercase()) {
        Some(v) => v.as_slice(),
        None    => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_duoc() {
        let r = lookup("duoc");
        assert!(r.contains(&"được"), "expected 'được' in {:?}", r);
    }

    #[test]
    fn lookup_toi() {
        let r = lookup("toi");
        assert!(r.contains(&"tôi"), "expected 'tôi' in {:?}", r);
    }

    #[test]
    fn lookup_nuoc() {
        let r = lookup("nuoc");
        assert!(r.contains(&"nước"), "expected 'nước' in {:?}", r);
    }

    #[test]
    fn lookup_khong() {
        let r = lookup("khong");
        assert!(r.contains(&"không"), "expected 'không' in {:?}", r);
    }

    #[test]
    fn lookup_unknown_returns_empty() {
        assert!(lookup("xyzabc").is_empty());
    }
}
