//! Страница DPI: ступени, значение, цвет индикатора, активная ступень.

use crate::protocol::dpi;
use crate::protocol::eeprom::DPI_SLOTS;
use crate::protocol::led::Rgb;
use crate::ui::app::AppCtx;
use crate::ui::icons;
use crate::ui::widgets::{reactive, card, page_header, swatch_dot};
use syngui::prelude::*;
use syngui::CursorIcon;
use syngui::widgets::*;

const PRESETS: [u16; 8] = [400, 800, 1200, 1600, 2400, 3200, 6400, 12_000];

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

pub fn view(ctx: AppCtx) -> impl Widget {
    Column::new()
        .gap(18.0)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(page_header(
            "DPI",
            "Ступени чувствительности сенсора PMW3370. Кнопка DPI на мыши переключает их по кругу.",
        ))
        .child(card(
            "Ступени",
            "Нажмите на ступень, чтобы настроить её. Звезда — ступень, включённая сейчас.",
            levels_row(ctx.clone()),
        ))
        .child(card("Настройка ступени", "", editor(ctx)))
        .class("page")
}

fn levels_row(ctx: AppCtx) -> impl Widget {
    reactive(move || {
        let cfg = ctx.sink.edit.get();
        let count = cfg.dpi_count as usize;
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
                .child(Text::new(format!("Ступень {}", i + 1)).class("level-name"));
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
                        .tooltip("Добавить ступень")
                        .disabled(count >= DPI_SLOTS)
                        .on_click(move || {
                            c_add.edit(|c| c.dpi_count = (c.dpi_count + 1).min(DPI_SLOTS as u8));
                            c_add.sel_level.set(count);
                        })
                        .class("count-btn"),
                )
                .child(
                    ToolButton::new(icons::REMOVE)
                        .tooltip("Убрать последнюю ступень")
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
                            .child(Text::new("Активная ступень").class("tag-text")),
                    )
                    .class("tag"),
            )
        } else {
            Box::new(
                Button::new("Сделать активной")
                    .icon(icons::STAR)
                    .on_click(move || c_active.edit(|c| c.dpi_active = i as u8))
                    .class("btn-ghost"),
            )
        };

        let mut presets = Row::new().gap(8.0);
        for p in PRESETS {
            let c = ctx.clone();
            presets = presets.child(
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
                    .child(Text::new(format!("Ступень {}", i + 1)).class("editor-level"))
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
                    .range(dpi::MIN as f32, dpi::MAX as f32)
                    .step(50.0)
                    .on_change(move |v| {
                        let v = dpi::snap(v.round() as u16);
                        c_slider.edit(|c| c.dpi[i].dpi = v)
                    })
                    .class("dpi-slider"),
            )
            .child(
                Row::new()
                    .child(Text::new(format!("{}", dpi::MIN)).class("scale-label"))
                    .child(DecoratedBox::new().class("grow"))
                    .child(Text::new("шаг 50 до 10 000, дальше 100").class("scale-label"))
                    .child(DecoratedBox::new().class("grow"))
                    .child(Text::new(format!("{}", dpi::MAX)).class("scale-label")),
            )
            .child(presets)
            .child(Text::new("Цвет индикатора ступени").class("field-label"))
            .child(palette)
    })
}
