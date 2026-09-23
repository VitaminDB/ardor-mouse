//! Карта EEPROM профиля мыши и модель настроек.
//!
//! Адреса — из `FUN_0040fd70` (запись профиля) и `FUN_0040e520` (чтение,
//! логи `GetProfile (0x2C~0x4B) (DPI Color)` и т.п.), значения сверены
//! с дампом живой мыши.
//!
//! | Адрес | Размер | Содержимое |
//! |---|---|---|
//! | 0x00 | 2 | частота опроса `[v][0x55−v]`: 1=1000, 2=500, 4=250, 8=125 Гц |
//! | 0x02 | 2 | число ступеней DPI (1…8) |
//! | 0x04 | 2 | активная ступень DPI (с нуля) |
//! | 0x0A | 2 | LOD (1 / 2 мм) |
//! | 0x0C | 32 | 8 ступеней DPI по 4 байта (см. [`dpi`](super::dpi)) |
//! | 0x2C | 32 | 8 цветов индикатора DPI `[R][G][B][0x55−Σ]` |
//! | 0x4C | 8 | эффект индикатора DPI (не редактируется) |
//! | 0x60 | 64 | матрица кнопок (см. [`buttons`](super::buttons)) |
//! | 0xA0 | 7 | подсветка (см. [`led`](super::led)) |
//! | 0xA9 | 2 | debounce, мс |
//! | 0xAD | 2 | время до сна, ×10 с |
//! | 0xAF | 2 | выпрямление линии (angle snapping) |
//! | 0xB1 | 2 | ripple control |
//! | 0xB3 | 2 | гасить подсветку при движении |

use super::buttons::{self, ButtonAction};
use super::dpi;
use super::led::{LedConfig, Rgb};
use super::packet::{checksum, CHECKSUM_BASE};

/// Сколько байт профиля читает оригинальная утилита.
pub const PROFILE_LEN: usize = 0xB5;
/// Ступеней DPI в таблице.
pub const DPI_SLOTS: usize = 8;

pub mod addr {
    pub const POLLING: u16 = 0x00;
    pub const DPI_COUNT: u16 = 0x02;
    pub const DPI_ACTIVE: u16 = 0x04;
    pub const LOD: u16 = 0x0A;
    pub const DPI_TABLE: u16 = 0x0C;
    pub const DPI_COLORS: u16 = 0x2C;
    pub const BUTTONS: u16 = 0x60;
    pub const LED: u16 = 0xA0;
    pub const DEBOUNCE: u16 = 0xA9;
    pub const SLEEP: u16 = 0xAD;
    pub const ANGLE_SNAP: u16 = 0xAF;
    pub const RIPPLE: u16 = 0xB1;
    pub const LED_OFF_MOVING: u16 = 0xB3;
}

/// Частота опроса.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PollingRate {
    Hz125,
    Hz250,
    Hz500,
    Hz1000,
}

impl PollingRate {
    pub const ALL: [PollingRate; 4] =
        [PollingRate::Hz125, PollingRate::Hz250, PollingRate::Hz500, PollingRate::Hz1000];

    pub fn hw(self) -> u8 {
        match self {
            PollingRate::Hz125 => 8,
            PollingRate::Hz250 => 4,
            PollingRate::Hz500 => 2,
            PollingRate::Hz1000 => 1,
        }
    }

    pub fn from_hw(v: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.hw() == v)
    }

    pub fn hz(self) -> u16 {
        match self {
            PollingRate::Hz125 => 125,
            PollingRate::Hz250 => 250,
            PollingRate::Hz500 => 500,
            PollingRate::Hz1000 => 1000,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DpiLevel {
    pub dpi: u16,
    pub color: Rgb,
}

/// Полный редактируемый профиль мыши.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceConfig {
    pub polling: PollingRate,
    /// Сколько ступеней DPI участвует в переключении (1…8).
    pub dpi_count: u8,
    /// Активная ступень (с нуля).
    pub dpi_active: u8,
    pub dpi: [DpiLevel; DPI_SLOTS],
    /// Высота отрыва: 1 или 2 мм.
    pub lod: u8,
    pub buttons: [ButtonAction; buttons::SLOTS],
    pub led: LedConfig,
    pub debounce_ms: u8,
    /// Время до сна в десятках секунд.
    pub sleep_x10s: u8,
    pub angle_snap: bool,
    pub ripple: bool,
    pub led_off_moving: bool,
}

impl Default for DeviceConfig {
    /// Заводские значения (`Cfg.ini [DEV_1]` + дамп новой мыши).
    fn default() -> Self {
        let dpis = [400, 800, 1600, 2400, 3200, 6400, 12_000, 19_000];
        let colors = [
            Rgb::new(255, 0, 0),
            Rgb::new(0, 0, 255),
            Rgb::new(0, 255, 0),
            Rgb::new(255, 255, 0),
            Rgb::new(0, 255, 255),
            Rgb::new(255, 0, 255),
            Rgb::new(255, 255, 255),
            Rgb::new(0, 255, 255),
        ];
        let mut buttons = [ButtonAction::Disabled; buttons::SLOTS];
        buttons[..6].copy_from_slice(&[
            ButtonAction::Left,
            ButtonAction::Right,
            ButtonAction::Middle,
            ButtonAction::Back,
            ButtonAction::Forward,
            ButtonAction::DpiLoop,
        ]);
        Self {
            polling: PollingRate::Hz1000,
            dpi_count: 6,
            dpi_active: 1,
            dpi: std::array::from_fn(|i| DpiLevel { dpi: dpis[i], color: colors[i] }),
            lod: 1,
            buttons,
            led: LedConfig::default(),
            debounce_ms: 4,
            sleep_x10s: 6,
            angle_snap: false,
            ripple: false,
            led_off_moving: false,
        }
    }
}

/// Непрерывный участок EEPROM для записи.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Region {
    pub addr: u16,
    pub data: Vec<u8>,
    pub name: &'static str,
}

/// Что не удалось разобрать при чтении (поле взято из заводских значений).
pub type DecodeWarnings = Vec<&'static str>;

fn pair(v: u8) -> Vec<u8> {
    vec![v, CHECKSUM_BASE.wrapping_sub(v)]
}

fn read_pair(mem: &[u8], a: u16) -> Option<u8> {
    let a = a as usize;
    let (v, c) = (*mem.get(a)?, *mem.get(a + 1)?);
    (c == CHECKSUM_BASE.wrapping_sub(v)).then_some(v)
}

impl DeviceConfig {
    /// Разбирает образ профиля (адрес 0 → `mem[0]`). Битые поля заменяются
    /// заводскими и перечисляются в предупреждениях.
    pub fn decode(mem: &[u8]) -> (Self, DecodeWarnings) {
        let d = Self::default();
        let mut warn = DecodeWarnings::new();
        let mut get = |a: u16, name: &'static str, ok: &dyn Fn(u8) -> bool| -> Option<u8> {
            match read_pair(mem, a) {
                Some(v) if ok(v) => Some(v),
                _ => {
                    warn.push(name);
                    None
                }
            }
        };

        let polling = get(addr::POLLING, "частота опроса", &|v| PollingRate::from_hw(v).is_some())
            .and_then(PollingRate::from_hw)
            .unwrap_or(d.polling);
        let dpi_count = get(addr::DPI_COUNT, "число ступеней DPI", &|v| (1..=8).contains(&v))
            .unwrap_or(d.dpi_count);
        let dpi_active =
            get(addr::DPI_ACTIVE, "активная ступень DPI", &|v| v < 8).unwrap_or(d.dpi_active);
        let lod = get(addr::LOD, "LOD", &|v| (1..=2).contains(&v)).unwrap_or(d.lod);
        let debounce_ms = get(addr::DEBOUNCE, "debounce", &|v| v <= 30).unwrap_or(d.debounce_ms);
        let sleep_x10s = get(addr::SLEEP, "время до сна", &|v| v > 0).unwrap_or(d.sleep_x10s);
        let angle_snap = get(addr::ANGLE_SNAP, "выпрямление линии", &|_| true)
            .map_or(d.angle_snap, |v| v != 0);
        let ripple = get(addr::RIPPLE, "ripple", &|_| true).map_or(d.ripple, |v| v != 0);
        let led_off_moving = get(addr::LED_OFF_MOVING, "гашение подсветки", &|_| true)
            .map_or(d.led_off_moving, |v| v != 0);

        let slice = |a: u16, n: usize| mem.get(a as usize..a as usize + n);

        let mut dpi_levels = d.dpi;
        let mut dpi_bad = false;
        for (i, lvl) in dpi_levels.iter_mut().enumerate() {
            let off = i as u16 * 4;
            match slice(addr::DPI_TABLE + off, 4).and_then(dpi::decode_level) {
                Some(v) => lvl.dpi = v,
                None => dpi_bad |= i < dpi_count as usize,
            }
            match slice(addr::DPI_COLORS + off, 4) {
                Some(c) if c[3] == checksum(&c[..3]) => lvl.color = Rgb::new(c[0], c[1], c[2]),
                _ => dpi_bad |= i < dpi_count as usize,
            }
        }
        if dpi_bad {
            warn.push("таблица DPI");
        }

        let mut buttons = d.buttons;
        for (i, b) in buttons.iter_mut().enumerate() {
            match slice(addr::BUTTONS + i as u16 * 4, 4).and_then(ButtonAction::decode) {
                Some(v) => *b = v,
                None if i < buttons::PHYSICAL.len() => warn.push("назначение кнопок"),
                None => {}
            }
        }

        let led = match slice(addr::LED, 7).and_then(LedConfig::decode) {
            Some(l) => l,
            None => {
                warn.push("подсветка");
                d.led
            }
        };

        warn.dedup();
        let cfg = Self {
            polling,
            dpi_count,
            dpi_active: dpi_active.min(dpi_count - 1),
            dpi: dpi_levels,
            lod,
            buttons,
            led,
            debounce_ms,
            sleep_x10s,
            angle_snap,
            ripple,
            led_off_moving,
        };
        (cfg, warn)
    }

    /// Все записываемые участки профиля.
    pub fn regions(&self) -> Vec<Region> {
        let mut dpi_tab = Vec::with_capacity(32);
        let mut colors = Vec::with_capacity(32);
        for l in &self.dpi {
            dpi_tab.extend_from_slice(&dpi::encode_level(l.dpi));
            let c = [l.color.r, l.color.g, l.color.b];
            colors.extend_from_slice(&c);
            colors.push(checksum(&c));
        }
        let buttons: Vec<u8> = self.buttons.iter().flat_map(|b| b.encode()).collect();
        vec![
            Region { addr: addr::POLLING, data: pair(self.polling.hw()), name: "частота опроса" },
            Region { addr: addr::DPI_COUNT, data: pair(self.dpi_count), name: "число ступеней DPI" },
            Region {
                addr: addr::DPI_ACTIVE,
                data: pair(self.dpi_active.min(self.dpi_count.saturating_sub(1))),
                name: "активная ступень DPI",
            },
            Region { addr: addr::LOD, data: pair(self.lod), name: "LOD" },
            Region { addr: addr::DPI_TABLE, data: dpi_tab, name: "значения DPI" },
            Region { addr: addr::DPI_COLORS, data: colors, name: "цвета DPI" },
            Region { addr: addr::BUTTONS, data: buttons, name: "кнопки" },
            Region { addr: addr::LED, data: self.led.encode().to_vec(), name: "подсветка" },
            Region { addr: addr::DEBOUNCE, data: pair(self.debounce_ms), name: "debounce" },
            Region { addr: addr::SLEEP, data: pair(self.sleep_x10s), name: "время до сна" },
            Region {
                addr: addr::ANGLE_SNAP,
                data: pair(self.angle_snap as u8),
                name: "выпрямление линии",
            },
            Region { addr: addr::RIPPLE, data: pair(self.ripple as u8), name: "ripple" },
            Region {
                addr: addr::LED_OFF_MOVING,
                data: pair(self.led_off_moving as u8),
                name: "гашение подсветки",
            },
        ]
    }

    /// Участки, отличающиеся от `old`, — только их и нужно писать.
    pub fn changed_regions(&self, old: &DeviceConfig) -> Vec<Region> {
        let before = old.regions();
        self.regions()
            .into_iter()
            .zip(before)
            .filter(|(new, old)| new.data != old.data)
            .map(|(new, _)| new)
            .collect()
    }

    /// Проверка перед записью: хотя бы одна физическая кнопка — левый клик
    /// (`tc_msg1` оригинала), иначе мышью станет невозможно пользоваться.
    pub fn validate(&self) -> Result<(), String> {
        let has_left = buttons::PHYSICAL
            .iter()
            .any(|&(_, slot)| self.buttons[slot] == ButtonAction::Left);
        if !has_left {
            return Err("Хотя бы одна кнопка должна быть назначена как «Левый клик»".into());
        }
        if !(1..=DPI_SLOTS as u8).contains(&self.dpi_count) {
            return Err("Нельзя отключить все ступени DPI".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::led::LedMode;

    /// Полный дамп профиля живой мыши (0x00…0xBD, чтение по 10 байт).
    pub const LIVE_DUMP: [u8; 190] = [
        0x01, 0x54, 0x06, 0x4F, 0x01, 0x54, 0x00, 0x55, 0x00, 0x55, 0x01, 0x54, 0x07, 0x07, 0x00,
        0x47, 0x0F, 0x0F, 0x00, 0x37, 0x1F, 0x1F, 0x00, 0x17, 0x2F, 0x2F, 0x00, 0xF7, 0x3F, 0x3F,
        0x00, 0xD7, 0x7F, 0x7F, 0x00, 0x57, 0x77, 0x77, 0x22, 0x45, 0xBD, 0xBD, 0x22, 0xB9, 0xFF,
        0x00, 0x00, 0x56, 0x00, 0x00, 0xFF, 0x56, 0x00, 0xFF, 0x00, 0x56, 0xFF, 0xFF, 0x00, 0x57,
        0x00, 0xFF, 0xFF, 0x57, 0xFF, 0x00, 0xFF, 0x57, 0xFF, 0xFF, 0xFF, 0x58, 0x00, 0xFF, 0xFF,
        0x57, 0x02, 0x53, 0x80, 0xD5, 0x03, 0x52, 0x00, 0x55, 0xFF, 0x00, 0xFF, 0x57, 0x00, 0x55,
        0x80, 0xD5, 0x03, 0x52, 0x00, 0x55, 0x01, 0x01, 0x00, 0x53, 0x01, 0x02, 0x00, 0x52, 0x01,
        0x04, 0x00, 0x50, 0x01, 0x08, 0x00, 0x4C, 0x01, 0x10, 0x00, 0x44, 0x02, 0x01, 0x00, 0x52,
        0x04, 0x0A, 0x03, 0x44, 0x08, 0x00, 0x00, 0x4D, 0x07, 0x00, 0x00, 0x4E, 0x02, 0x02, 0x00,
        0x51, 0x02, 0x03, 0x00, 0x50, 0x00, 0x00, 0x00, 0x55, 0x00, 0x00, 0x00, 0x55, 0x00, 0x00,
        0x00, 0x55, 0x00, 0x00, 0x00, 0x55, 0x00, 0x00, 0x00, 0x55, 0x07, 0xFF, 0x00, 0xFF, 0x96,
        0x96, 0x24, 0x00, 0x55, 0x04, 0x51, 0x00, 0x55, 0x06, 0x4F, 0x00, 0x55, 0x00, 0x55, 0x01,
        0x54, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    ];

    #[test]
    fn decodes_live_profile() {
        let (c, warn) = DeviceConfig::decode(&LIVE_DUMP);
        assert!(warn.is_empty(), "{warn:?}");
        assert_eq!(c.polling, PollingRate::Hz1000);
        assert_eq!(c.dpi_count, 6);
        assert_eq!(c.dpi_active, 1);
        assert_eq!(c.dpi[c.dpi_active as usize].dpi, 800);
        assert_eq!(c.dpi[5].dpi, 6400);
        assert_eq!(c.dpi[0].color, Rgb::new(255, 0, 0));
        assert_eq!(c.dpi[1].color, Rgb::new(0, 0, 255));
        assert_eq!(c.lod, 1);
        assert_eq!(c.buttons[0], ButtonAction::Left);
        assert_eq!(c.buttons[5], ButtonAction::DpiLoop);
        assert_eq!(c.led.mode, LedMode::ColorBreathing);
        assert_eq!(c.debounce_ms, 4);
        assert_eq!(c.sleep_x10s, 6);
        assert!(!c.angle_snap);
        assert!(!c.ripple);
        assert!(c.led_off_moving);
    }

    #[test]
    fn regions_reproduce_live_bytes() {
        let (c, _) = DeviceConfig::decode(&LIVE_DUMP);
        for r in c.regions() {
            let a = r.addr as usize;
            assert_eq!(&LIVE_DUMP[a..a + r.data.len()], &r.data[..], "регион {}", r.name);
        }
    }

    #[test]
    fn diff_only_changed() {
        let (old, _) = DeviceConfig::decode(&LIVE_DUMP);
        assert!(old.changed_regions(&old).is_empty());
        let mut new = old.clone();
        new.dpi[2].dpi = 1750;
        new.polling = PollingRate::Hz500;
        let ch = new.changed_regions(&old);
        let addrs: Vec<u16> = ch.iter().map(|r| r.addr).collect();
        assert_eq!(addrs, vec![addr::POLLING, addr::DPI_TABLE]);
        assert_eq!(ch[0].data, vec![2, 0x53]);
    }

    #[test]
    fn broken_fields_fall_back_to_defaults() {
        let mut mem = LIVE_DUMP;
        mem[1] = 0; // битая пара частоты
        mem[0xA6] ^= 1; // битая подсветка
        let (c, warn) = DeviceConfig::decode(&mem);
        assert_eq!(c.polling, PollingRate::Hz1000);
        assert_eq!(c.led, LedConfig::default());
        assert_eq!(warn, vec!["частота опроса", "подсветка"]);
    }

    #[test]
    fn validate_requires_left_click() {
        let mut c = DeviceConfig::default();
        assert!(c.validate().is_ok());
        c.buttons[0] = ButtonAction::Right;
        assert!(c.validate().is_err());
        c.buttons[4] = ButtonAction::Left;
        assert!(c.validate().is_ok());
    }
}
