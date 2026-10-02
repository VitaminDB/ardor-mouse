//! Транспорт и команды высокого уровня поверх него.

pub mod hidraw;
pub mod sim;

use crate::protocol::eeprom::Region;
#[cfg(test)]
use crate::protocol::eeprom::{DecodeWarnings, DeviceConfig};
use crate::protocol::model::Model;
use crate::protocol::packet::{cmd, to_hex, Packet, Response, MAX_DATA};
use std::fmt;
use std::time::{Duration, Instant};

/// Сколько ждать ответ (оригинал: 50 × 20 мс).
const RESPONSE_TIMEOUT: Duration = Duration::from_millis(1000);
/// Повторы команды (`DAT_005f4870`).
const RETRIES: usize = 3;
/// Пауза между пакетами записи (оригинал: `Sleep(10)`).
const WRITE_GAP: Duration = Duration::from_millis(10);

#[derive(Debug)]
pub enum Error {
    /// Ни одна поддерживаемая мышь / её приёмник не подключены.
    NotFound,
    /// Нет прав на `/dev/hidrawN` (нужно udev-правило).
    Permission(String),
    Io(std::io::Error),
    /// Устройство не ответило (для беспроводной мыши — обычно спит).
    Timeout { cmd: u8 },
    /// Устройство ответило ошибкой.
    Rejected { cmd: u8, status: u8 },
    /// Режим `--read-only`: команда изменила бы мышь и не отправлена.
    ReadOnly { cmd: u8 },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotFound => write!(f, "приёмник мыши не найден"),
            Error::Permission(p) => write!(f, "нет доступа к {p} — установите udev-правило"),
            Error::Io(e) => write!(f, "ошибка ввода-вывода: {e}"),
            Error::Timeout { cmd } => {
                write!(f, "мышь не ответила на команду 0x{cmd:02X} — пошевелите ей")
            }
            Error::Rejected { cmd, status } => {
                write!(f, "мышь отклонила команду 0x{cmd:02X} (статус 0x{status:02X})")
            }
            Error::ReadOnly { cmd } => {
                write!(f, "режим только чтения: команда 0x{cmd:02X} не отправлена")
            }
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// Низкоуровневый канал: отправка команды (report 0x08) и чтение input reports.
pub trait Transport: Send {
    /// Мышь на том конце — от неё зависят формат обмена и разбор памяти.
    fn model(&self) -> Model;
    fn send(&mut self, pkt: &Packet) -> Result<()>;
    /// Следующий input report или `None` по таймауту.
    fn read_report(&mut self, timeout: Duration) -> Result<Option<Vec<u8>>>;
    fn name(&self) -> String;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Battery {
    pub percent: u8,
    pub charging: bool,
}

/// Мышь: команды JM03 поверх любого [`Transport`].
pub struct Device {
    t: Box<dyn Transport>,
    /// `ARDOR_TRACE=1` — печатать все пакеты в stderr.
    trace: bool,
    /// Отправлять только команды, которые ничего не меняют в мыши.
    read_only: bool,
}

impl Device {
    pub fn new(t: Box<dyn Transport>) -> Self {
        Self { t, trace: std::env::var_os("ARDOR_TRACE").is_some(), read_only: false }
    }

    /// Запретить всё, кроме чтения ([`cmd::is_read_only`]).
    pub fn read_only(mut self, on: bool) -> Self {
        self.read_only = on;
        self
    }

    pub fn name(&self) -> String {
        self.t.name()
    }

    pub fn model(&self) -> Model {
        self.t.model()
    }

    /// Отправить команду и дождаться ответа с тем же кодом (3 попытки).
    pub fn transact(&mut self, pkt: &Packet) -> Result<Response> {
        if self.read_only && !cmd::is_read_only(pkt.cmd()) {
            return Err(Error::ReadOnly { cmd: pkt.cmd() });
        }
        let response_id = self.t.model().response_id();
        let mut last = Error::Timeout { cmd: pkt.cmd() };
        for _ in 0..RETRIES {
            // Сбросить накопившиеся отчёты (клавиатура/мультимедиа того же интерфейса).
            while self.t.read_report(Duration::ZERO)?.is_some() {}
            if self.trace {
                eprintln!("-> {}", pkt.to_hex());
            }
            self.t.send(pkt)?;
            let deadline = Instant::now() + RESPONSE_TIMEOUT;
            loop {
                let left = deadline.saturating_duration_since(Instant::now());
                if left.is_zero() {
                    last = Error::Timeout { cmd: pkt.cmd() };
                    break;
                }
                let Some(raw) = self.t.read_report(left)? else { continue };
                if self.trace {
                    eprintln!("<- {}", to_hex(&raw));
                }
                let Some(r) = Response::parse(&raw, response_id) else { continue };
                if r.cmd != pkt.cmd() {
                    continue;
                }
                if r.status != 0 {
                    last = Error::Rejected { cmd: r.cmd, status: r.status };
                    break;
                }
                return Ok(r);
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Err(last)
    }

    /// Мышь на связи с приёмником (у проводного подключения — всегда).
    pub fn is_connected(&mut self) -> Result<bool> {
        let r = self.transact(&Packet::simple(cmd::CONNECT_STATUS))?;
        Ok(r.payload().first() == Some(&1))
    }

    pub fn battery(&mut self) -> Result<Battery> {
        let r = self.transact(&Packet::simple(cmd::BATTERY))?;
        let d = r.payload();
        Ok(Battery {
            percent: d.first().copied().unwrap_or(0).min(100),
            charging: d.get(1).is_some_and(|&v| v != 0),
        })
    }

    pub fn read_eeprom(&mut self, addr: u16, len: usize) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(len);
        while out.len() < len {
            let a = addr + out.len() as u16;
            let n = (len - out.len()).min(MAX_DATA);
            let r = self.transact(&Packet::read_eeprom(a, n as u8))?;
            out.extend_from_slice(&r.data[..n]);
        }
        Ok(out)
    }

    pub fn write_eeprom(&mut self, addr: u16, data: &[u8]) -> Result<()> {
        for (i, chunk) in data.chunks(MAX_DATA).enumerate() {
            let a = addr + (i * MAX_DATA) as u16;
            self.transact(&Packet::write_eeprom(a, chunk))?;
            std::thread::sleep(WRITE_GAP);
        }
        Ok(())
    }

    /// Сырой образ профиля — столько байт, сколько читает оригинальная утилита.
    pub fn read_profile(&mut self) -> Result<Vec<u8>> {
        let len = self.model().profile_len();
        self.read_eeprom(0, len)
    }

    /// Прочитать и разобрать весь профиль.
    #[cfg(test)]
    pub fn read_config(&mut self) -> Result<(DeviceConfig, DecodeWarnings)> {
        let mem = self.read_profile()?;
        Ok(DeviceConfig::decode(&mem, self.model()))
    }

    /// Записать изменённые участки. `progress(i, n, name)` — перед каждым.
    pub fn write_regions(
        &mut self,
        regions: &[Region],
        mut progress: impl FnMut(usize, usize, &str),
    ) -> Result<()> {
        for (i, r) in regions.iter().enumerate() {
            progress(i, regions.len(), r.name);
            self.write_eeprom(r.addr, &r.data)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::sim::SimTransport;
    use super::*;
    use crate::protocol::eeprom::PollingRate;
    use crate::protocol::led::LedMode;

    fn sim_device() -> Device {
        Device::new(Box::new(SimTransport::new()))
    }

    fn sim_rukh() -> Device {
        Device::new(Box::new(SimTransport::with_model(Model::Rukh)))
    }

    #[test]
    fn status_commands() {
        let mut d = sim_device();
        assert!(d.is_connected().unwrap());
        let b = d.battery().unwrap();
        assert!(b.percent <= 100);
    }

    #[test]
    fn full_read_modify_write_cycle() {
        let mut d = sim_device();
        let (orig, warn) = d.read_config().unwrap();
        assert!(warn.is_empty(), "{warn:?}");

        let mut cfg = orig.clone();
        cfg.polling = PollingRate::Hz500;
        cfg.dpi[3].dpi = 12_300;
        cfg.led.mode = LedMode::Neon;
        cfg.debounce_ms = 8;
        let regions = cfg.changed_regions(&orig);
        assert_eq!(regions.len(), 4);
        let mut seen = Vec::new();
        d.write_regions(&regions, |_, _, n| seen.push(n.to_string())).unwrap();
        assert_eq!(seen.len(), 4);

        let (back, warn) = d.read_config().unwrap();
        assert!(warn.is_empty(), "{warn:?}");
        assert_eq!(back, cfg);
    }

    #[test]
    fn rukh_read_modify_write_cycle() {
        let mut d = sim_rukh();
        let (orig, warn) = d.read_config().unwrap();
        assert!(warn.is_empty(), "{warn:?}");
        assert_eq!(orig.model, Model::Rukh);

        let mut cfg = orig.clone();
        cfg.dpi[1].dpi = 26_000;
        cfg.debounce_ms = 2;
        d.write_regions(&cfg.changed_regions(&orig), |_, _, _| {}).unwrap();
        let (back, _) = d.read_config().unwrap();
        assert_eq!(back, cfg);
    }

    #[test]
    fn read_only_blocks_writes_before_sending() {
        let mut d = sim_rukh().read_only(true);
        assert!(d.read_config().is_ok());
        assert!(d.battery().is_ok());
        assert!(matches!(d.write_eeprom(0x0C, &[1, 2]), Err(Error::ReadOnly { cmd: 0x07 })));
        assert!(matches!(
            d.transact(&Packet::simple(cmd::FACTORY_RESET)),
            Err(Error::ReadOnly { cmd: 0x09 })
        ));
        // в память симулятора ничего не попало
        let (cfg, _) = d.read_config().unwrap();
        assert_eq!(cfg, DeviceConfig::factory(Model::Rukh));
    }

    #[test]
    fn asleep_mouse_times_out() {
        let mut sim = SimTransport::new();
        sim.asleep = true;
        let mut d = Device::new(Box::new(sim));
        assert!(!d.is_connected().unwrap());
        assert!(matches!(d.read_eeprom(0, 2), Err(Error::Timeout { cmd: 0x08 })));
    }
}
