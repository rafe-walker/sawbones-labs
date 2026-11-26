//! CRC-32C, the frame's integrity check.

const POLY_REFLECTED: u32 = 0x82F6_3B78;

const TABLE: [u32; 256] = table();

const fn table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                (c >> 1) ^ POLY_REFLECTED
            } else {
                c >> 1
            };
            k += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
}

/// CRC-32C (Castagnoli): reflected, initial value and final XOR
/// `0xFFFF_FFFF`. The check value of `b"123456789"` is `0xE306_9283`.
///
/// It detects accidental corruption, not tampering: anyone can compute it.
#[must_use]
pub fn crc32c(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in bytes {
        crc = TABLE[((crc ^ u32::from(b)) & 0xFF) as usize] ^ (crc >> 8);
    }
    !crc
}
