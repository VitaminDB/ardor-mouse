//! Страница сенсора и радиоканала: частота опроса, LOD, debounce, сон.

use crate::protocol::eeprom::PollingRate;
use crate::ui::app::AppCtx;
use crate::ui::widgets::{reactive, card, page_header, setting_row};
use syngui::prelude::*;
use syngui::widgets::buttons::Segment;

/// Варианты времени до сна: (единицы по 10 с, подпись).
const SLEEP: [(u8, &str); 9] = [
    (1, "10 секунд"),
    (3, "30 секунд"),
    (6, "1 минута"),
    (12, "2 минуты"),
    (18, "3 минуты"),
    (30, "5 минут"),
    (60, "10 минут"),
    (120, "20 минут"),
    (180, "30 минут"),
];

pub fn view(ctx: AppCtx) -> impl Widget {
    Column::new()
        .gap(18.0)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(page_header("Сенсор", "Частота опроса, высота отрыва, фильтры и энергосбережение."))
        .child(card("Отклик", "", response(ctx.clone())))
        .child(card("Движение", "", motion(ctx.clone())))
        .child(card("Энергосбережение", "", power(ctx)))
        .class("page")
}

fn response(ctx: AppCtx) -> impl Widget {
    reactive(move || {
        let cfg = ctx.sink.edit.get();
        let (c_rate, c_deb) = (ctx.clone(), ctx.clone());
        let rate_idx = PollingRate::ALL.iter().position(|&r| r == cfg.polling).unwrap_or(3);
        let segments: Vec<Segment> =
            PollingRate::ALL.iter().map(|r| Segment::new(format!("{} Гц", r.hz()))).collect();
        Column::new()
            .gap(20.0)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(setting_row(
                "Частота опроса",
                "Как часто мышь сообщает о движении. 1000 Гц — минимальная задержка.",
                Box::new(
                    SegmentedButton::new(segments)
                        .selected(rate_idx)
                        .on_change(move |i| c_rate.edit(|c| c.polling = PollingRate::ALL[i]))
                        .class("seg"),
                ),
            ))
            .child(setting_row(
                "Защита от дребезга (debounce)",
                &format!("Задержка срабатывания кнопок: {} мс. Меньше — быстрее, больше — без двойных кликов.", cfg.debounce_ms),
                Box::new(
                    Slider::new()
                        .value(cfg.debounce_ms as f32)
                        .range(0.0, 30.0)
                        .step(1.0)
                        .show_value(0)
                        .width(260.0)
                        .on_change(move |v| c_deb.edit(|c| c.debounce_ms = v.round() as u8)),
                ),
            ))
    })
}

fn motion(ctx: AppCtx) -> impl Widget {
    reactive(move || {
        let cfg = ctx.sink.edit.get();
        let (c_lod, c_ang, c_rip) = (ctx.clone(), ctx.clone(), ctx.clone());
        Column::new()
            .gap(20.0)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(setting_row(
                "Высота отрыва (LOD)",
                "На какой высоте над ковриком сенсор перестаёт отслеживать движение.",
                Box::new(
                    SegmentedButton::new(vec![Segment::new("1 мм"), Segment::new("2 мм")])
                        .selected(if cfg.lod == 2 { 1 } else { 0 })
                        .on_change(move |i| c_lod.edit(|c| c.lod = i as u8 + 1))
                        .class("seg"),
                ),
            ))
            .child(setting_row(
                "Выпрямление линий (angle snapping)",
                "Сглаживает дрожание при рисовании прямых. Для игр обычно выключают.",
                Box::new(Toggle::new().on(cfg.angle_snap).on_change(move |v| c_ang.edit(|c| c.angle_snap = v))),
            ))
            .child(setting_row(
                "Подавление пульсаций (ripple control)",
                "Фильтрует шум сенсора на высоких DPI ценой небольшой задержки.",
                Box::new(Toggle::new().on(cfg.ripple).on_change(move |v| c_rip.edit(|c| c.ripple = v))),
            ))
    })
}

fn power(ctx: AppCtx) -> impl Widget {
    reactive(move || {
        let cfg = ctx.sink.edit.get();
        let c = ctx.clone();
        let mut items: Vec<DropdownItem> =
            SLEEP.iter().map(|(v, l)| DropdownItem::new(v.to_string(), *l)).collect();
        if !SLEEP.iter().any(|(v, _)| *v == cfg.sleep_x10s) {
            items.push(DropdownItem::new(cfg.sleep_x10s.to_string(), format!("{} с", cfg.sleep_x10s as u32 * 10)));
        }
        setting_row(
            "Переход в сон",
            "Через сколько секунд бездействия беспроводная мышь засыпает.",
            Box::new(
                Dropdown::with_items(items)
                    .selected(cfg.sleep_x10s.to_string())
                    .width(200.0)
                    .on_change(move |v| {
                        if let Ok(v) = v.parse::<u8>() {
                            c.edit(|cfg| cfg.sleep_x10s = v);
                        }
                    }),
            ),
        )
    })
}
