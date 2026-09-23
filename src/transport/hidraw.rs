//! Реальный транспорт через `/dev/hidrawN`.
//!
//! У приёмника два HID-интерфейса с одинаковыми VID/PID:
//! * `:1.0` — boot-мышь (только движение/кнопки);
//! * `:1.1` — клавиатура + vendor-коллекции, в т.ч. **feature report 0x08**
//!   (команды) и **input report 0x09** (ответы).
//!
//! Нужный интерфейс определяется по report descriptor: в нём должен быть
//! feature report с ID 0x08. Команды отправляются `ioctl(HIDIOCSFEATURE(17))`
//! (аналог `HidD_SetFeature`), а не `write()` — у интерфейса нет OUT-репортов.

use super::{Error, Result, Transport};
use crate::protocol::packet::{Packet, PACKET_SIZE, REPORT_ID};
use std::ffi::CString;
use std::io;
use std::path::Path;
use std::time::Duration;

pub const VID: u16 = 0x25A7;
/// Проводное подключение.
pub const PID_WIRED: u16 = 0xFA7B;
/// 2.4G-приёмник.
pub const PID_DONGLE: u16 = 0xFA7C;

/// `_IOC(_IOC_READ|_IOC_WRITE, 'H', 0x06, len)` из `<linux/hidraw.h>`.
const fn hidiocsfeature(len: usize) -> libc::c_ulong {
    ((3 << 30) | ((len as u32) << 16) | ((b'H' as u32) << 8) | 0x06) as libc::c_ulong
}

pub struct HidrawTransport {
    fd: libc::c_int,
    path: String,
    pid: u16,
}

/// Найденный, но ещё не открытый интерфейс.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub path: String,
    pub pid: u16,
}

/// Ищет командный интерфейс мыши среди `/sys/class/hidraw/*`.
pub fn find() -> Option<Candidate> {
    let mut entries: Vec<_> = std::fs::read_dir("/sys/class/hidraw").ok()?.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let name = e.file_name().to_string_lossy().into_owned();
        let dev = e.path().join("device");
        let Ok(uevent) = std::fs::read_to_string(dev.join("uevent")) else { continue };
        let Some((vid, pid)) = parse_hid_id(&uevent) else { continue };
        if vid != VID || (pid != PID_DONGLE && pid != PID_WIRED) {
            continue;
        }
        let Ok(desc) = std::fs::read(dev.join("report_descriptor")) else { continue };
        if has_feature_report(&desc, REPORT_ID) {
            return Some(Candidate { path: format!("/dev/{name}"), pid });
        }
    }
    None
}

/// `HID_ID=0003:000025A7:0000FA7C` → (VID, PID).
fn parse_hid_id(uevent: &str) -> Option<(u16, u16)> {
    let line = uevent.lines().find_map(|l| l.strip_prefix("HID_ID="))?;
    let mut it = line.split(':');
    let _bus = it.next()?;
    let vid = u32::from_str_radix(it.next()?, 16).ok()?;
    let pid = u32::from_str_radix(it.next()?, 16).ok()?;
    Some((vid as u16, pid as u16))
}

/// Есть ли в report descriptor FEATURE-элемент (0xB1) под Report ID `id`.
fn has_feature_report(desc: &[u8], id: u8) -> bool {
    let mut i = 0;
    let mut cur_id = 0u8;
    while i < desc.len() {
        let prefix = desc[i];
        if prefix == 0xFE {
            // long item
            let len = *desc.get(i + 1).unwrap_or(&0) as usize;
            i += 3 + len;
            continue;
        }
        let size = match prefix & 0x03 {
            3 => 4,
            n => n as usize,
        };
        let tag = prefix & 0xFC;
        match tag {
            0x84 => cur_id = *desc.get(i + 1).unwrap_or(&0), // Report ID
            0xB0 if cur_id == id => return true,             // Feature
            _ => {}
        }
        i += 1 + size;
    }
    false
}

impl HidrawTransport {
    pub fn open() -> Result<Self> {
        let c = find().ok_or(Error::NotFound)?;
        Self::open_path(&c.path, c.pid)
    }

    pub fn open_path(path: &str, pid: u16) -> Result<Self> {
        let cpath = CString::new(path).map_err(|_| Error::NotFound)?;
        let fd = unsafe { libc::open(cpath.as_ptr(), libc::O_RDWR | libc::O_CLOEXEC | libc::O_NONBLOCK) };
        if fd < 0 {
            let e = io::Error::last_os_error();
            return Err(match e.kind() {
                io::ErrorKind::PermissionDenied => Error::Permission(path.to_string()),
                io::ErrorKind::NotFound => Error::NotFound,
                _ => Error::Io(e),
            });
        }
        Ok(Self { fd, path: path.to_string(), pid })
    }

    pub fn is_dongle(&self) -> bool {
        self.pid == PID_DONGLE
    }
}

impl Transport for HidrawTransport {
    fn send_feature(&mut self, pkt: &Packet) -> Result<()> {
        let mut buf = pkt.0;
        let rc = unsafe { libc::ioctl(self.fd, hidiocsfeature(PACKET_SIZE), buf.as_mut_ptr()) };
        if rc < 0 {
            let e = io::Error::last_os_error();
            // Устройство выдернули — пусть верхний уровень переподключится.
            return Err(match e.raw_os_error() {
                Some(libc::ENODEV) | Some(libc::ENXIO) => Error::NotFound,
                _ => Error::Io(e),
            });
        }
        Ok(())
    }

    fn read_report(&mut self, timeout: Duration) -> Result<Option<Vec<u8>>> {
        let mut pfd = libc::pollfd { fd: self.fd, events: libc::POLLIN, revents: 0 };
        let ms = timeout.as_millis().min(i32::MAX as u128) as libc::c_int;
        let rc = unsafe { libc::poll(&mut pfd, 1, ms) };
        if rc < 0 {
            let e = io::Error::last_os_error();
            if e.kind() == io::ErrorKind::Interrupted {
                return Ok(None);
            }
            return Err(Error::Io(e));
        }
        if rc == 0 {
            return Ok(None);
        }
        if pfd.revents & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL) != 0 {
            return Err(Error::NotFound);
        }
        let mut buf = [0u8; 64];
        let n = unsafe { libc::read(self.fd, buf.as_mut_ptr().cast(), buf.len()) };
        if n < 0 {
            let e = io::Error::last_os_error();
            return match e.kind() {
                io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted => Ok(None),
                _ if e.raw_os_error() == Some(libc::ENODEV) => Err(Error::NotFound),
                _ => Err(Error::Io(e)),
            };
        }
        Ok(Some(buf[..n as usize].to_vec()))
    }

    fn name(&self) -> String {
        let kind = if self.is_dongle() { "2.4G-приёмник" } else { "USB-кабель" };
        let dev = Path::new(&self.path).file_name().map(|s| s.to_string_lossy().into_owned());
        format!("{kind} · {}", dev.unwrap_or_default())
    }
}

impl Drop for HidrawTransport {
    fn drop(&mut self) {
        unsafe { libc::close(self.fd) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Report descriptor интерфейса :1.1 живого приёмника (189 байт).
    const DESC_IF1: &[u8] = &[
        0x05, 0x01, 0x09, 0x06, 0xA1, 0x01, 0x85, 0x01, 0x05, 0x07, 0x19, 0xE0, 0x29, 0xE7, 0x15,
        0x00, 0x25, 0x01, 0x75, 0x01, 0x95, 0x08, 0x81, 0x02, 0x05, 0x07, 0x19, 0x00, 0x2A, 0xFF,
        0x00, 0x95, 0x06, 0x75, 0x08, 0x15, 0x00, 0x26, 0xFF, 0x00, 0x81, 0x00, 0xC0, 0x06, 0x03,
        0xFF, 0x09, 0x00, 0xA1, 0x01, 0x85, 0x02, 0x09, 0x00, 0x15, 0x00, 0x26, 0xFF, 0x00, 0x75,
        0x08, 0x95, 0x07, 0x81, 0x02, 0xC0, 0x05, 0x0C, 0x09, 0x01, 0xA1, 0x01, 0x85, 0x05, 0x15,
        0x00, 0x26, 0x3C, 0x02, 0x19, 0x00, 0x2A, 0x3C, 0x02, 0x75, 0x10, 0x95, 0x01, 0x81, 0x00,
        0xC0, 0x05, 0x01, 0x09, 0x80, 0xA1, 0x01, 0x85, 0x03, 0x19, 0x81, 0x29, 0x83, 0x15, 0x00,
        0x25, 0x01, 0x95, 0x03, 0x75, 0x01, 0x81, 0x02, 0x95, 0x01, 0x75, 0x05, 0x81, 0x01, 0xC0,
        0x06, 0x01, 0xFF, 0x09, 0x00, 0xA1, 0x01, 0x85, 0x09, 0x09, 0x00, 0x95, 0x10, 0x75, 0x08,
        0x15, 0x00, 0x26, 0xFF, 0x00, 0x81, 0x02, 0xC0, 0x06, 0x04, 0xFF, 0x09, 0x02, 0xA1, 0x01,
        0x85, 0x06, 0x09, 0x02, 0x15, 0x00, 0x26, 0xFF, 0x00, 0x75, 0x08, 0x95, 0x07, 0xB1, 0x02,
        0xC0, 0x06, 0x02, 0xFF, 0x09, 0x02, 0xA1, 0x01, 0x85, 0x08, 0x09, 0x02, 0x15, 0x00, 0x26,
        0xFF, 0x00, 0x75, 0x08, 0x95, 0x10, 0xB1, 0x02, 0xC0,
    ];

    #[test]
    fn ioctl_number_matches_kernel_header() {
        // HIDIOCSFEATURE(17) = 0xC0114806
        assert_eq!(hidiocsfeature(17), 0xC011_4806);
    }

    #[test]
    fn detects_command_interface() {
        assert!(has_feature_report(DESC_IF1, 0x08));
        assert!(has_feature_report(DESC_IF1, 0x06));
        assert!(!has_feature_report(DESC_IF1, 0x09)); // 0x09 — input
        // boot-мышь (:1.0) — без feature-репортов
        let mouse = [0x05, 0x01, 0x09, 0x02, 0xA1, 0x01, 0x09, 0x01, 0x81, 0x02, 0xC0];
        assert!(!has_feature_report(&mouse, 0x08));
    }

    #[test]
    fn parses_uevent() {
        let s = "DRIVER=hid-generic\nHID_ID=0003:000025A7:0000FA7C\nHID_NAME=Compx\n";
        assert_eq!(parse_hid_id(s), Some((0x25A7, 0xFA7C)));
        assert_eq!(parse_hid_id("HID_NAME=x"), None);
    }
}
