//! Страница DPI: ступени, значение, цвет индикатора, активная ступень.

use crate::protocol::dpi::Sensor;
use crate::protocol::led::Rgb;
use crate::ui::app::AppCtx;
use crate::ui::icons;
use crate::ui::mouse_view::{mouse_view, Place, Scene};
use crate::ui::widgets::{card, page_header, reactive, swatch_dot};
use syngui::prelude::*;
use syngui::t;
use syngui::widgets::*;
use syngui::CursorIcon;

/// Быстрые значения; последнее — максимум сенсора.
fn presets(sensor: Sensor) -> [u16; 6] {
    [400, 800, 1600, 3200, 6400, sensor.max()]
}

/// Палитра быстрых цветов индикатора.
const PALETTE: [Rgb; 8] = [
    Rgb::new(255, 0, 0),
    Rgb::new(255, 128, 0),
    Rgb::new(255, 255, 0),
    Rgb::new(0, 255, 0),
    Rgb::new(0, 255, 255),
    Rgb::new(0, 0, 255),
    Rgb::new(255, 0, 255),
    Rgb::new(255, 255, 255),
];

/// Подпись под шкалой: сетка значений сенсора.
fn grid_hint(sensor: Sensor) -> String {
    match sensor {
        Sensor::Pmw3370 => t!("шаг 50 до 10 000, дальше 100"),
        Sensor::Paw3950 => t!("шаг 50"),
    }
}

pub fn view(ctx: AppCtx) -> impl Widget {
    let sensor = ctx.sink.edit.get_untracked().model.sensor();
    Column::new()
        .gap(18.0)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(page_header(
            "DPI",
            &t!(
                "Ступени чувствительности сенсора {sensor}. Кнопка DPI на мыши переключает их по кругу.",
                sensor = sensor.name()
            ),
        ))
        .child(card(
            &t!("Ступени"),
            &t!("Нажмите на ступень, чтобы настроить её. Звезда — ступень, включённая сейчас."),
            levels_row(ctx.clone()),
        ))
        .child(
            Row::new()
                .gap(18.0)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .child(DecoratedBox::new().child(card(&t!("Настройка ступени"), "", editor(ctx.clone()))).class("grow"))
                .child(preview(ctx)),
        )
        .class("page")
}

/// Мышь, у которой индикатор горит цветом выбранной ступени; при выборе
/// другой ступени — вспышка.
fn preview(ctx: AppCtx) -> impl Widget {
    DecoratedBox::new()
        .child(reactive(move || {
            let cfg = ctx.sink.edit.get();
            let i = ctx.sel_level.get().min(cfg.dpi_count as usize - 1);
            let lvl = cfg.dpi[i];
            let key = (i as u64) << 32 | (lvl.color.r as u64) << 16 | (lvl.color.g as u64) << 8 | lvl.color.b as u64;
            Column::new()
                .gap(10.0)
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .child(Text::new(t!("Индикатор")).class("card-title"))
                .child(mouse_view(cfg.model, Place::Preview, Scene::Dpi(lvl.color), key))
                .child(
                    Row::new()
                        .gap(8.0)
                        .cross_axis_alignment(CrossAxisAlignment::Center)
                        .child(swatch_dot(lvl.color.r, lvl.color.g, lvl.color.b, "level-dot"))
                        .child(Text::new(t!("Ступень {n} · {dpi} DPI", n = i + 1, dpi = lvl.dpi)).class("preview-caption")),
                )
        }))
        .class("card preview-card")
}

fn levels_row(ctx: AppCtx) -> impl Widget {
    reactive(move || {
        let cfg = ctx.sink.edit.get();
        let count = cfg.dpi_count as usize;
        let max = cfg.model.spec().max_dpi_stages;
        let sel = ctx.sel_level.get().min(count - 1);
        let mut row = Row::new().gap(10.0).cross_axis_alignment(CrossAxisAlignment::Stretch);
        for i in 0..count {
            let lvl = cfg.dpi[i];
            let active = i == cfg.dpi_active as usize;
            let sel_sig = ctx.sel_level;
            let mut head = Row::new()
                .gap(6.0)
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .child(swatch_dot(lvl.color.r, lvl.color.g, lvl.color.b, "level-dot"))
                .child(Text::new(t!("Ступень {n}", n = i + 1)).class("level-name"));
            if active {
                head = head.child(Icon::new(icons::STAR).class("level-star"));
            }
            let body = Column::new()
                .gap(6.0)
                .child(head)
                .child(Text::new(lvl.dpi.to_string()).class("level-dpi"));
            row = row.child(
                GestureDetector::new()
                    .on_click(move || sel_sig.set(i))
                    .cursor(CursorIcon::Pointer)
                    .child(DecoratedBox::new().child(body).class(if i == sel {
                        "level-card level-card-sel"
                    } else {
                        "level-card"
                    })),
            );
        }
        let (c_add, c_del) = (ctx.clone(), ctx.clone());
        row.child(
            Column::new()
                .gap(6.0)
                .child(
                    ToolButton::new(icons::ADD)
                        .tooltip(t!("Добавить ступень"))
                        .disabled(count >= max as usize)
                        .on_click(move || {
                            c_add.edit(|c| c.dpi_count = (c.dpi_count + 1).min(max));
                            c_add.sel_level.set(count);
                        })
                        .class("count-btn"),
                )
                .child(
                    ToolButton::new(icons::REMOVE)
                        .tooltip(t!("Убрать последнюю ступень"))
                        .disabled(count <= 1)
                        .on_click(move || {
                            c_del.edit(|c| {
                                c.dpi_count = c.dpi_count.saturating_sub(1).max(1);
                                c.dpi_active = c.dpi_active.min(c.dpi_count - 1);
                            })
                        })
                        .class("count-btn"),
                ),
        )
    })
}

fn editor(ctx: AppCtx) -> impl Widget {
    reactive(move || {
        let cfg = ctx.sink.edit.get();
        let sensor = cfg.model.sensor();
        let i = ctx.sel_level.get().min(cfg.dpi_count as usize - 1);
        let lvl = cfg.dpi[i];
        let is_active = i == cfg.dpi_active as usize;

        let (c_slider, c_active) = (ctx.clone(), ctx.clone());
        let active_ctl: Box<dyn Widget> = if is_active {
            Box::new(
                DecoratedBox::new()
                    .child(
                        Row::new()
                            .gap(6.0)
                            .cross_axis_alignment(CrossAxisAlignment::Center)
                            .child(Icon::new(icons::STAR).class("tag-icon"))
                            .child(Text::new(t!("Активная ступень")).class("tag-text")),
                    )
                    .class("tag"),
            )
        } else {
            Box::new(
                Button::new(t!("Сделать активной"))
                    .icon(icons::STAR)
                    .on_click(move || c_active.edit(|c| c.dpi_active = i as u8))
                    .class("btn-ghost"),
            )
        };

        let mut chips = Row::new().gap(8.0);
        for p in presets(sensor) {
            let c = ctx.clone();
            chips = chips.child(
                Button::new(p.to_string())
                    .on_click(move || c.edit(|cfg| cfg.dpi[i].dpi = p))
                    .class(if lvl.dpi == p { "chip chip-on" } else { "chip" }),
            );
        }

        let mut palette = Row::new().gap(8.0).cross_axis_alignment(CrossAxisAlignment::Center);
        for col in PALETTE {
            let c = ctx.clone();
            let cls = if lvl.color == col { "color-swatch color-swatch-on" } else { "color-swatch" };
            palette = palette.child(
                GestureDetector::new()
                    .on_click(move || c.edit(|cfg| cfg.dpi[i].color = col))
                    .cursor(CursorIcon::Pointer)
                    .child(swatch_dot(col.r, col.g, col.b, cls)),
            );
        }
        let c_pick = ctx.clone();
        palette = palette.child(
            ColorPicker::new()
                .color(ColorValue::new(lvl.color.r, lvl.color.g, lvl.color.b))
                .width(150.0)
                .on_change(move |v| c_pick.edit(|cfg| cfg.dpi[i].color = Rgb::new(v.r, v.g, v.b))),
        );

        Column::new()
            .gap(18.0)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(
                Row::new()
                    .gap(10.0)
                    .cross_axis_alignment(CrossAxisAlignment::End)
                    .child(Text::new(t!("Ступень {n}", n = i + 1)).class("editor-level"))
                    .child(DecoratedBox::new().class("grow"))
                    .children(vec![active_ctl]),
            )
            .child(
                Row::new()
                    .gap(8.0)
                    .cross_axis_alignment(CrossAxisAlignment::End)
                    .child(Text::new(lvl.dpi.to_string()).class("dpi-big"))
                    .child(Text::new("DPI").class("dpi-unit")),
            )
            .child(
                Slider::new()
                    .value(lvl.dpi as f32)
                    .range(sensor.min() as f32, sensor.max() as f32)
                    .step(50.0)
                    .on_change(move |v| {
                        let v = sensor.snap(v.round() as u16);
                        c_slider.edit(|c| c.dpi[i].dpi = v)
                    })
                    .class("dpi-slider"),
            )
            .child(
                Row::new()
                    .child(Text::new(sensor.min().to_string()).class("scale-label"))
                    .child(DecoratedBox::new().class("grow"))
                    .child(Text::new(grid_hint(sensor)).class("scale-label"))
                    .child(DecoratedBox::new().class("grow"))
                    .child(Text::new(sensor.max().to_string()).class("scale-label")),
            )
            .child(chips)
            .child(Text::new(t!("Цвет индикатора ступени")).class("field-label"))
            .child(palette)
    })
}
