//! Назначение кнопок — матрица EEPROM 0x60…0x9F (16 слотов × 4 байта),
//! RE `FUN_0044ed10` (запись), `FUN_0040f270` (код Cfg.ini → hw).
//!
//! Слот: `[type][b1][b2][0x55 − Σ]`, где `type | b1 << 8 | b2 << 16` — это
//! «hw value» из таблицы `FUN_0040f270` (например, 0x101 → `01 01 00`).

use super::packet::checksum;

/// Слотов в матрице кнопок.
pub const SLOTS: usize = 16;

/// Физические кнопки мыши: подпись и индекс слота в матрице.
/// Номера совпадают с цифрами на картинке `skins/0806/mouse_nr.png`
/// (`Cfg.ini`: `K4_1=…,0x05` — кнопка 4 живёт в слоте 5, `K5_1=…,0x04` — в слоте 4).
pub const PHYSICAL: [(&str, usize); 6] = [
    ("Левая кнопка", 0),
    ("Правая кнопка", 1),
    ("Колесо (нажатие)", 2),
    ("Боковая передняя", 4),
    ("Боковая задняя", 3),
    ("Кнопка DPI", 5),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonAction {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    DpiLoop,
    DpiUp,
    DpiDown,
    Disabled,
    /// Функция, которую приложение не умеет редактировать (макрос,
    /// «огонь», мультимедиа…) — хранится как есть и не портится при записи.
    Other([u8; 3]),
}

impl ButtonAction {
    /// Варианты, доступные в выпадающем списке.
    pub const CHOICES: [ButtonAction; 9] = [
        ButtonAction::Left,
        ButtonAction::Right,
        ButtonAction::Middle,
        ButtonAction::Back,
        ButtonAction::Forward,
        ButtonAction::DpiLoop,
        ButtonAction::DpiUp,
        ButtonAction::DpiDown,
        ButtonAction::Disabled,
    ];

    pub fn bytes(self) -> [u8; 3] {
        match self {
            ButtonAction::Left => [0x01, 0x01, 0x00],
            ButtonAction::Right => [0x01, 0x02, 0x00],
            ButtonAction::Middle => [0x01, 0x04, 0x00],
            ButtonAction::Back => [0x01, 0x08, 0x00],
            ButtonAction::Forward => [0x01, 0x10, 0x00],
            ButtonAction::DpiLoop => [0x02, 0x01, 0x00],
            ButtonAction::DpiUp => [0x02, 0x02, 0x00],
            ButtonAction::DpiDown => [0x02, 0x03, 0x00],
            ButtonAction::Disabled => [0x00, 0x00, 0x00],
            ButtonAction::Other(b) => b,
        }
    }

    pub fn from_bytes(b: [u8; 3]) -> Self {
        Self::CHOICES
            .into_iter()
            .find(|a| a.bytes() == b)
            .unwrap_or(ButtonAction::Other(b))
    }

    pub fn encode(self) -> [u8; 4] {
        let [a, b, c] = self.bytes();
        [a, b, c, checksum(&[a, b, c])]
    }

    pub fn decode(raw: &[u8]) -> Option<Self> {
        if raw.len() < 4 || raw[3] != checksum(&raw[..3]) {
            return None;
        }
        Some(Self::from_bytes([raw[0], raw[1], raw[2]]))
    }

    pub fn label(self) -> String {
        match self {
            ButtonAction::Left => "Левый клик".into(),
            ButtonAction::Right => "Правый клик".into(),
            ButtonAction::Middle => "Средний клик".into(),
            ButtonAction::Back => "Назад".into(),
            ButtonAction::Forward => "Вперёд".into(),
            ButtonAction::DpiLoop => "Цикл DPI".into(),
            ButtonAction::DpiUp => "DPI +".into(),
            ButtonAction::DpiDown => "DPI −".into(),
            ButtonAction::Disabled => "Отключена".into(),
            ButtonAction::Other([t, a, b]) => {
                let kind = match t {
                    0x04 => "Кнопка «Огонь»",
                    0x05 => "Клавиша клавиатуры",
                    0x06 => "Макрос",
                    _ => "Спецфункция",
                };
                format!("{kind} ({t:02X} {a:02X} {b:02X})")
            }
        }
    }

    /// Строковый id для `Dropdown`.
    pub fn id(self) -> String {
        let [a, b, c] = self.bytes();
        format!("{a:02x}{b:02x}{c:02x}")
    }

    pub fn from_id(id: &str) -> Option<Self> {
        let v = u32::from_str_radix(id, 16).ok()?;
        Some(Self::from_bytes([(v >> 16) as u8, (v >> 8) as u8, v as u8]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Реальная матрица 0x60…0x8B с мыши (первые 11 слотов).
    const LIVE: [[u8; 4]; 11] = [
        [0x01, 0x01, 0x00, 0x53],
        [0x01, 0x02, 0x00, 0x52],
        [0x01, 0x04, 0x00, 0x50],
        [0x01, 0x08, 0x00, 0x4C],
        [0x01, 0x10, 0x00, 0x44],
        [0x02, 0x01, 0x00, 0x52],
        [0x04, 0x0A, 0x03, 0x44],
        [0x08, 0x00, 0x00, 0x4D],
        [0x07, 0x00, 0x00, 0x4E],
        [0x02, 0x02, 0x00, 0x51],
        [0x02, 0x03, 0x00, 0x50],
    ];

    #[test]
    fn decodes_live_matrix() {
        let expect = [
            ButtonAction::Left,
            ButtonAction::Right,
            ButtonAction::Middle,
            ButtonAction::Back,
            ButtonAction::Forward,
            ButtonAction::DpiLoop,
            ButtonAction::Other([0x04, 0x0A, 0x03]),
            ButtonAction::Other([0x08, 0x00, 0x00]),
            ButtonAction::Other([0x07, 0x00, 0x00]),
            ButtonAction::DpiUp,
            ButtonAction::DpiDown,
        ];
        for (raw, act) in LIVE.iter().zip(expect) {
            assert_eq!(ButtonAction::decode(raw), Some(act));
            assert_eq!(&act.encode(), raw);
        }
    }

    #[test]
    fn matches_cfg_hw_table() {
        // FUN_0040f270: 0x11→0x101, 0x13→0x201, 0x12→0x401, 0x15→0x801,
        // 0x14→0x1001, 0xAA→0x102, 0xA8→0x202, 0xA9→0x302, 0xA2→0.
        let hw = |a: ButtonAction| {
            let [t, b1, b2] = a.bytes();
            t as u32 | (b1 as u32) << 8 | (b2 as u32) << 16
        };
        assert_eq!(hw(ButtonAction::Left), 0x101);
        assert_eq!(hw(ButtonAction::Right), 0x201);
        assert_eq!(hw(ButtonAction::Middle), 0x401);
        assert_eq!(hw(ButtonAction::Back), 0x801);
        assert_eq!(hw(ButtonAction::Forward), 0x1001);
        assert_eq!(hw(ButtonAction::DpiLoop), 0x102);
        assert_eq!(hw(ButtonAction::DpiUp), 0x202);
        assert_eq!(hw(ButtonAction::DpiDown), 0x302);
        assert_eq!(hw(ButtonAction::Disabled), 0);
    }

    #[test]
    fn id_roundtrip() {
        for a in ButtonAction::CHOICES {
            assert_eq!(ButtonAction::from_id(&a.id()), Some(a));
        }
        let other = ButtonAction::Other([0x06, 0x02, 0x00]);
        assert_eq!(ButtonAction::from_id(&other.id()), Some(other));
    }
}
