/// Frequency ranking for Vietnamese words and English dev-term recognition.
///
/// VI_FREQ: top Vietnamese words in approximate corpus frequency order.
///   vi_rank(word) → u32, lower = more common, u32::MAX = unknown.
///   Used to sort get_suggestions() output so the most likely word appears first.
///
/// EN_DEV: common English names/terms used in software development.
///   is_en_dev(word) → bool
///   Used in finalize_buffer() as a safety net: if Telex somehow produced a
///   known English dev term as the candidate, revert to raw keystrokes.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

// ── Vietnamese word frequency ─────────────────────────────────────────────

/// Top Vietnamese words ordered by frequency (index 0 = most frequent).
/// Source: approximated from common Vietnamese text corpora.
static VI_FREQ: &[&str] = &[
    // ── Function words / particles (highest frequency) ────────────────────
    "không", "của", "và", "là", "có", "trong", "được", "cho",
    "với", "những", "đó", "về", "từ", "khi", "thì", "cũng",
    "như", "đây", "theo", "lại", "vào", "ra", "còn", "đã",
    "sẽ", "nhiều", "hơn", "bị", "đến", "phải", "nên", "thế",
    "qua", "nếu", "sau", "trên", "dưới", "mà", "hay", "rất",
    "đang", "làm", "đi", "lên", "xuống", "gì", "ai", "bao",
    "các", "một", "này", "kia", "đây", "đó", "vì", "để",
    "tuy", "dù", "mặc", "dầu", "nên", "vậy", "thì", "mới",
    "chỉ", "ngay", "vừa", "cũng", "đều", "lại", "rồi", "thôi",
    "hết", "cả", "mọi", "từng", "suốt", "khắp", "quanh",

    // ── Pronouns / address terms ───────────────────────────────────────────
    "tôi", "bạn", "anh", "chị", "em", "ông", "bà", "họ",
    "ta", "mình", "chúng", "người", "con", "cô", "chú", "bác",
    "thầy", "cô", "bé", "trẻ", "già",

    // ── Common nouns ───────────────────────────────────────────────────────
    "năm", "ngày", "tháng", "giờ", "phút", "giây", "tuần",
    "sáng", "chiều", "tối", "đêm", "hôm", "mai", "mùa",
    "nước", "nhà", "đường", "trường", "thành", "phố", "tỉnh",
    "quốc", "gia", "dân", "tộc", "xã", "hội", "chính", "phủ",
    "công", "ty", "doanh", "nghiệp", "kinh", "tế", "tài",
    "chính", "ngân", "hàng", "thị", "trường", "sản", "phẩm",
    "dịch", "vụ", "khách", "hàng", "đối", "tác", "hợp", "đồng",
    "dự", "án", "kế", "hoạch", "chiến", "lược", "mục", "tiêu",
    "kết", "quả", "hiệu", "suất", "chất", "lượng", "giá", "trị",
    "thông", "tin", "dữ", "liệu", "hệ", "thống", "phần", "mềm",
    "máy", "tính", "điện", "thoại", "internet", "mạng", "lưới",
    "công", "nghệ", "khoa", "học", "nghiên", "cứu", "phát", "triển",
    "giáo", "dục", "đào", "tạo", "sinh", "viên", "học", "sinh",
    "giáo", "viên", "bác", "sĩ", "kỹ", "sư", "nhà", "văn",
    "nghệ", "sĩ", "ca", "sĩ", "diễn", "viên", "vận", "động",
    "thể", "thao", "âm", "nhạc", "nghệ", "thuật", "văn", "hóa",
    "lịch", "sử", "địa", "lý", "toán", "học", "vật", "lý",
    "hóa", "học", "sinh", "học", "môi", "trường", "thiên", "nhiên",
    "thực", "phẩm", "ăn", "uống", "sức", "khỏe", "bệnh", "viện",
    "thuốc", "điều", "trị", "phẫu", "thuật",

    // ── Common verbs ───────────────────────────────────────────────────────
    "nói", "thấy", "biết", "muốn", "cần", "giúp", "dùng",
    "đặt", "đưa", "lấy", "mang", "đem", "chơi", "học", "dạy",
    "viết", "đọc", "nghe", "nhìn", "ăn", "uống", "ngủ", "thức",
    "dậy", "đứng", "ngồi", "chạy", "bước", "tìm", "gặp", "gọi",
    "trả", "lời", "hỏi", "giải", "thích", "hiểu", "nhớ", "quên",
    "nghĩ", "cảm", "thấy", "thích", "yêu", "ghét", "sợ", "vui",
    "buồn", "giận", "khóc", "cười", "nằm", "bơi", "bay", "nhảy",
    "hát", "múa", "vẽ", "chụp", "quay", "phát", "sóng", "chiếu",
    "xem", "theo", "dõi", "kiểm", "tra", "phân", "tích", "đánh",
    "giá", "cải", "thiện", "xây", "dựng", "phát", "triển", "mở",
    "rộng", "thu", "hẹp", "tăng", "giảm", "thêm", "bớt", "xóa",
    "sửa", "cập", "nhật", "tải", "lên", "xuống", "chia", "sẻ",
    "gửi", "nhận", "đăng", "ký", "đăng", "nhập", "thoát", "khởi",

    // ── Adjectives ─────────────────────────────────────────────────────────
    "tốt", "xấu", "đẹp", "lớn", "nhỏ", "mới", "cũ", "trẻ",
    "già", "cao", "thấp", "dài", "ngắn", "rộng", "hẹp", "nhanh",
    "chậm", "mạnh", "yếu", "sạch", "bẩn", "khó", "dễ", "đúng",
    "sai", "thật", "giả", "quan", "trọng", "cần", "thiết", "phù",
    "hợp", "hiệu", "quả", "tiện", "lợi", "an", "toàn", "bảo",
    "mật", "chuyên", "nghiệp", "sáng", "tạo", "linh", "hoạt",
    "thông", "minh", "nhanh", "nhẹn", "kiên", "nhẫn", "trách",
    "nhiệm", "trung", "thực", "công", "bằng", "hòa", "bình",

    // ── Numbers / quantifiers ──────────────────────────────────────────────
    "một", "hai", "ba", "bốn", "năm", "sáu", "bảy", "tám",
    "chín", "mười", "trăm", "nghìn", "triệu", "tỷ", "đầu",
    "cuối", "giữa", "trước", "sau", "trên", "dưới", "trong", "ngoài",
    "bên", "cạnh", "gần", "xa", "đây", "đó", "kia", "đâu",

    // ── Tech / digital ─────────────────────────────────────────────────────
    "máy", "tính", "điện", "thoại", "màn", "hình", "bàn", "phím",
    "chuột", "camera", "loa", "tai", "nghe", "pin", "sạc",
    "wifi", "bluetooth", "internet", "mạng", "server", "website",
    "ứng", "dụng", "phần", "mềm", "lập", "trình", "code", "debug",
    "deploy", "database", "frontend", "backend", "giao", "diện",
    "người", "dùng", "tài", "khoản", "mật", "khẩu", "xác", "thực",
    "bảo", "mật", "mã", "hóa", "dữ", "liệu", "đám", "mây",
    "điện", "toán", "trí", "tuệ", "nhân", "tạo", "học", "máy",
    "xử", "lý", "ngôn", "ngữ", "tự", "nhiên", "hình", "ảnh",
    "âm", "thanh", "video", "livestream", "nội", "dung", "quảng",
    "cáo", "thương", "mại", "điện", "tử", "thanh", "toán",
    "giao", "dịch", "tài", "khoản", "ngân", "hàng", "ví",
    "điện", "tử", "tiền", "mã", "hóa",

    // ── Common Vietnamese single-syllable words ────────────────────────────
    "à", "ừ", "ơi", "nhé", "nào", "thôi", "vậy", "ấy", "ạ",
    "ư", "ôi", "ủa", "chứ", "cơ", "mà", "nhỉ", "nhé", "đấy",
    "đây", "kia", "đó", "đâu", "sao", "thế", "vậy", "bao",
];

fn vi_map() -> &'static HashMap<String, u32> {
    static MAP: OnceLock<HashMap<String, u32>> = OnceLock::new();
    MAP.get_or_init(|| {
        VI_FREQ.iter()
            .enumerate()
            .map(|(rank, &word)| (word.to_string(), rank as u32))
            .collect()
    })
}

/// Frequency rank of a Vietnamese word (0 = most common, u32::MAX = unknown).
pub fn vi_rank(word: &str) -> u32 {
    vi_map().get(word).copied().unwrap_or(u32::MAX)
}

// ── English dev terms ─────────────────────────────────────────────────────

/// Common English names and terms used in software development.
/// Only words ≥ 5 chars to avoid false-positive with short Vietnamese syllables.
static EN_DEV: &[&str] = &[
    // Programming languages
    "javascript", "typescript", "python", "kotlin", "swift", "golang",
    "haskell", "elixir", "clojure", "erlang", "scala", "groovy",
    "fortran", "cobol", "pascal", "delphi", "matlab", "julia",
    // Frameworks / libraries
    "react", "angular", "nextjs", "nuxtjs", "svelte", "astro", "remix",
    "vuejs", "alpine", "htmx", "jquery", "redux", "zustand", "mobx",
    "django", "fastapi", "flask", "starlette", "tornado",
    "express", "nestjs", "fastify", "hapi", "koa",
    "spring", "quarkus", "micronaut", "jakarta",
    "rails", "sinatra", "hanami",
    "laravel", "symfony", "codeigniter", "yii2",
    "flutter", "swiftui", "jetpack", "compose",
    "tensorflow", "pytorch", "keras", "sklearn",
    "numpy", "pandas", "polars", "scipy", "matplotlib",
    // Databases
    "mysql", "postgres", "sqlite", "mongodb", "redis",
    "cassandra", "dynamodb", "elasticsearch", "opensearch",
    "cockroach", "tidb", "vitess", "planetscale",
    "supabase", "firebase", "appwrite", "convex",
    "prisma", "drizzle", "typeorm", "sequelize",
    // Cloud / DevOps
    "docker", "kubernetes", "terraform", "ansible", "puppet",
    "jenkins", "github", "gitlab", "bitbucket", "jira",
    "vercel", "netlify", "heroku", "render", "railway",
    "nginx", "apache", "traefik", "caddy", "haproxy",
    "grafana", "prometheus", "datadog", "sentry", "newrelic",
    "cloudflare", "fastly", "akamai",
    // Platforms / services
    "google", "apple", "amazon", "stripe", "twilio",
    "microsoft", "shopify", "salesforce", "atlassian",
    "openai", "anthropic", "cohere", "mistral", "groq",
    "slack", "discord", "notion", "linear", "figma",
    "postman", "insomnia", "swagger", "openapi",
    // Common tech terms (≥5 chars)
    "localhost", "readme", "markdown", "boolean", "integer",
    "string", "float", "double", "object", "array", "tuple",
    "class", "struct", "interface", "abstract", "override",
    "async", "await", "promise", "callback", "closure",
    "import", "export", "module", "package", "library",
    "return", "throw", "catch", "finally", "yield",
    "const", "static", "public", "private", "protected",
    "function", "method", "property", "attribute", "annotation",
    "serialize", "deserialize", "encode", "decode",
    "encrypt", "decrypt", "hash", "token", "oauth",
    "webhook", "socket", "stream", "buffer", "queue",
    "thread", "process", "goroutine", "coroutine",
    "mutex", "semaphore", "deadlock", "race", "atomic",
    "benchmark", "profiler", "coverage", "unittest",
    "staging", "production", "deployment", "rollback",
    "container", "runtime", "compiler", "interpreter",
    "transpiler", "bundler", "linter", "formatter",
    "repository", "branch", "commit", "merge", "rebase",
    "pipeline", "workflow", "action", "trigger", "cron",
    "endpoint", "payload", "schema", "migration",
    "indexing", "caching", "sharding", "replication",
    "latency", "throughput", "timeout", "retry",
    "localhost", "dotenv", "config", "secret",
    // Common proper nouns / tools
    "github", "vscode", "jetbrains", "neovim", "emacs",
    "chrome", "firefox", "safari", "electron", "tauri",
    "linux", "ubuntu", "debian", "fedora", "alpine",
    "macos", "windows", "android", "iphone", "ipad",
    "tailwind", "bootstrap", "chakra", "material", "shadcn",
];

fn en_dev_set() -> &'static HashSet<String> {
    static SET: OnceLock<HashSet<String>> = OnceLock::new();
    SET.get_or_init(|| EN_DEV.iter().map(|&w| w.to_string()).collect())
}

/// Returns true if `word` is a recognized English dev term.
/// Case-insensitive. Only fires for words ≥ 5 chars to avoid false-positives
/// with short Vietnamese syllables (e.g. "go", "in", "an").
pub fn is_en_dev(word: &str) -> bool {
    if word.chars().count() < 5 {
        return false;
    }
    let lower = word.to_lowercase();
    en_dev_set().contains(&lower)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vi_rank_known_word() {
        assert!(vi_rank("không") < vi_rank("tốt"),   "không phổ biến hơn tốt");
        assert!(vi_rank("tôi")   < u32::MAX,         "tôi phải có trong bảng");
        assert_eq!(vi_rank("xyzabc"), u32::MAX,      "từ lạ → MAX");
    }

    #[test]
    fn vi_rank_sorts_suggestions() {
        let mut words = vec!["tội".to_string(), "tôi".to_string(), "tồi".to_string()];
        words.sort_by_key(|w| vi_rank(w));
        assert_eq!(words[0], "tôi", "tôi phải đứng đầu");
    }

    #[test]
    fn en_dev_known_terms() {
        assert!(is_en_dev("react"),      "react là dev term");
        assert!(is_en_dev("Docker"),     "case-insensitive");
        assert!(is_en_dev("GITHUB"),     "all-caps");
        assert!(is_en_dev("javascript"), "javascript recognized");
    }

    #[test]
    fn en_dev_short_words_excluded() {
        // Short words must not be protected — could be Vietnamese syllables
        assert!(!is_en_dev("go"),   "go quá ngắn");
        assert!(!is_en_dev("in"),   "in quá ngắn");
        assert!(!is_en_dev("api"),  "api quá ngắn");
        assert!(!is_en_dev("sql"),  "sql quá ngắn");
    }

    #[test]
    fn en_dev_vietnamese_not_matched() {
        assert!(!is_en_dev("không"), "từ thuần Việt không phải dev term");
        assert!(!is_en_dev("được"),  "từ thuần Việt");
    }
}
