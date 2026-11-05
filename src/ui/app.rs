//! Контекст приложения и оболочка окна: свой заголовок (CSD), навигация,
//! страница, панель действий.

use super::mouse_view::{mouse_view, Place, Scene};
use super::{icons, pages, widgets};
use crate::protocol::eeprom::DeviceConfig;
use crate::protocol::model::Model;
use crate::settings::Settings;
use crate::transport::sim::SimTransport;
use crate::transport::{Battery, Device};
use crate::worker::{self, Job, Link, NoticeKind, Options, Sink};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use syngui::overlay::{ControlsSide, SystemWindowControls, WindowDragRegion, WindowResizeRegion};
use syngui::prelude::*;
use syngui::widgets::RepeatMode;
use syngui::window::WindowState;
use syngui::i18n::t;
use syngui::{n_, t};

/// Разделы навигации; индекс в массиве — значение `AppCtx::page`.
pub const PAGES: [(&str, &str); 4] = [
    (n_!("DPI"), icons::SPEED),
    (n_!("Подсветка"), icons::LIGHT),
    (n_!("Кнопки"), icons::BUTTONS),
    (n_!("Сенсор"), icons::TUNE),
];
const PAGE_LED: usize = 1;

/// Есть ли раздел у этой мыши (подсветки у Rukh нет).
fn page_available(page: usize, model: Option<Model>) -> bool {
    page != PAGE_LED || model.is_none_or(|m| m.spec().body_led)
}

#[derive(Clone)]
pub struct AppCtx {
    pub sink: Sink,
    pub page: RwSignal<usize>,
    /// Ступень DPI, открытая в редакторе.
    pub sel_level: RwSignal<usize>,
    /// Кнопка, на которую навели в списке, — подсвечивается на схеме.
    pub focus_button: RwSignal<Option<usize>>,
    pub window: RwSignal<WindowState>,
    prefs: Arc<Mutex<Settings>>,
    jobs: Sender<Job>,
    pending: Arc<Mutex<Option<Receiver<Job>>>>,
    opts: Options,
}

impl AppCtx {
    /// Создаёт сигналы. Вызывать один раз, до `App::run`.
    pub fn new(opts: Options, prefs: Settings) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            sink: Sink {
                link: use_signal(Link::Searching),
                model: use_signal(None::<Model>),
                device_name: use_signal(String::new()),
                battery: use_signal(None::<Battery>),
                snapshot: use_signal(None::<DeviceConfig>),
                edit: use_signal(DeviceConfig::default()),
                busy: use_signal(None::<String>),
                notice: use_signal(None),
            },
            page: use_signal(0usize),
            sel_level: use_signal(1usize),
            focus_button: use_signal(None::<usize>),
            window: use_signal(WindowState::default()),
            prefs: Arc::new(Mutex::new(prefs)),
            jobs: tx,
            pending: Arc::new(Mutex::new(Some(rx))),
            opts,
        }
    }

    /// Запускает поток устройства (после инициализации рантайма окна).
    pub fn start_worker(&self) {
        if let Some(rx) = self.pending.lock().unwrap().take() {
            worker::spawn(self.sink, self.opts.clone(), rx);
        }
    }

    pub fn simulated_model(&self) -> Option<Model> {
        self.opts.simulate
    }

    /// Для снимков окна: профиль симулятора читается сразу, без потока.
    pub fn preload_simulator(&self, model: Model) {
        let mut dev = Device::new(Box::new(SimTransport::with_model(model)));
        let s = &self.sink;
        if let (Ok(mem), Ok(b)) = (dev.read_profile(), dev.battery()) {
            let (cfg, _) = DeviceConfig::decode(&mem, model);
            s.edit.set(cfg.clone());
            s.snapshot.set(Some(cfg));
            s.battery.set(Some(b));
        }
        s.model.set(Some(model));
        s.device_name.set(dev.name());
        s.link.set(Link::Ready);
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

    pub fn read_only(&self) -> bool {
        self.opts.read_only
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
            self.sink.notice.set(Some(worker::Notice::new(NoticeKind::Info, t!("Изменения отменены"))));
        }
    }

    pub fn refresh(&self) {
        let _ = self.jobs.send(Job::Refresh);
    }

    /// Выбранный язык (`None` — как в системе).
    pub fn language_pref(&self) -> Option<String> {
        self.prefs.lock().unwrap().language.clone()
    }

    /// Сменить язык интерфейса и запомнить выбор.
    pub fn set_language(&self, tag: Option<String>) {
        let lang = tag.clone().unwrap_or_else(|| syngui::i18n::system_language().to_string());
        let mut p = self.prefs.lock().unwrap();
        p.language = tag;
        p.save();
        drop(p);
        syngui::i18n::set_language(lang.as_str());
    }
}

pub fn build_root(ctx: AppCtx) -> impl Widget {
    // Рамка окна перестраивается только при смене «развёрнуто», а не фокуса.
    let maximized = use_signal(false);
    let win = ctx.window;
    create_effect(move || {
        let v = win.get().maximized;
        if maximized.get_untracked() != v {
            maximized.set(v);
        }
    });
    Reactive::new(move || {
        // смена языка перестраивает дерево на месте, без потери состояния
        syngui::i18n::subscribe();
        let max = maximized.get();
        let c = ctx.clone();
        let window = Column::new()
            .gap(0.0)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(titlebar(c.clone()))
            .child(
                Row::new()
                    .gap(0.0)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .child(nav(c.clone()))
                    .child(content(c.clone()))
                    .class("grow"),
            )
            .child(footer(c))
            .class("window");
        let frame = DecoratedBox::new().class("window-frame").clip(true).child(window);
        vec![Box::new(WindowResizeRegion::new().over_content(true).inset(5.0).enabled(!max).child(frame))
            as Box<dyn Widget>]
    })
}

fn titlebar(ctx: AppCtx) -> impl Widget {
    let (c_title, c_link, c_bat, c_lang) = (ctx.clone(), ctx.clone(), ctx.clone(), ctx.clone());
    let win = ctx.window;
    DecoratedBox::new().class("titlebar").child(
        Row::new()
            .gap(0.0)
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .child(
                DecoratedBox::new().class("grow").child(
                    WindowDragRegion::new().child(
                        Row::new()
                            .gap(12.0)
                            .cross_axis_alignment(CrossAxisAlignment::Center)
                            .child(
                                DecoratedBox::new()
                                    .child(Center::new().child(Icon::new(icons::MOUSE).class("brand-icon")))
                                    .class("brand-badge"),
                            )
                            .child(
                                Column::new()
                                    .gap(0.0)
                                    .child(Text::new("ARDOR GAMING").class("brand-kicker"))
                                    .child(widgets::reactive(move || {
                                        let name =
                                            c_title.sink.model.get().map_or_else(|| t!("Мышь"), |m| m.name().into());
                                        Text::new(name).class("brand-title")
                                    })),
                            )
                            .child(DecoratedBox::new().class("grow drag-space"))
                            .class("tb-drag"),
                    ),
                ),
            )
            .child(
                Row::new()
                    .gap(8.0)
                    .cross_axis_alignment(CrossAxisAlignment::Center)
                    .child(move || link_pill(&c_link))
                    .child(widgets::reactive_box(move || battery_pill(c_bat.sink.battery.get())))
                    .child(move || language_picker(&c_lang))
                    .class("tb-tools"),
            )
            .child(DecoratedBox::new().class("window-controls").child(move || {
                let w = win.get();
                SystemWindowControls::new(ControlsSide::Right).maximized(w.maximized).active(w.focused)
            })),
    )
}

fn language_picker(ctx: &AppCtx) -> impl Widget {
    syngui::i18n::subscribe();
    let mut items = vec![DropdownItem::new("system", t!("Как в системе"))];
    for l in syngui::i18n::languages() {
        items.push(DropdownItem::new(l.tag.tag(), l.name));
    }
    let selected = ctx.language_pref().unwrap_or_else(|| "system".into());
    let c = ctx.clone();
    Row::new()
        .gap(4.0)
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .child(Icon::new(icons::LANGUAGE).class("tb-icon"))
        .child(
            Dropdown::with_items(items)
                .selected(selected)
                .width(150.0)
                .on_change(move |id| c.set_language((id != "system").then(|| id.to_string())))
                .class("lang-dropdown"),
        )
}

fn link_pill(ctx: &AppCtx) -> impl Widget {
    let (tone, text) = match ctx.sink.link.get() {
        Link::Ready => ("ok", ctx.sink.device_name.get()),
        Link::Asleep => ("warn", t!("Мышь спит — пошевелите ей")),
        Link::Searching => ("idle", t!("Поиск мыши…")),
        Link::NotFound => ("bad", t!("Приёмник не найден")),
        Link::NoAccess(_) => ("bad", t!("Нет доступа к устройству")),
        Link::Error(_) => ("bad", t!("Ошибка связи")),
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
    let label = if b.charging {
        t!("{p}% · заряжается", p = b.percent)
    } else {
        format!("{}%", b.percent)
    };
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
    let (c_pages, c_hero) = (ctx.clone(), ctx.clone());
    Column::new()
        .gap(6.0)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(move || {
            syngui::i18n::subscribe();
            Text::new(t!("НАСТРОЙКИ")).class("nav-caption")
        })
        .child(widgets::reactive(move || {
            let model = c_pages.sink.model.get();
            let mut col = Column::new().gap(6.0).cross_axis_alignment(CrossAxisAlignment::Stretch);
            for (i, (label, icon)) in PAGES.into_iter().enumerate() {
                if !page_available(i, model) {
                    continue;
                }
                let page = c_pages.page;
                col = col.child(
                    Button::new(t(label))
                        .icon(icon)
                        .active_index(page, i)
                        .on_click(move || page.set(i))
                        .class("nav-btn"),
                );
            }
            col
        }))
        .child(DecoratedBox::new().class("grow"))
        .child(widgets::reactive_box(move || hero(&c_hero)))
        .class("nav")
}

/// Мышь в боковой панели: покачивается и светится текущими настройками.
fn hero(ctx: &AppCtx) -> Box<dyn Widget> {
    let Some(model) = ctx.sink.model.get() else { return Box::new(DecoratedBox::new()) };
    let cfg = ctx.sink.edit.get();
    let scene = if model.spec().body_led && cfg.model == model {
        Scene::Lighting(cfg.led)
    } else {
        Scene::Dpi(cfg.dpi[cfg.dpi_active as usize].color)
    };
    let float = Animation::tween(Easing::EaseInOutSine).from(-4.0).to(4.0).duration_ms(2600).build();
    Box::new(
        Column::new()
            .gap(0.0)
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .child(
                Animated::new(mouse_view(model, Place::Hero, scene, 0))
                    .translate_y(float)
                    .repeat_mode(RepeatMode::PingPong(0)),
            )
            .child(DecoratedBox::new().class("hero-shadow"))
            .child(Text::new(format!("{} · 2.4G / USB", model.sensor().name())).class("nav-foot")),
    )
}

fn content(ctx: AppCtx) -> impl Widget {
    ScrollView::new()
        .vertical()
        .child(Padding::all(24.0).child(widgets::reactive_box(move || {
            if ctx.sink.snapshot.with(|s| s.is_none()) {
                return Box::new(widgets::placeholder(ctx.sink.link.get()));
            }
            let page = ctx.page.get();
            let page = if page_available(page, ctx.sink.model.get()) { page } else { 0 };
            let c = ctx.clone();
            Box::new(
                AnimatedSwitcher::new(page as u64, move || -> Box<dyn Widget> {
                    match page {
                        0 => Box::new(pages::dpi::view(c.clone())),
                        1 => Box::new(pages::led::view(c.clone())),
                        2 => Box::new(pages::buttons::view(c.clone())),
                        _ => Box::new(pages::sensor::view(c.clone())),
                    }
                })
                .slide(0.0, 18.0)
                .duration_ms(260)
                .easing(Easing::EMPHASIZED_DECELERATE),
            )
        })))
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
            Button::new(t!("Считать с мыши"))
                .icon(icons::REFRESH)
                .disabled(!c1.is_ready() || c1.sink.busy.get().is_some())
                .on_click(move || c.refresh())
                .class("btn-ghost")
        })
        .child(move || {
            let c = c2.clone();
            Button::new(t!("Отменить"))
                .icon(icons::UNDO)
                .disabled(!c2.is_dirty())
                .on_click(move || c.revert())
                .class("btn-ghost")
        })
        .child(move || {
            let c = c3.clone();
            let dirty = c3.is_dirty();
            let ro = c3.read_only();
            let can = dirty && c3.is_ready() && c3.sink.busy.get().is_none() && !ro;
            let (label, icon) = match (ro, dirty) {
                (true, _) => (t!("Только чтение"), icons::LOCK),
                (false, true) => (t!("Применить"), icons::SAVE),
                (false, false) => (t!("Сохранено"), icons::CHECK),
            };
            Button::new(label)
                .icon(icon)
                .disabled(!can)
                .on_click(move || c.apply())
                .class(if can { "btn-primary btn-pulse" } else { "btn-primary" })
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
                .child(Text::new(t!("Есть несохранённые изменения")).class("status-text")),
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
