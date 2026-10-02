//! ardor_mouse — настройка мышей ARDOR GAMING на контроллере JM03 (Compx) в Linux:
//! Edge Air Ultra (PMW3370) и Rukh (PAW3950).
//!
//! `ardor_mouse`                 — работа с реальной мышью через /dev/hidraw*;
//! `ardor_mouse --simulate [rukh]` — демо-режим без устройства (по умолчанию Edge);
//! `ardor_mouse --read-only`     — в мышь уходят только команды чтения;
//! `ardor_mouse --dump FILE`     — сохранить сырой образ профиля, прочитанный первым;
//! `ARDOR_TRACE=1`               — печатать пакеты обмена в stderr.

mod protocol;
mod transport;
mod ui;
mod worker;

use protocol::model::Model;
use syngui::prelude::*;
use syngui::text::icon_fonts::material;
use ui::app::{build_root, AppCtx};
use worker::Options;

const STYLES: &str = include_str!("../styles/app.mss");
const ICON: &[u8] = include_bytes!("../assets/icon/ardor-mouse-256.png");

/// Значение после флага `name`, если оно не начинается с `--`.
fn arg_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    let i = args.iter().position(|a| a == name)?;
    args.get(i + 1).map(String::as_str).filter(|v| !v.starts_with("--"))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let simulate = args.iter().any(|a| a == "--simulate").then(|| {
        match arg_value(&args, "--simulate") {
            Some("rukh") => Model::Rukh,
            _ => Model::EdgeAirUltra,
        }
    });
    let opts = Options {
        simulate,
        read_only: args.iter().any(|a| a == "--read-only"),
        dump: arg_value(&args, "--dump").map(Into::into),
    };
    // Сигналы создаются один раз, до run().
    let ctx = AppCtx::new(opts);
    // `--page N` — открыть раздел N (0 DPI, 1 подсветка, 2 кнопки, 3 сенсор).
    if let Some(n) = args.iter().position(|a| a == "--page").and_then(|i| args.get(i + 1)?.parse().ok()) {
        ctx.page.set(n);
    }

    App::new()
        .title("ARDOR GAMING")
        .app_id("ardor-mouse")
        .with_window_icon_png(ICON)
        .size(1180, 800)
        .min_size(1000, 680)
        .with_icon_font(material::FONT_DATA)
        .with_styles_str(STYLES)
        .run(move |_| {
            ctx.start_worker();
            Box::new(build_root(ctx.clone()))
        });
}
