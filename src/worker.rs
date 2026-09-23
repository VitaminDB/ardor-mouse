//! Фоновый поток обмена с мышью. UI не блокируется: команды приходят через
//! канал [`Job`], результаты уходят в сигналы (`set()` потокобезопасен).
//! Сигналы здесь только пишутся — читать их из фонового потока нельзя.

use crate::protocol::eeprom::DeviceConfig;
use crate::transport::hidraw::HidrawTransport;
use crate::transport::sim::SimTransport;
use crate::transport::{Battery, Device, Error};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};
use syngui::prelude::*;

/// Период опроса связи.
const POLL: Duration = Duration::from_secs(2);
/// Период опроса батареи.
const BATTERY_POLL: Duration = Duration::from_secs(30);

pub enum Job {
    /// Перечитать профиль с мыши (правки в UI будут заменены).
    Refresh,
    /// Записать отличия `new` от `old`.
    Apply { new: DeviceConfig, old: DeviceConfig },
}

/// Состояние связи для шапки и заглушек.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Link {
    Searching,
    NotFound,
    NoAccess(String),
    /// Приёмник есть, мышь спит или выключена.
    Asleep,
    Ready,
    Error(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeKind {
    Info,
    Success,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice {
    pub kind: NoticeKind,
    pub text: String,
}

impl Notice {
    pub fn new(kind: NoticeKind, text: impl Into<String>) -> Self {
        Self { kind, text: text.into() }
    }
}

/// Сигналы, в которые пишет поток.
#[derive(Clone, Copy)]
pub struct Sink {
    pub link: RwSignal<Link>,
    pub device_name: RwSignal<String>,
    pub battery: RwSignal<Option<Battery>>,
    /// Профиль, который сейчас записан в мыши.
    pub snapshot: RwSignal<Option<DeviceConfig>>,
    /// Профиль, который редактирует пользователь.
    pub edit: RwSignal<DeviceConfig>,
    pub busy: RwSignal<Option<String>>,
    pub notice: RwSignal<Option<Notice>>,
}

impl Sink {
    fn notify(&self, kind: NoticeKind, text: impl Into<String>) {
        self.notice.set(Some(Notice::new(kind, text)));
    }

    /// Новый снимок с мыши. Несохранённые правки сохраняются, если `force == false`.
    fn loaded(&self, cfg: DeviceConfig, force: bool) {
        let sink = *self;
        run_on_main_thread(move || {
            let dirty = sink.snapshot.get_untracked().is_some_and(|s| s != sink.edit.get_untracked());
            if force || !dirty {
                sink.edit.set(cfg.clone());
            }
            sink.snapshot.set(Some(cfg));
        });
    }
}

pub fn spawn(sink: Sink, simulate: bool, rx: Receiver<Job>) {
    std::thread::Builder::new()
        .name("ardor-device".into())
        .spawn(move || Worker { sink, simulate, dev: None, need_read: true, last_battery: None }.run(rx))
        .expect("не удалось запустить поток устройства");
}

struct Worker {
    sink: Sink,
    simulate: bool,
    dev: Option<Device>,
    need_read: bool,
    last_battery: Option<Instant>,
}

impl Worker {
    fn run(mut self, rx: Receiver<Job>) {
        loop {
            if self.dev.is_none() {
                self.try_open();
            }
            if self.dev.is_some() {
                self.poll();
            }
            match rx.recv_timeout(POLL) {
                Ok(job) => self.handle(job),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    fn try_open(&mut self) {
        let opened = if self.simulate {
            let mut sim = SimTransport::new();
            sim.latency = Duration::from_millis(4);
            Ok(Device::new(Box::new(sim)))
        } else {
            HidrawTransport::open().map(|t| Device::new(Box::new(t)))
        };
        match opened {
            Ok(d) => {
                self.sink.device_name.set(d.name());
                self.dev = Some(d);
                self.need_read = true;
                self.last_battery = None;
            }
            Err(Error::NotFound) => self.sink.link.set(Link::NotFound),
            Err(Error::Permission(p)) => self.sink.link.set(Link::NoAccess(p)),
            Err(e) => self.sink.link.set(Link::Error(e.to_string())),
        }
    }

    /// Ошибка обмена: при потере устройства — переподключение.
    fn fail(&mut self, e: &Error) {
        match e {
            Error::NotFound | Error::Io(_) => {
                self.dev = None;
                self.sink.link.set(Link::Searching);
                self.sink.battery.set(None);
            }
            Error::Timeout { .. } => self.sink.link.set(Link::Asleep),
            _ => {}
        }
    }

    fn poll(&mut self) {
        let Some(dev) = self.dev.as_mut() else { return };
        let connected = match dev.is_connected() {
            Ok(c) => c,
            Err(e) => return self.fail(&e),
        };
        if !connected {
            self.sink.link.set(Link::Asleep);
            return;
        }
        if self.last_battery.is_none_or(|t| t.elapsed() >= BATTERY_POLL) {
            if let Ok(b) = dev.battery() {
                self.sink.battery.set(Some(b));
                self.last_battery = Some(Instant::now());
            }
        }
        if self.need_read {
            self.read(false);
        } else {
            self.sink.link.set(Link::Ready);
        }
    }

    fn read(&mut self, force: bool) {
        let Some(dev) = self.dev.as_mut() else { return };
        self.sink.busy.set(Some("Чтение настроек мыши…".into()));
        let res = dev.read_config();
        self.sink.busy.set(None);
        match res {
            Ok((cfg, warn)) => {
                self.need_read = false;
                self.sink.link.set(Link::Ready);
                self.sink.loaded(cfg, force);
                if !warn.is_empty() {
                    self.sink.notify(
                        NoticeKind::Error,
                        format!("Повреждённые поля заменены заводскими: {}", warn.join(", ")),
                    );
                } else if force {
                    self.sink.notify(NoticeKind::Info, "Настройки перечитаны с мыши");
                }
            }
            Err(e) => {
                if force {
                    self.sink.notify(NoticeKind::Error, format!("Не удалось прочитать: {e}"));
                }
                self.fail(&e);
            }
        }
    }

    fn handle(&mut self, job: Job) {
        match job {
            Job::Refresh => {
                if self.dev.is_none() {
                    self.sink.notify(NoticeKind::Error, "Мышь не подключена");
                    return;
                }
                self.read(true);
            }
            Job::Apply { new, old } => self.apply(new, old),
        }
    }

    fn apply(&mut self, new: DeviceConfig, old: DeviceConfig) {
        if let Err(msg) = new.validate() {
            return self.sink.notify(NoticeKind::Error, msg);
        }
        let regions = new.changed_regions(&old);
        if regions.is_empty() {
            return self.sink.notify(NoticeKind::Info, "Изменений нет");
        }
        let Some(dev) = self.dev.as_mut() else {
            return self.sink.notify(NoticeKind::Error, "Мышь не подключена");
        };
        let sink = self.sink;
        let res = dev.write_regions(&regions, |i, n, name| {
            sink.busy.set(Some(format!("Запись: {name} ({}/{n})", i + 1)));
        });
        sink.busy.set(None);
        match res {
            Ok(()) => {
                let names: Vec<&str> = regions.iter().map(|r| r.name).collect();
                sink.snapshot.set(Some(new));
                sink.notify(NoticeKind::Success, format!("Сохранено в мышь: {}", names.join(", ")));
            }
            Err(e) => {
                sink.notify(NoticeKind::Error, format!("Запись не удалась: {e}"));
                // Часть участков могла записаться — перечитать при следующем опросе.
                self.need_read = true;
                self.fail(&e);
            }
        }
    }
}
