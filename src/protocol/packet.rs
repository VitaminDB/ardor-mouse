//! Пакеты JM03 (Compx) — формат из RE `FUN_0044f0c0` / `FUN_0044e490` /
//! `FUN_0044e7f0`, подтверждён на живом донгле 25a7:fa7c.
//!
//! Запрос — HID-репорт **0x08** (Usage Page 0xFF02, 16 байт данных): у Edge Air
//! Ultra это feature report, у Rukh — output report (см. [`Model`]). Итого 17 байт
//! вместе с Report ID:
//!
//! ```text
//! [0]  0x08        Report ID (а не «маркер пакета»!)
//! [1]  cmd         код команды (0x07 запись EEPROM, 0x08 чтение, 0x04 батарея…)
//! [2]  0x00
//! [3]  addr_hi     адрес EEPROM (для прочих команд 0)
//! [4]  addr_lo
//! [5]  len         длина данных (≤ 10)
//! [6..16] data
//! [16] checksum    0x55 − Σ[0..16]
//! ```
//!
//! Ответ — input report того же формата: у Edge Air Ultra с ID **0x09**
//! (Usage Page 0xFF01), у Rukh — с тем же ID **0x08**:
//! `[id][cmd][status][addr_hi][addr_lo][len][data×10][checksum]`,
//! `status == 0` — успех.
//!
//! [`Model`]: super::model::Model

/// Report ID запроса.
pub const REPORT_ID: u8 = 0x08;
/// Report ID ответа у Edge Air Ultra (у Rukh ответ приходит с [`REPORT_ID`]).
pub const RESPONSE_ID: u8 = 0x09;
/// Размер пакета с Report ID (`HidD_SetFeature(..., 0x11)`).
pub const PACKET_SIZE: usize = 17;
/// Инициализатор всех контрольных сумм JM03 ('U').
pub const CHECKSUM_BASE: u8 = 0x55;
/// Максимум полезных байт в одном пакете.
pub const MAX_DATA: usize = 10;

/// Коды команд (байт `[1]`).
#[allow(dead_code)] // часть известна из RE, но приложением не используется
pub mod cmd {
    /// «Драйвер активен» (1/0) — официальная утилита шлёт при старте/выходе.
    pub const DRIVER_STATUS: u8 = 0x02;
    /// Связь донгла с мышью: data[0] == 1 — мышь на связи.
    pub const CONNECT_STATUS: u8 = 0x03;
    /// Заряд: data[0] — проценты, data[1] — идёт зарядка.
    pub const BATTERY: u8 = 0x04;
    /// Запись EEPROM (≤ 10 байт за пакет).
    pub const WRITE_EEPROM: u8 = 0x07;
    /// Чтение EEPROM (≤ 10 байт за пакет).
    pub const READ_EEPROM: u8 = 0x08;
    /// Сброс к заводским настройкам (кнопка «Reset» оригинала).
    pub const FACTORY_RESET: u8 = 0x09;
    /// Событие от мыши без запроса: data[0] — битовая маска изменений
    /// (b0 DPI, b1 частота, b2 профиль, b3 индикатор DPI, b5 подсветка,
    /// b6 батарея) — `CS_GetDeviceStatusChanged` в `HIDUsb.dll` Rukh.
    pub const STATUS_CHANGED: u8 = 0x0A;
    /// Номер текущего профиля.
    pub const CURRENT_PROFILE: u8 = 0x0E;

    /// Команды, которые ничего не меняют в мыши, — только они разрешены
    /// в режиме `--read-only`.
    pub fn is_read_only(cmd: u8) -> bool {
        matches!(cmd, CONNECT_STATUS | BATTERY | READ_EEPROM | CURRENT_PROFILE)
    }
}

/// Контрольная сумма JM03: `0x55 − Σ bytes` (mod 256).
pub fn checksum(bytes: &[u8]) -> u8 {
    bytes
        .iter()
        .fold(CHECKSUM_BASE, |acc, &b| acc.wrapping_sub(b))
}

/// Собранный 17-байтный запрос.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Packet(pub [u8; PACKET_SIZE]);

impl Packet {
    /// Общий вид команды: `cmd`, адрес и до 10 байт данных.
    pub fn new(cmd: u8, addr: u16, data: &[u8]) -> Self {
        assert!(data.len() <= MAX_DATA, "JM03: больше {MAX_DATA} байт в пакете");
        let mut buf = [0u8; PACKET_SIZE];
        buf[0] = REPORT_ID;
        buf[1] = cmd;
        buf[3] = (addr >> 8) as u8;
        buf[4] = addr as u8;
        buf[5] = data.len() as u8;
        buf[6..6 + data.len()].copy_from_slice(data);
        buf[PACKET_SIZE - 1] = checksum(&buf[..PACKET_SIZE - 1]);
        Packet(buf)
    }

    /// Команда без адреса и данных (статус, батарея…).
    pub fn simple(cmd: u8) -> Self {
        Self::new(cmd, 0, &[])
    }

    /// Чтение `len` байт EEPROM с адреса `addr`.
    pub fn read_eeprom(addr: u16, len: u8) -> Self {
        assert!(len as usize <= MAX_DATA);
        let mut p = Self::new(cmd::READ_EEPROM, addr, &[]);
        p.0[5] = len;
        p.0[PACKET_SIZE - 1] = checksum(&p.0[..PACKET_SIZE - 1]);
        p
    }

    /// Запись данных в EEPROM с адреса `addr`.
    pub fn write_eeprom(addr: u16, data: &[u8]) -> Self {
        Self::new(cmd::WRITE_EEPROM, addr, data)
    }

    pub fn cmd(&self) -> u8 {
        self.0[1]
    }

    pub fn addr(&self) -> u16 {
        u16::from_be_bytes([self.0[3], self.0[4]])
    }

    pub fn len(&self) -> usize {
        self.0[5] as usize
    }

    pub fn data(&self) -> &[u8] {
        &self.0[6..6 + self.len().min(MAX_DATA)]
    }

    pub fn verify(&self) -> bool {
        self.0[0] == REPORT_ID && self.0[PACKET_SIZE - 1] == checksum(&self.0[..PACKET_SIZE - 1])
    }

    pub fn to_hex(&self) -> String {
        to_hex(&self.0)
    }
}

/// Разобранный ответ устройства.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Response {
    pub cmd: u8,
    pub status: u8,
    pub addr: u16,
    pub len: u8,
    pub data: [u8; MAX_DATA],
}

impl Response {
    /// Разбирает сырой input report с Report ID ответа `id`. `None` — не ответ
    /// JM03 (например, отчёт клавиатуры/мультимедиа с того же интерфейса) или
    /// битая контрольная сумма.
    pub fn parse(report: &[u8], id: u8) -> Option<Self> {
        if report.len() < PACKET_SIZE || report[0] != id {
            return None;
        }
        if report[PACKET_SIZE - 1] != checksum(&report[..PACKET_SIZE - 1]) {
            return None;
        }
        let mut data = [0u8; MAX_DATA];
        data.copy_from_slice(&report[6..16]);
        Some(Self {
            cmd: report[1],
            status: report[2],
            addr: u16::from_be_bytes([report[3], report[4]]),
            len: report[5],
            data,
        })
    }

    /// Собирает ответ с Report ID `id` (для симулятора и тестов).
    pub fn build(id: u8, cmd: u8, status: u8, addr: u16, data: &[u8]) -> [u8; PACKET_SIZE] {
        let mut buf = [0u8; PACKET_SIZE];
        buf[0] = id;
        buf[1] = cmd;
        buf[2] = status;
        buf[3] = (addr >> 8) as u8;
        buf[4] = addr as u8;
        buf[5] = data.len() as u8;
        buf[6..6 + data.len()].copy_from_slice(data);
        buf[PACKET_SIZE - 1] = checksum(&buf[..PACKET_SIZE - 1]);
        buf
    }

    pub fn payload(&self) -> &[u8] {
        &self.data[..(self.len as usize).min(MAX_DATA)]
    }
}

pub fn to_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Живые ответы донгла 25a7:fa7c (сняты через hidraw).
    const LIVE_BATTERY: [u8; 17] = [
        0x09, 0x04, 0x00, 0x00, 0x00, 0x02, 0x28, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x1E,
    ];
    const LIVE_READ0: [u8; 17] = [
        0x09, 0x08, 0x00, 0x00, 0x00, 0x0A, 0x01, 0x54, 0x06, 0x4F, 0x01, 0x54, 0x00, 0x55, 0x00,
        0x55, 0x91,
    ];

    #[test]
    fn report_id_and_size() {
        let p = Packet::simple(cmd::BATTERY);
        assert_eq!(p.0.len(), 17);
        assert_eq!(p.0[0], REPORT_ID);
        assert_eq!(p.cmd(), cmd::BATTERY);
        assert!(p.verify());
    }

    #[test]
    fn checksum_matches_re_formula() {
        // FUN_0044f0c0: 'U' − Σ(чётные) − Σ(нечётные) по buf[0..16].
        let p = Packet::write_eeprom(0x00A0, &[7, 0xFF, 0, 0xFF, 0x96, 0x96, 0x24]);
        let mut odd = 0u8;
        let mut even = 0u8;
        for i in (0..16).step_by(2) {
            odd = odd.wrapping_add(p.0[i]);
            even = even.wrapping_add(p.0[i + 1]);
        }
        assert_eq!(p.0[16], b'U'.wrapping_sub(even).wrapping_sub(odd));
    }

    #[test]
    fn read_request_layout() {
        // ReadEEPROM: [08][08][00][hi][lo][len]
        let p = Packet::read_eeprom(0x0123, 10);
        assert_eq!(&p.0[..6], &[0x08, 0x08, 0x00, 0x01, 0x23, 0x0A]);
        assert_eq!(p.addr(), 0x0123);
        assert!(p.verify());
    }

    #[test]
    fn write_request_layout() {
        // WriteEEPROM: [08][07][00][hi][lo][len][data]
        let p = Packet::write_eeprom(0x0004, &[1, 0x54]);
        assert_eq!(&p.0[..8], &[0x08, 0x07, 0x00, 0x00, 0x04, 0x02, 0x01, 0x54]);
        assert_eq!(p.data(), &[1, 0x54]);
    }

    #[test]
    fn parses_live_responses() {
        let r = Response::parse(&LIVE_BATTERY, RESPONSE_ID).unwrap();
        assert_eq!(r.cmd, cmd::BATTERY);
        assert_eq!(r.status, 0);
        assert_eq!(r.payload(), &[40, 0]);

        let r = Response::parse(&LIVE_READ0, RESPONSE_ID).unwrap();
        assert_eq!(r.cmd, cmd::READ_EEPROM);
        assert_eq!(r.payload()[..6], [0x01, 0x54, 0x06, 0x4F, 0x01, 0x54]);
    }

    #[test]
    fn rejects_foreign_or_broken_reports() {
        let mut bad = LIVE_BATTERY;
        bad[16] ^= 1;
        assert!(Response::parse(&bad, RESPONSE_ID).is_none());
        // отчёт boot-клавиатуры (ID 1) с того же интерфейса
        assert!(Response::parse(&[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], RESPONSE_ID).is_none());
    }

    #[test]
    fn rukh_status_changed_event() {
        // Нажатие кнопки DPI на Rukh (hid-recorder, 3554:f53e): input 0x08,
        // cmd 0x0A, data[0] = 0x01 — «сменилась ступень DPI».
        const DPI_PRESSED: [u8; 17] = [
            0x08, 0x0A, 0x00, 0x00, 0x00, 0x0A, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x38,
        ];
        let r = Response::parse(&DPI_PRESSED, REPORT_ID).unwrap();
        assert_eq!(r.cmd, cmd::STATUS_CHANGED);
        assert_eq!(r.payload()[0], 0x01);
        // как ответ Edge (ID 0x09) этот отчёт не принимается
        assert!(Response::parse(&DPI_PRESSED, RESPONSE_ID).is_none());
    }

    #[test]
    fn read_only_commands() {
        assert!(cmd::is_read_only(cmd::READ_EEPROM));
        assert!(cmd::is_read_only(cmd::BATTERY));
        assert!(!cmd::is_read_only(cmd::WRITE_EEPROM));
        assert!(!cmd::is_read_only(cmd::FACTORY_RESET));
        assert!(!cmd::is_read_only(cmd::DRIVER_STATUS));
    }

    #[test]
    fn build_roundtrip() {
        let raw = Response::build(RESPONSE_ID, cmd::BATTERY, 0, 0, &[40, 0]);
        assert_eq!(raw, LIVE_BATTERY);
    }
}
