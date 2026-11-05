//! Страница сенсора и радиоканала: частота опроса, LOD, фильтры, сон.

use crate::protocol::model::Lod;
use crate::ui::app::AppCtx;
use crate::ui::widgets::{card, page_header, reactive, setting_row};
use syngui::prelude::*;
use syngui::widgets::buttons::Segment;
use syngui::{t, tn};

/// Варианты времени до сна, в единицах по 10 с.
const SLEEP: [u8; 9] = [1, 3, 6, 12, 18, 30, 60, 120, 180];

fn sleep_label(x10s: u8) -> String {
    let secs = x10s as u32 * 10;
    if secs < 60 || secs % 60 != 0 {
        tn!(secs, "{n} секунда", "{n} секунды", "{n} секунд")
    } else {
        tn!(secs / 60, "{n} минута", "{n} минуты", "{n} минут")
    }
}

/// «0,7 мм» / «0.7 mm» — разделитель дробной части по языку.
fn lod_label(l: Lod) -> String {
    let v = if l.tenths_mm % 10 == 0 {
        (l.tenths_mm / 10).to_string()
    } else {
        // в каталоге: "," → "." для языков с десятичной точкой
        format!("{}{}{}", l.tenths_mm / 10, t!(","), l.tenths_mm % 10)
    };
    t!("{v} мм", v = v)
}

pub fn view(ctx: AppCtx) -> impl Widget {
    Column::new()
        .gap(18.0)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(page_header(&t!("Сенсор"), &t!("Частота опроса, высота отрыва, фильтры и энергосбережение.")))
        .child(card(&t!("Отклик"), "", response(ctx.clone())))
        .child(card(&t!("Движение"), "", motion(ctx.clone())))
        .child(card(&t!("Энергосбережение"), "", power(ctx)))
        .class("page")
}

fn response(ctx: AppCtx) -> impl Widget {
    reactive(move || {
        let cfg = ctx.sink.edit.get();
        let (c_rate, c_deb) = (ctx.clone(), ctx.clone());
        let rates = cfg.model.spec().polling_rates;
        let rate_idx = rates.iter().position(|&r| r == cfg.polling).unwrap_or(rates.len() - 1);
        let segments: Vec<Segment> = rates.iter().map(|r| Segment::new(t!("{hz} Гц", hz = r.hz()))).collect();
        let top = rates.last().map_or(1000, |r| r.hz());
        Column::new()
            .gap(20.0)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(setting_row(
                &t!("Частота опроса"),
                &t!("Как часто мышь сообщает о движении. {hz} Гц — минимальная задержка.", hz = top),
                Box::new(
                    SegmentedButton::new(segments)
                        .selected(rate_idx)
                        .on_change(move |i| c_rate.edit(|c| c.polling = rates[i]))
                        .class("seg"),
                ),
            ))
            .child(setting_row(
                &t!("Защита от дребезга (debounce)"),
                &t!(
                    "Задержка срабатывания кнопок: {ms} мс. Меньше — быстрее, больше — без двойных кликов.",
                    ms = cfg.debounce_ms
                ),
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
        let spec = cfg.model.spec();
        let (c_lod, c_ang, c_rip, c_ms) = (ctx.clone(), ctx.clone(), ctx.clone(), ctx.clone());
        let lods = spec.lod;
        let mut col = Column::new()
            .gap(20.0)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(setting_row(
                &t!("Высота отрыва (LOD)"),
                &t!("На какой высоте над ковриком сенсор перестаёт отслеживать движение."),
                Box::new(
                    SegmentedButton::new(lods.iter().map(|&l| Segment::new(lod_label(l))).collect())
                        .selected(lods.iter().position(|l| l.hw == cfg.lod).unwrap_or(0))
                        .on_change(move |i| c_lod.edit(|c| c.lod = lods[i].hw))
                        .class("seg"),
                ),
            ))
            .child(setting_row(
                &t!("Выпрямление линий (angle snapping)"),
                &t!("Сглаживает дрожание при рисовании прямых. Для игр обычно выключают."),
                Box::new(Toggle::new().on(cfg.angle_snap).on_change(move |v| c_ang.edit(|c| c.angle_snap = v))),
            ))
            .child(setting_row(
                &t!("Подавление пульсаций (ripple control)"),
                &t!("Фильтрует шум сенсора на высоких DPI ценой небольшой задержки."),
                Box::new(Toggle::new().on(cfg.ripple).on_change(move |v| c_rip.edit(|c| c.ripple = v))),
            ));
        if spec.motion_sync {
            col = col.child(setting_row(
                "Motion Sync",
                &t!("Синхронизирует опрос сенсора с отправкой отчётов: движение ровнее, задержка чуть больше (около половины интервала опроса)."),
                Box::new(Toggle::new().on(cfg.motion_sync).on_change(move |v| c_ms.edit(|c| c.motion_sync = v))),
            ));
        }
        col
    })
}

fn power(ctx: AppCtx) -> impl Widget {
    reactive(move || {
        let cfg = ctx.sink.edit.get();
        let (c, c_mode) = (ctx.clone(), ctx.clone());
        let mut items: Vec<DropdownItem> =
            SLEEP.iter().map(|&v| DropdownItem::new(v.to_string(), sleep_label(v))).collect();
        if !SLEEP.contains(&cfg.sleep_x10s) {
            items.push(DropdownItem::new(cfg.sleep_x10s.to_string(), sleep_label(cfg.sleep_x10s)));
        }
        let mut col = Column::new().gap(20.0).cross_axis_alignment(CrossAxisAlignment::Stretch).child(setting_row(
            &t!("Переход в сон"),
            &t!("Через сколько секунд бездействия беспроводная мышь засыпает."),
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
        ));
        if cfg.model.spec().sensor_mode {
            col = col.child(setting_row(
                &t!("Режим сенсора"),
                &t!("HP — максимальная частота кадров сенсора и минимальная задержка. LP — экономия батареи ценой чуть худшего трекинга на резких движениях."),
                Box::new(
                    SegmentedButton::new(vec![Segment::new("LP"), Segment::new("HP")])
                        .selected(cfg.sensor_hp as usize)
                        .on_change(move |i| c_mode.edit(|c| c.sensor_hp = i == 1))
                        .class("seg"),
                ),
            ));
        }
        col
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sleep_labels() {
        syngui::i18n::set_source_language("ru");
        syngui::i18n::set_language("ru");
        assert_eq!(sleep_label(3), "30 секунд");
        assert_eq!(sleep_label(6), "1 минута");
        assert_eq!(sleep_label(12), "2 минуты");
        assert_eq!(sleep_label(30), "5 минут");
        assert_eq!(sleep_label(9), "90 секунд");
    }
}
