# PKTCore — Claude Code Instructions

> File này là **luật bất biến** cho mọi session. Đọc kỹ trước khi sinh code.

## Project

PKTCore là blockchain PoW lấy cảm hứng Bitcoin, dùng PacketCrypt PoW (CPU/GPU/NPU, không ASIC). Viết bằng Rust, kiến trúc Cargo workspace: `pkt-core` (consensus/storage/p2p), `pkt-sdk` (client lib), `pkt-api` (REST/RPC). Phát triển trên macOS, deploy binary lên VPS `oceif` (user `tuyenpkt`).

## Ngôn ngữ giao tiếp

- Trả lời **100% tiếng Việt**, ngắn gọn, thực dụng.
- Không xã giao thừa, không "Tuyệt vời!", không lặp câu hỏi.
- Trả lời sai → nói thẳng "tôi sai", sửa, không xin lỗi dài dòng.

---

## 🛑 NGUYÊN TẮC TỐI THƯỢNG — KHÔNG MẮC LỖI

Đây là phần quan trọng nhất. Vi phạm = code lỗi = phải sửa lại.

### 1. Verify trước khi viết
- **Không bịa API.** Trước khi gọi method/crate, đọc file thật hoặc `cargo doc --open`. Nếu không chắc → grep codebase, không đoán.
- **Không bịa version.** Crate version lấy từ `Cargo.toml` thật, không viết `tokio = "1.45"` nếu chưa check.
- **Đọc file liên quan trước khi sửa.** Trước khi edit `src/foo.rs`, xem nó và mọi nơi import nó (`grep -r "use crate::foo"`).

### 2. Plan trước khi code thay đổi >20 dòng
- Liệt kê: file nào sửa, function nào thêm, breaking change không?
- Đợi xác nhận với task lớn. Task nhỏ (<20 dòng, 1 file) thì làm thẳng.

### 3. Compile & test SAU MỖI thay đổi đáng kể
- Chạy `cargo check --workspace` ngay sau edit. **Không báo "xong" khi chưa compile.**
- Chạy `cargo test -p <crate>` cho crate vừa đụng tới.
- Lỗi compile → fix trước khi viết code mới. Không chồng lỗi.

### 4. Không phá kiến trúc hiện có
- Tôn trọng ranh giới crate: `pkt-core` không phụ thuộc `pkt-api`/`pkt-sdk`. Một chiều: api/sdk → core.
- Không thêm dependency mới nếu chưa hỏi (mỗi crate mới = attack surface + build time).
- Không refactor "tiện thể". Một PR — một mục đích.

### 5. Khi không chắc — DỪNG và HỎI
- Tốt hơn hỏi 1 câu rõ ràng còn hơn sinh 200 dòng sai hướng.
- Không "đoán theo cảm giác" với consensus logic, mật mã, hay UTXO state — sai chỗ này = mất tiền thật.

---

## Commands

```bash
# Build & check (chạy thường xuyên)
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo fmt --all -- --check

# Release build (chỉ khi cần)
cargo build --release --workspace

# Cross-compile cho VPS oceif (Linux x86_64)
cargo build --release --target x86_64-unknown-linux-gnu

# Deploy
scp target/x86_64-unknown-linux-gnu/release/<bin> tuyenpkt@oceif:~/pkt/
```

Trước khi commit: **clippy sạch + fmt + test pass**. Không có ngoại lệ.

## Rust conventions

- Edition 2021, MSRV theo `Cargo.toml`. Không dùng feature nightly.
- **`unsafe` cần justification comment** ngay phía trên (`// SAFETY: ...`). Không dùng `unsafe` để né borrow checker.
- **`unwrap()`/`expect()`** chỉ trong test và `main()` setup. Production code → `Result<_, E>` với error type riêng (dùng `thiserror`).
- **`as` cast** giữa số nguyên có thể mất data → dùng `try_into()` cho amount/balance/nonce.
- Public API có doc comment `///`. Module phức tạp có `//!` ở đầu file.
- Đặt tên rõ: `verify_block_header`, không phải `check_b`.

## Blockchain-specific rules

- **Mọi thay đổi consensus logic** (validation, hashing, difficulty, PoW) phải có test vector cụ thể, đối chiếu với reference impl (PacketCrypt gốc nếu có).
- **Cryptography**: chỉ dùng crate đã review (`blake3`, `secp256k1`, `ed25519-dalek`, `ring`). **Không tự implement primitive.**
- **Serialization** phải deterministic — dùng `bincode` config strict hoặc encoding tự viết có spec.
- **Integer overflow**: dùng `checked_add/sub/mul` cho amount, supply, fee. Không bao giờ `wrapping_*` trừ khi cố ý (comment rõ).
- **Random**: chỉ `OsRng` cho key generation. Không `thread_rng` cho crypto.
- Comment mọi assumption về byte order (big/little endian), đặc biệt khi serialize hash/nonce.

## Workflow macOS → VPS oceif

- Dev và test trên macOS (Apple Silicon).
- Cross-compile lên target Linux trước khi deploy. Không build trực tiếp trên VPS (tốn RAM, chậm).
- SSH key only, password disabled. Fail2ban đang chạy.
- Log lưu ở `~/pkt/logs/` trên VPS, rotate bằng `logrotate`.

## Anti-patterns — TUYỆT ĐỐI KHÔNG

- ❌ Sinh code rồi nói "chưa test nhưng chắc chạy được".
- ❌ Thêm `#[allow(...)]` để né clippy mà không giải thích.
- ❌ Comment kiểu "// TODO: handle error properly" trong code consensus — fix luôn hoặc đừng viết.
- ❌ Đổi public API mà không grep tất cả call site.
- ❌ Tạo file mới khi có thể sửa file hiện có.
- ❌ Viết test mock-toàn-bộ làm test mất ý nghĩa. Test consensus phải dùng input thật.
- ❌ Commit `Cargo.lock` conflict mà không rebuild.

## Khi gặp bug

1. Reproduce trước, fix sau. Không "fix mù".
2. Viết test fail trước khi sửa code. Test đó pass = fix đúng.
3. Bug consensus/crypto → ưu tiên tuyệt đối, dừng feature khác.

---

*Cập nhật khi convention thay đổi. Giữ file <150 dòng — mỗi dòng phải đáng giá một instruction slot.*
