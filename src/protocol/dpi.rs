//! Кодирование ступеней DPI. Формат ступени в EEPROM общий для мышей на JM03:
//! `[code_x][code_y][flags][0x55 − Σ]`, а вот как `code`/`flags` превращаются
//! в DPI, зависит от сенсора — см. [`Sensor`].

use super::packet::checksum;

/// Сенсор мыши: диапазон DPI и кодек ступени.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sensor {
    /// PixArt PMW3370 (Edge Air Ultra): 50…19 000.
    Pmw3370,
    /// PixArt PAW3950 (Rukh): 50…30 000 шаг 50.
    Paw3950,
}

impl Sensor {
    pub fn name(self) -> &'static str {
        match self {
            Sensor::Pmw3370 => "PMW3370",
            Sensor::Paw3950 => "PAW3950",
        }
    }

    pub fn min(self) -> u16 {
        match self {
            Sensor::Pmw3370 => pmw3370::MIN,
            Sensor::Paw3950 => paw3950::MIN,
        }
    }

    pub fn max(self) -> u16 {
        match self {
            Sensor::Pmw3370 => pmw3370::MAX,
            Sensor::Paw3950 => paw3950::MAX,
        }
    }

    /// Приводит значение к сетке DPI, которую умеет сенсор.
    pub fn snap(self, dpi: u16) -> u16 {
        match self {
            Sensor::Pmw3370 => pmw3370::snap(dpi),
            Sensor::Paw3950 => paw3950::snap(dpi),
        }
    }

    /// Кодирует ступень DPI (одинаковый X/Y) в 4 байта EEPROM.
    pub fn encode_level(self, dpi: u16) -> [u8; 4] {
        match self {
            Sensor::Pmw3370 => pmw3370::encode_level(dpi),
            Sensor::Paw3950 => paw3950::encode_level(dpi),
        }
    }

    /// Декодирует ступень DPI (берётся ось X). `None` — битая контрольная сумма
    /// или значение вне диапазона сенсора.
    pub fn decode_level(self, raw: &[u8]) -> Option<u16> {
        if raw.len() < 4 || raw[3] != checksum(&raw[..3]) {
            return None;
        }
        match self {
            Sensor::Pmw3370 => pmw3370::decode_level(raw),
            Sensor::Paw3950 => paw3950::decode_level(raw),
        }
    }
}

/// PMW3370 — RE `FUN_0040b9c0` (DPI → код) и `FUN_0040bd90` (код → DPI)
/// из `OemDrv.exe`.
///
/// `flags = mul_x | mul_y << 4`, `dpi = base(mul) · (code + 1)`, где
/// `base = 50` (mul 0), `100` (mul 1/2), `200` (mul 3).
pub mod pmw3370 {
    use super::checksum;

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

    pub fn encode_level(dpi: u16) -> [u8; 4] {
        let (code, mul) = encode(dpi);
        let flags = (mul & 0x0F) | (mul << 4);
        [code, code, flags, checksum(&[code, code, flags])]
    }

    /// Контрольная сумма уже проверена в [`super::Sensor::decode_level`].
    pub fn decode_level(raw: &[u8]) -> Option<u16> {
        let dpi = decode(raw[0], raw[2] & 0x03);
        (MIN..=38_000).contains(&dpi).then_some(dpi)
    }
}

/// PAW3950 — из `DPIControl.cs` утилиты Rukh («Mouse Drive Beta» 1.0.0.43,
/// ветка `Sensor.Type == "3950"`, `DPIRange=50,30000,50,0x00`).
///
/// Код 10-битный: младшие 8 бит — в `code_x`/`code_y`, старшие 2 бита — в
/// `flags`: биты 7–6 для X, биты 3–2 для Y. `dpi = 50 · (code + 1)`.
/// Флаги множителя `0x22` (база 100) и `0x11` (×2) эта мышь не использует,
/// но при чтении учитываются так же, как в оригинале.
pub mod paw3950 {
    use super::checksum;

    pub const MIN: u16 = 50;
    pub const MAX: u16 = 30_000;
    pub const STEP: u16 = 50;

    pub fn snap(dpi: u16) -> u16 {
        let dpi = dpi.clamp(MIN, MAX);
        ((dpi + STEP / 2) / STEP * STEP).clamp(MIN, MAX)
    }

    pub fn encode_level(dpi: u16) -> [u8; 4] {
        let code = snap(dpi) / STEP - 1;
        let hi = (code >> 8) as u8 & 0x03;
        let lo = code as u8;
        let flags = (hi << 6) | (hi << 2);
        [lo, lo, flags, checksum(&[lo, lo, flags])]
    }

    /// Контрольная сумма уже проверена в [`super::Sensor::decode_level`].
    pub fn decode_level(raw: &[u8]) -> Option<u16> {
        let flags = raw[2];
        let code = raw[0] as u32 | ((flags as u32 >> 6) << 8);
        let base = if flags & 0x22 != 0 { 100 } else { 50 };
        let mut dpi = (code + 1) * base;
        if flags & 0x11 != 0 {
            dpi *= 2;
        }
        (MIN as u32..=MAX as u32).contains(&dpi).then_some(dpi as u16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_eeprom_levels() {
        // Реальная таблица 0x0C…0x2B с мыши Edge Air Ultra.
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
        let s = Sensor::Pmw3370;
        for (raw, dpi) in live.iter().zip(expect) {
            assert_eq!(s.decode_level(raw), Some(dpi));
            assert_eq!(&s.encode_level(dpi), raw, "encode {dpi}");
        }
    }

    #[test]
    fn snapping() {
        let s = Sensor::Pmw3370;
        assert_eq!(s.snap(0), 50);
        assert_eq!(s.snap(820), 800);
        assert_eq!(s.snap(830), 850);
        assert_eq!(s.snap(10_020), 10_000);
        assert_eq!(s.snap(10_060), 10_100);
        assert_eq!(s.snap(25_000), 19_000);
    }

    #[test]
    fn roundtrip_whole_range() {
        let mut d = pmw3370::MIN;
        while d <= pmw3370::MAX {
            let (c, m) = pmw3370::encode(d);
            assert_eq!(pmw3370::decode(c, m), d);
            d += if d < pmw3370::FINE_LIMIT { 50 } else { 100 };
        }
    }

    #[test]
    fn broken_checksum_rejected() {
        assert_eq!(Sensor::Pmw3370.decode_level(&[0x07, 0x07, 0x00, 0x48]), None);
        assert_eq!(Sensor::Paw3950.decode_level(&[0x07, 0x07, 0x00, 0x48]), None);
    }

    #[test]
    fn paw3950_matches_vendor_formula() {
        let s = Sensor::Paw3950;
        // 400: код 7, старших бит нет.
        assert_eq!(s.encode_level(400), [0x07, 0x07, 0x00, 0x47]);
        // 12 800: код 255 — ещё 8 бит.
        assert_eq!(&s.encode_level(12_800)[..3], &[0xFF, 0xFF, 0x00]);
        // 12 850: код 256 → старшая единица в битах 6 и 2 (0x44).
        assert_eq!(&s.encode_level(12_850)[..3], &[0x00, 0x00, 0x44]);
        // 30 000: код 599 = 0x257 → lo 0x57, hi 2 → 0x88.
        assert_eq!(&s.encode_level(30_000)[..3], &[0x57, 0x57, 0x88]);
    }

    #[test]
    fn paw3950_roundtrip_whole_range() {
        let s = Sensor::Paw3950;
        let mut d = paw3950::MIN;
        while d <= paw3950::MAX {
            assert_eq!(s.decode_level(&s.encode_level(d)), Some(d), "{d}");
            d += paw3950::STEP;
        }
    }

    #[test]
    fn paw3950_snapping() {
        let s = Sensor::Paw3950;
        assert_eq!(s.snap(0), 50);
        assert_eq!(s.snap(820), 800);
        assert_eq!(s.snap(26_030), 26_050);
        assert_eq!(s.snap(40_000), 30_000);
    }

    #[test]
    fn paw3950_multiplier_flags_like_vendor() {
        let s = Sensor::Paw3950;
        let level = |code: u8, flags: u8| [code, code, flags, checksum(&[code, code, flags])];
        // 0x22 — база 100, 0x11 — удвоение (как в DPIControl.UpdateUI).
        assert_eq!(s.decode_level(&level(7, 0x22)), Some(800));
        assert_eq!(s.decode_level(&level(7, 0x11)), Some(800));
        assert_eq!(s.decode_level(&level(7, 0x33)), Some(1600));
    }
}
