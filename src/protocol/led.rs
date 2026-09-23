//! Подсветка корпуса — EEPROM 0xA0…0xA6, RE `FUN_0040fd70` («Set LED»)
//! и `FUN_0040eee0` (`EEPROM_To_MainRGB`).
//!
//! ```text
//! 0xA0: [hw_mode][R][G][B][speed][brightness][0x55 − Σ]
//! ```
//! `speed` / `brightness` — не «уровни», а значения из таблиц прошивки
//! (`DAT_005da583` / `DAT_005da58f`; для «Потока» — `DAT_005da577` / `DAT_005da56b`).

use super::packet::checksum;

/// Уровней скорости / яркости в интерфейсе (1…10).
pub const LEVELS: u8 = 10;

/// Скорость (для всех режимов, кроме «Потока»), индекс 0 = уровень 1.
const SPEED_STD: [u8; 10] = [0xFF, 0xE6, 0xD2, 0xBE, 0xAA, 0x96, 0x82, 0x6E, 0x46, 0x28];
/// Яркость (для всех режимов, кроме «Потока»).
const BRI_STD: [u8; 10] = [0x2D, 0x32, 0x4B, 0x64, 0x7D, 0x96, 0xAF, 0xC8, 0xE1, 0xFF];
/// Скорость «Потока».
const SPEED_STREAM: [u8; 10] = [0x0A, 0x09, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01];
/// Яркость «Потока».
const BRI_STREAM: [u8; 10] = [0x48, 0x5C, 0x70, 0x84, 0x98, 0xAC, 0xC0, 0xD4, 0xE8, 0xFC];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn to_hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}

/// Режимы подсветки в порядке оригинального интерфейса (`tc_led_mode1..7`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LedMode {
    Steady,
    Breathing,
    Streaming,
    Neon,
    Scrolling,
    ColorBreathing,
    Off,
}

impl LedMode {
    pub const ALL: [LedMode; 7] = [
        LedMode::Steady,
        LedMode::Breathing,
        LedMode::Streaming,
        LedMode::Neon,
        LedMode::Scrolling,
        LedMode::ColorBreathing,
        LedMode::Off,
    ];

    /// Код режима в прошивке (UI-индекс → hw в `FUN_0040fd70`).
    pub fn hw(self) -> u8 {
        match self {
            LedMode::Steady => 2,
            LedMode::Breathing => 1,
            LedMode::Streaming => 0,
            LedMode::Neon => 3,
            LedMode::Scrolling => 5,
            LedMode::ColorBreathing => 7,
            LedMode::Off => 4,
        }
    }

    pub fn from_hw(v: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.hw() == v)
    }

    pub fn label(self) -> &'static str {
        match self {
            LedMode::Steady => "Статичная",
            LedMode::Breathing => "Дыхание",
            LedMode::Streaming => "Поток",
            LedMode::Neon => "Неон",
            LedMode::Scrolling => "Бег",
            LedMode::ColorBreathing => "Цветное дыхание",
            LedMode::Off => "Выключена",
        }
    }

    /// Режим использует выбранный цвет (`LedOptN`, 5-е поле).
    pub fn has_color(self) -> bool {
        matches!(self, LedMode::Steady | LedMode::Breathing | LedMode::Scrolling)
    }

    pub fn has_speed(self) -> bool {
        !matches!(self, LedMode::Steady | LedMode::Off)
    }

    fn tables(self) -> (&'static [u8; 10], &'static [u8; 10]) {
        if self == LedMode::Streaming {
            (&SPEED_STREAM, &BRI_STREAM)
        } else {
            (&SPEED_STD, &BRI_STD)
        }
    }
}

/// Настройки подсветки. `speed` и `brightness` — уровни 1…10.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LedConfig {
    pub mode: LedMode,
    pub color: Rgb,
    pub speed: u8,
    pub brightness: u8,
}

impl Default for LedConfig {
    fn default() -> Self {
        Self { mode: LedMode::ColorBreathing, color: Rgb::new(255, 0, 255), speed: 6, brightness: 6 }
    }
}

impl LedConfig {
    /// 7 байт блока 0xA0.
    pub fn encode(&self) -> [u8; 7] {
        let (speeds, bris) = self.mode.tables();
        let s = speeds[level_index(self.speed)];
        let b = bris[level_index(self.brightness)];
        let mut out = [self.mode.hw(), self.color.r, self.color.g, self.color.b, s, b, 0];
        out[6] = checksum(&out[..6]);
        out
    }

    /// Разбор блока 0xA0. Неизвестные значения таблиц → средний уровень
    /// (как «Unkown Speed / Unkown Bri» у оригинала).
    pub fn decode(raw: &[u8]) -> Option<Self> {
        if raw.len() < 7 || raw[6] != checksum(&raw[..6]) {
            return None;
        }
        let mode = LedMode::from_hw(raw[0])?;
        // У выключенной подсветки параметры остаются от прошлого режима.
        let (speeds, bris) = mode.tables();
        let find = |t: &[u8; 10], v: u8| t.iter().position(|&x| x == v).map(|i| i as u8 + 1);
        Some(Self {
            mode,
            color: Rgb::new(raw[1], raw[2], raw[3]),
            speed: find(speeds, raw[4]).unwrap_or(6),
            brightness: find(bris, raw[5]).unwrap_or(6),
        })
    }
}

fn level_index(level: u8) -> usize {
    (level.clamp(1, LEVELS) - 1) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Реальный блок 0xA0 с мыши: «Цветное дыхание», #FF00FF, скорость 6, яркость 6.
    const LIVE: [u8; 7] = [0x07, 0xFF, 0x00, 0xFF, 0x96, 0x96, 0x24];

    #[test]
    fn decodes_live_block() {
        let c = LedConfig::decode(&LIVE).unwrap();
        assert_eq!(c.mode, LedMode::ColorBreathing);
        assert_eq!(c.color, Rgb::new(255, 0, 255));
        assert_eq!(c.speed, 6);
        assert_eq!(c.brightness, 6);
        assert_eq!(c.encode(), LIVE);
    }

    #[test]
    fn hw_codes_are_unique_and_roundtrip() {
        for m in LedMode::ALL {
            assert_eq!(LedMode::from_hw(m.hw()), Some(m));
        }
    }

    #[test]
    fn streaming_uses_own_tables() {
        let c = LedConfig { mode: LedMode::Streaming, color: Rgb::default(), speed: 10, brightness: 1 };
        let raw = c.encode();
        assert_eq!(raw[0], 0);
        assert_eq!(raw[4], 0x01);
        assert_eq!(raw[5], 0x48);
        assert_eq!(LedConfig::decode(&raw), Some(c));
    }

    #[test]
    fn levels_are_clamped() {
        let c = LedConfig { mode: LedMode::Breathing, color: Rgb::default(), speed: 0, brightness: 99 };
        let raw = c.encode();
        assert_eq!(raw[4], SPEED_STD[0]);
        assert_eq!(raw[5], BRI_STD[9]);
    }

    #[test]
    fn bad_checksum_rejected() {
        let mut raw = LIVE;
        raw[6] ^= 0x10;
        assert!(LedConfig::decode(&raw).is_none());
    }
}
