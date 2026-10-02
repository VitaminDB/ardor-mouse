//! Симулятор мыши для `--simulate` и тестов: держит образ EEPROM в памяти и
//! отвечает на команды JM03 так же, как живой приёмник (с проверкой
//! контрольных сумм).

use super::{Result, Transport};
use crate::protocol::eeprom::DeviceConfig;
use crate::protocol::model::Model;
use crate::protocol::packet::{cmd, Packet, Response, MAX_DATA};
use std::collections::VecDeque;
use std::time::Duration;

pub struct SimTransport {
    pub model: Model,
    pub mem: [u8; 0x200],
    /// Мышь «спит»: приёмник отвечает, но на чтение/запись EEPROM — тишина.
    pub asleep: bool,
    pub battery: u8,
    pub charging: bool,
    /// Имитация задержки радиоканала.
    pub latency: Duration,
    queue: VecDeque<Vec<u8>>,
}

impl SimTransport {
    /// Симулятор Edge Air Ultra.
    pub fn new() -> Self {
        Self::with_model(Model::EdgeAirUltra)
    }

    /// Симулятор мыши `model` с заводским профилем.
    pub fn with_model(model: Model) -> Self {
        let mut mem = [0xFFu8; 0x200];
        for r in DeviceConfig::factory(model).regions() {
            let a = r.addr as usize;
            mem[a..a + r.data.len()].copy_from_slice(&r.data);
        }
        Self {
            model,
            mem,
            asleep: false,
            battery: 76,
            charging: false,
            latency: Duration::ZERO,
            queue: VecDeque::new(),
        }
    }
}

impl Default for SimTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl Transport for SimTransport {
    fn model(&self) -> Model {
        self.model
    }

    fn send(&mut self, pkt: &Packet) -> Result<()> {
        if !pkt.verify() {
            // Живое устройство молча игнорирует пакеты с битой суммой.
            return Ok(());
        }
        if !self.latency.is_zero() {
            std::thread::sleep(self.latency);
        }
        let addr = pkt.addr() as usize;
        let id = self.model.response_id();
        let reply = match pkt.cmd() {
            cmd::CONNECT_STATUS => Some(Response::build(id, cmd::CONNECT_STATUS, 0, 0, &[!self.asleep as u8])),
            cmd::BATTERY => {
                Some(Response::build(id, cmd::BATTERY, 0, 0, &[self.battery, self.charging as u8]))
            }
            cmd::READ_EEPROM if !self.asleep => {
                let n = pkt.len().min(MAX_DATA);
                let mut data = [0u8; MAX_DATA];
                if let Some(src) = self.mem.get(addr..addr + n) {
                    data[..n].copy_from_slice(src);
                }
                Some(Response::build(id, cmd::READ_EEPROM, 0, pkt.addr(), &data[..n]))
            }
            cmd::WRITE_EEPROM if !self.asleep => {
                let data = pkt.data();
                let status = match self.mem.get_mut(addr..addr + data.len()) {
                    Some(dst) => {
                        dst.copy_from_slice(data);
                        0
                    }
                    None => 1,
                };
                Some(Response::build(id, cmd::WRITE_EEPROM, status, pkt.addr(), &[]))
            }
            _ => None,
        };
        if let Some(r) = reply {
            self.queue.push_back(r.to_vec());
        }
        Ok(())
    }

    fn read_report(&mut self, timeout: Duration) -> Result<Option<Vec<u8>>> {
        match self.queue.pop_front() {
            Some(r) => Ok(Some(r)),
            None => {
                // Имитируем ожидание, но не дольше 50 мс, чтобы тесты были быстрыми.
                std::thread::sleep(timeout.min(Duration::from_millis(50)));
                Ok(None)
            }
        }
    }

    fn name(&self) -> String {
        format!("симулятор {}", self.model.name())
    }

}
