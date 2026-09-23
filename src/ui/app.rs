//! Контекст приложения и оболочка окна: шапка, навигация, страница, панель действий.

use super::{icons, pages, widgets};
use crate::protocol::eeprom::DeviceConfig;
use crate::transport::Battery;
use crate::worker::{self, Job, Link, NoticeKind, Sink};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use syngui::prelude::*;

const HERO_PNG: &[u8] = include_bytes!("../../assets/skins/0806/mouse_led.png");

/// Разделы навигации.
pub const PAGES: [(&str, &str); 4] = [
    ("DPI", icons::SPEED),
    ("Подсветка", icons::LIGHT),
    ("Кнопки", icons::BUTTONS),
    ("Сенсор", icons::TUNE),
];

#[derive(Clone)]
pub struct AppCtx {
    pub sink: Sink,
    pub page: RwSignal<usize>,
    /// Ступень DPI, открытая в редакторе.
    pub sel_level: RwSignal<usize>,
    jobs: Sender<Job>,
    pending: Arc<Mutex<Option<Receiver<Job>>>>,
    simulate: bool,
}

impl AppCtx {
    /// Создаёт сигналы. Вызывать один раз, до `App::run`.
    pub fn new(simulate: bool) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            sink: Sink {
                link: use_signal(Link::Searching),
                device_name: use_signal(String::new()),
                battery: use_signal(None::<Battery>),
                snapshot: use_signal(None::<DeviceConfig>),
                edit: use_signal(DeviceConfig::default()),
                busy: use_signal(None::<String>),
                notice: use_signal(None),
            },
            page: use_signal(0usize),
            sel_level: use_signal(1usize),
            jobs: tx,
            pending: Arc::new(Mutex::new(Some(rx))),
            simulate,
        }
    }

    /// Запускает поток устройства (после инициализации рантайма окна).
    pub fn start_worker(&self) {
        if let Some(rx) = self.pending.lock().unwrap().take() {
            worker::spawn(self.sink, self.simulate, rx);
        }
    }

    /// Изменить редактируемый профиль.
    pub fn edit(&self, f: impl FnOnce(&mut DeviceConfig)) {
        let mut c = self.sink.edit.get_untracked();
        f(&mut c);
        self.sink.edit.set(c);
    }

    /// Есть несохранённые изменения (подписывается на сигналы).
    pub fn is_dirty(&self) -> bool {
        let edit = self.sink.edit.get();
        self.sink.snapshot.get().is_some_and(|s| s != edit)
    }

    pub fn is_ready(&self) -> bool {
        self.sink.link.get() == Link::Ready && self.sink.snapshot.get().is_some()
    }

    pub fn apply(&self) {
        let Some(old) = self.sink.snapshot.get_untracked() else { return };
        let new = self.sink.edit.get_untracked();
        if let Err(msg) = new.validate() {
            self.sink.notice.set(Some(worker::Notice::new(NoticeKind::Error, msg)));
            return;
        }
        let _ = self.jobs.send(Job::Apply { new, old });
    }

    pub fn revert(&self) {
        if let Some(s) = self.sink.snapshot.get_untracked() {
            self.sink.edit.set(s);
            self.sink.notice.set(Some(worker::Notice::new(NoticeKind::Info, "Изменения отменены")));
        }
    }

    pub fn refresh(&self) {
        let _ = self.jobs.send(Job::Refresh);
    }
}

pub fn build_root(ctx: AppCtx) -> impl Widget {
    Column::new()
        .gap(0.0)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(header(ctx.clone()))
        .child(
            Row::new()
                .gap(0.0)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .child(nav(ctx.clone()))
                .child(content(ctx.clone()))
                .class("grow"),
        )
        .child(footer(ctx))
        .class("root")
}

fn header(ctx: AppCtx) -> impl Widget {
    let link_ctx = ctx.clone();
    Row::new()
        .gap(14.0)
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .child(
            DecoratedBox::new()
                .child(Center::new().child(Icon::new(icons::MOUSE).class("brand-icon")))
                .class("brand-badge"),
        )
        .child(
            Column::new()
                .gap(1.0)
                .child(Text::new("ARDOR GAMING").class("brand-kicker"))
                .child(Text::new("Edge Air Ultra").class("brand-title")),
        )
        .child(DecoratedBox::new().class("grow"))
        .child(move || link_pill(&link_ctx))
        .child(widgets::reactive_box(move || battery_pill(ctx.sink.battery.get())))
        .class("header")
}

fn link_pill(ctx: &AppCtx) -> impl Widget {
    let (tone, text) = match ctx.sink.link.get() {
        Link::Ready => ("ok", ctx.sink.device_name.get()),
        Link::Asleep => ("warn", "Мышь спит — пошевелите ей".to_string()),
        Link::Searching => ("idle", "Поиск мыши…".to_string()),
        Link::NotFound => ("bad", "Приёмник не найден".to_string()),
        Link::NoAccess(_) => ("bad", "Нет доступа к устройству".to_string()),
        Link::Error(_) => ("bad", "Ошибка связи".to_string()),
    };
    DecoratedBox::new()
        .child(
            Row::new()
                .gap(8.0)
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .child(DecoratedBox::new().class(&format!("dot dot-{tone}")))
                .child(Text::new(text).class("pill-text")),
        )
        .class("pill")
}

fn battery_pill(b: Option<Battery>) -> Box<dyn Widget> {
    let Some(b) = b else { return Box::new(DecoratedBox::new()) };
    let (icon, tone) = if b.charging {
        (icons::BATTERY_CHARGING, "ok")
    } else if b.percent <= 15 {
        (icons::BATTERY_LOW, "bad")
    } else {
        (icons::BATTERY_FULL, "ok")
    };
    let label = if b.charging { format!("{}% · заряжается", b.percent) } else { format!("{}%", b.percent) };
    Box::new(
        DecoratedBox::new()
            .child(
                Row::new()
                    .gap(6.0)
                    .cross_axis_alignment(CrossAxisAlignment::Center)
                    .child(Icon::new(icon).class(&format!("battery-icon battery-{tone}")))
                    .child(Text::new(label).class("pill-text")),
            )
            .class("pill"),
    )
}

fn nav(ctx: AppCtx) -> impl Widget {
    let mut col = Column::new().gap(6.0).cross_axis_alignment(CrossAxisAlignment::Stretch);
    col = col.child(Text::new("НАСТРОЙКИ").class("nav-caption"));
    for (i, (label, icon)) in PAGES.into_iter().enumerate() {
        let page = ctx.page;
        col = col.child(
            Button::new(label)
                .icon(icon)
                .active_index(page, i)
                .on_click(move || page.set(i))
                .class("nav-btn"),
        );
    }
    col.child(DecoratedBox::new().class("grow"))
        .child(
            Image::from_bytes("hero-mouse", HERO_PNG.to_vec())
                .fit(ImageFit::Contain)
                .class("nav-hero"),
        )
        .child(Text::new("PMW3370 · 2.4G / USB").class("nav-foot"))
        .class("nav")
}

fn content(ctx: AppCtx) -> impl Widget {
    ScrollView::new()
        .vertical()
        .child(
            Padding::all(24.0).child(widgets::reactive_box(move || {
                let has_profile = ctx.sink.snapshot.get().is_some();
                if !has_profile {
                    return Box::new(widgets::placeholder(ctx.sink.link.get()));
                }
                match ctx.page.get() {
                    0 => Box::new(pages::dpi::view(ctx.clone())),
                    1 => Box::new(pages::led::view(ctx.clone())),
                    2 => Box::new(pages::buttons::view(ctx.clone())),
                    _ => Box::new(pages::sensor::view(ctx.clone())),
                }
            })),
        )
        .class("content")
        .class("grow")
}

fn footer(ctx: AppCtx) -> impl Widget {
    let (c1, c2, c3) = (ctx.clone(), ctx.clone(), ctx.clone());
    Row::new()
        .gap(10.0)
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .child(widgets::reactive_box(move || status_line(&ctx)))
        .child(DecoratedBox::new().class("grow"))
        .child(move || {
            let c = c1.clone();
            Button::new("Считать с мыши")
                .icon(icons::REFRESH)
                .disabled(!c1.is_ready() || c1.sink.busy.get().is_some())
                .on_click(move || c.refresh())
                .class("btn-ghost")
        })
        .child(move || {
            let c = c2.clone();
            Button::new("Отменить")
                .icon(icons::UNDO)
                .disabled(!c2.is_dirty())
                .on_click(move || c.revert())
                .class("btn-ghost")
        })
        .child(move || {
            let c = c3.clone();
            let dirty = c3.is_dirty();
            let can = dirty && c3.is_ready() && c3.sink.busy.get().is_none();
            Button::new(if dirty { "Применить" } else { "Сохранено" })
                .icon(if dirty { icons::SAVE } else { icons::CHECK })
                .disabled(!can)
                .on_click(move || c.apply())
                .class("btn-primary")
        })
        .class("footer")
}

fn status_line(ctx: &AppCtx) -> Box<dyn Widget> {
    if let Some(busy) = ctx.sink.busy.get() {
        return Box::new(
            Row::new()
                .gap(10.0)
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .child(CircularProgress::new().indeterminate().size(16.0).stroke_width(2.0))
                .child(Text::new(busy).class("status-text")),
        );
    }
    if ctx.is_dirty() {
        return Box::new(
            Row::new()
                .gap(8.0)
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .child(DecoratedBox::new().class("dot dot-warn"))
                .child(Text::new("Есть несохранённые изменения").class("status-text")),
        );
    }
    match ctx.sink.notice.get() {
        Some(n) => {
            let (icon, tone) = match n.kind {
                NoticeKind::Info => (icons::INFO, "info"),
                NoticeKind::Success => (icons::CHECK, "ok"),
                NoticeKind::Error => (icons::ERROR, "bad"),
            };
            Box::new(
                Row::new()
                    .gap(8.0)
                    .cross_axis_alignment(CrossAxisAlignment::Center)
                    .child(Icon::new(icon).class(&format!("status-icon status-{tone}")))
                    .child(Text::new(n.text).class("status-text")),
            )
        }
        None => Box::new(Text::new("").class("status-text")),
    }
}
