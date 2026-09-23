//! Кодирование DPI для сенсора PMW3370 — RE `FUN_0040b9c0` (DPI → код)
//! и `FUN_0040bd90` (код → DPI).
//!
//! Одна ступень DPI в EEPROM занимает 4 байта:
//! `[code_x][code_y][mul_x | mul_y << 4][0x55 − Σ]`,
//! `dpi = base(mul) · (code + 1)`, где `base = 50` (mul 0), `100` (mul 1/2),
//! `200` (mul 3).

use super::packet::checksum;

/// Минимальный DPI (`DPIRANGE=50,…`).
pub const MIN: u16 = 50;
/// Максимальный DPI для этой мыши (`DPIRANGE=…,19000,100`).
pub const MAX: u16 = 19_000;
/// До этого значения шаг 50, дальше — 100.
pub const FINE_LIMIT: u16 = 10_000;

/// DPI → (код, множитель).
pub fn encode(dpi: u16) -> (u8, u8) {
    let dpi = snap(dpi) as u32;
    let (code, mul) = if dpi <= 10_000 {
        (dpi / 50 - 1, 0)
    } else if dpi <= 19_000 {
        (dpi / 100 - 1, 2)
    } else if dpi <= 19_999 {
        (dpi / 100 - 1, 1)
    } else {
        (dpi / 200 - 1, 3)
    };
    (code as u8, mul)
}

/// (код, множитель) → DPI.
pub fn decode(code: u8, mul: u8) -> u16 {
    let base: u32 = if mul & 2 != 0 { 100 } else { 50 };
    let v = base * (code as u32 + 1);
    (if mul & 1 != 0 { v * 2 } else { v }) as u16
}

/// Приводит значение к допустимой сетке: 50…10000 шаг 50, 10100…19000 шаг 100.
pub fn snap(dpi: u16) -> u16 {
    let dpi = dpi.clamp(MIN, MAX);
    if dpi < FINE_LIMIT + 50 {
        ((dpi + 25) / 50 * 50).min(FINE_LIMIT)
    } else {
        ((dpi + 50) / 100 * 100).clamp(FINE_LIMIT + 100, MAX)
    }
}

/// Кодирует ступень DPI (одинаковый X/Y) в 4 байта EEPROM.
pub fn encode_level(dpi: u16) -> [u8; 4] {
    let (code, mul) = encode(dpi);
    let flags = (mul & 0x0F) | (mul << 4);
    [code, code, flags, checksum(&[code, code, flags])]
}

/// Декодирует ступень DPI (берётся ось X). `None` — битая контрольная сумма.
pub fn decode_level(raw: &[u8]) -> Option<u16> {
    if raw.len() < 4 || raw[3] != checksum(&raw[..3]) {
        return None;
    }
    let dpi = decode(raw[0], raw[2] & 0x03);
    (MIN..=38_000).contains(&dpi).then_some(dpi)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_eeprom_levels() {
        // Реальная таблица 0x0C…0x2B с мыши.
        let live: [[u8; 4]; 8] = [
            [0x07, 0x07, 0x00, 0x47],
            [0x0F, 0x0F, 0x00, 0x37],
            [0x1F, 0x1F, 0x00, 0x17],
            [0x2F, 0x2F, 0x00, 0xF7],
            [0x3F, 0x3F, 0x00, 0xD7],
            [0x7F, 0x7F, 0x00, 0x57],
            [0x77, 0x77, 0x22, 0x45],
            [0xBD, 0xBD, 0x22, 0xB9],
        ];
        let expect = [400, 800, 1600, 2400, 3200, 6400, 12_000, 19_000];
        for (raw, dpi) in live.iter().zip(expect) {
            assert_eq!(decode_level(raw), Some(dpi));
            assert_eq!(&encode_level(dpi), raw, "encode {dpi}");
        }
    }

    #[test]
    fn snapping() {
        assert_eq!(snap(0), 50);
        assert_eq!(snap(820), 800);
        assert_eq!(snap(830), 850);
        assert_eq!(snap(10_020), 10_000);
        assert_eq!(snap(10_060), 10_100);
        assert_eq!(snap(25_000), 19_000);
    }

    #[test]
    fn roundtrip_whole_range() {
        let mut d = MIN;
        while d <= MAX {
            let (c, m) = encode(d);
            assert_eq!(decode(c, m), d);
            d += if d < FINE_LIMIT { 50 } else { 100 };
        }
    }

    #[test]
    fn broken_checksum_rejected() {
        assert_eq!(decode_level(&[0x07, 0x07, 0x00, 0x48]), None);
    }
}
