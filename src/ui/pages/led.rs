//! Страница подсветки корпуса.

use crate::protocol::led::{LedMode, Rgb, LEVELS};
use crate::ui::app::AppCtx;
use crate::ui::icons;
use crate::ui::widgets::{reactive, reactive_box, card, labeled, page_header, setting_row, swatch_dot};
use syngui::prelude::*;
use syngui::{CursorIcon, StyleValue};
use syngui::widgets::*;

const PREVIEW_PNG: &[u8] = include_bytes!("../../../assets/skins/0806/mouse_led.png");

const PALETTE: [Rgb; 8] = [
    Rgb::new(255, 0, 0),
    Rgb::new(255, 96, 0),
    Rgb::new(255, 220, 0),
    Rgb::new(0, 255, 64),
    Rgb::new(0, 220, 255),
    Rgb::new(0, 64, 255),
    Rgb::new(160, 0, 255),
    Rgb::new(255, 0, 255),
];

fn mode_icon(m: LedMode) -> &'static str {
    match m {
        LedMode::Steady => icons::LED_STEADY,
        LedMode::Breathing => icons::LED_BREATH,
        LedMode::Streaming => icons::LED_STREAM,
        LedMode::Neon => icons::LED_NEON,
        LedMode::Scrolling => icons::LED_SCROLL,
        LedMode::ColorBreathing => icons::LED_COLOR_BREATH,
        LedMode::Off => icons::LED_OFF,
    }
}

pub fn view(ctx: AppCtx) -> impl Widget {
    Column::new()
        .gap(18.0)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(page_header("Подсветка", "RGB-подсветка корпуса: эффект, цвет, скорость и яркость."))
        .child(
            Row::new()
                .gap(18.0)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .child(DecoratedBox::new().child(card("Эффект", "", modes(ctx.clone()))).class("grow"))
                .child(preview(ctx.clone())),
        )
        .child(card("Параметры эффекта", "", params(ctx.clone())))
        .child(card("Дополнительно", "", extra(ctx)))
        .class("page")
}

fn modes(ctx: AppCtx) -> impl Widget {
    reactive(move || {
        let cur = ctx.sink.edit.get().led.mode;
        let mut grid = Grid::new(4).gap(10.0);
        for m in LedMode::ALL {
            let c = ctx.clone();
            grid = grid.child(
                GestureDetector::new()
                    .on_click(move || c.edit(|cfg| cfg.led.mode = m))
                    .cursor(CursorIcon::Pointer)
                    .child(
                        DecoratedBox::new()
                            .child(
                                Column::new()
                                    .gap(8.0)
                                    .cross_axis_alignment(CrossAxisAlignment::Center)
                                    .child(Icon::new(mode_icon(m)).class("mode-icon"))
                                    .child(Text::new(m.label()).class("mode-label")),
                            )
                            .class(if m == cur { "mode-tile mode-tile-on" } else { "mode-tile" }),
                    ),
            );
        }
        grid
    })
}

fn preview(ctx: AppCtx) -> impl Widget {
    DecoratedBox::new()
        .child(
            Column::new()
                .gap(0.0)
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .child(Text::new("Превью").class("card-title"))
                .child(
                    Image::from_bytes("led-preview", PREVIEW_PNG.to_vec())
                        .fit(ImageFit::Contain)
                        .class("led-preview-img"),
                )
                .child(move || {
                    let led = ctx.sink.edit.get().led;
                    let opacity = if led.mode == LedMode::Off {
                        0.0
                    } else {
                        0.35 + 0.065 * led.brightness as f32
                    };
                    let bar = if led.mode.has_color() {
                        DecoratedBox::new()
                            .class("glow-bar")
                            .style("background-color", Color::from_srgb(led.color.r, led.color.g, led.color.b, 1.0))
                    } else {
                        DecoratedBox::new().class("glow-bar glow-rainbow")
                    };
                    bar.style("opacity", StyleValue::Number(opacity))
                })
                .child(move || {
                    let led = ctx.sink.edit.get().led;
                    Text::new(led.mode.label()).class("preview-caption")
                }),
        )
        .class("card preview-card")
}

fn params(ctx: AppCtx) -> impl Widget {
    reactive_box(move || {
        let led = ctx.sink.edit.get().led;
        if led.mode == LedMode::Off {
            return Box::new(Text::new("Подсветка выключена — выберите эффект, чтобы настроить его.").class("muted"));
        }
        let mut col = Column::new().gap(18.0).cross_axis_alignment(CrossAxisAlignment::Stretch);

        if led.mode.has_color() {
            let mut palette = Row::new().gap(8.0).cross_axis_alignment(CrossAxisAlignment::Center);
            for col_rgb in PALETTE {
                let c = ctx.clone();
                let cls = if led.color == col_rgb { "color-swatch color-swatch-on" } else { "color-swatch" };
                palette = palette.child(
                    GestureDetector::new()
                        .on_click(move || c.edit(|cfg| cfg.led.color = col_rgb))
                        .cursor(CursorIcon::Pointer)
                        .child(swatch_dot(col_rgb.r, col_rgb.g, col_rgb.b, cls)),
                );
            }
            let c_pick = ctx.clone();
            palette = palette.child(
                ColorPicker::new()
                    .color(ColorValue::new(led.color.r, led.color.g, led.color.b))
                    .width(150.0)
                    .on_change(move |v| c_pick.edit(|cfg| cfg.led.color = Rgb::new(v.r, v.g, v.b))),
            );
            col = col.child(
                Column::new()
                    .gap(10.0)
                    .child(labeled("Цвет", led.color.to_hex()))
                    .child(palette),
            );
        } else {
            col = col.child(Text::new("Этот эффект переливается всеми цветами — выбор цвета не нужен.").class("muted"));
        }

        if led.mode.has_speed() {
            let c = ctx.clone();
            col = col.child(level_slider("Скорость", led.speed, move |v| c.edit(|cfg| cfg.led.speed = v)));
        }
        let c = ctx.clone();
        col = col.child(level_slider("Яркость", led.brightness, move |v| c.edit(|cfg| cfg.led.brightness = v)));
        Box::new(col)
    })
}

fn level_slider(label: &str, value: u8, mut set: impl FnMut(u8) + Send + 'static) -> impl Widget {
    Column::new()
        .gap(8.0)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(labeled(label, format!("{value} / {LEVELS}")))
        .child(
            Slider::new()
                .value(value as f32)
                .range(1.0, LEVELS as f32)
                .step(1.0)
                .on_change(move |v| set(v.round() as u8))
                .class("wide-slider"),
        )
}

fn extra(ctx: AppCtx) -> impl Widget {
    reactive(move || {
        let on = ctx.sink.edit.get().led_off_moving;
        let c = ctx.clone();
        setting_row(
            "Гасить подсветку при движении",
            "Экономит заряд: подсветка выключается, пока мышь двигается.",
            Box::new(Toggle::new().on(on).on_change(move |v| c.edit(|cfg| cfg.led_off_moving = v))),
        )
    })
}
