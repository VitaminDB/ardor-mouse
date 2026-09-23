//! ardor_mouse — настройка мыши ARDOR GAMING Edge Air Ultra (PMW3370, JM03) в Linux.
//!
//! `ardor_mouse`            — работа с реальной мышью через /dev/hidraw*;
//! `ardor_mouse --simulate` — демо-режим без устройства;
//! `ARDOR_TRACE=1`          — печатать пакеты обмена в stderr.

mod protocol;
mod transport;
mod ui;
mod worker;

use syngui::prelude::*;
use syngui::text::icon_fonts::material;
use ui::app::{build_root, AppCtx};

const STYLES: &str = include_str!("../styles/app.mss");
const ICON: &[u8] = include_bytes!("../assets/icon/ardor-mouse-256.png");

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let simulate = args.iter().any(|a| a == "--simulate");
    // Сигналы создаются один раз, до run().
    let ctx = AppCtx::new(simulate);
    // `--page N` — открыть раздел N (0 DPI, 1 подсветка, 2 кнопки, 3 сенсор).
    if let Some(n) = args.iter().position(|a| a == "--page").and_then(|i| args.get(i + 1)?.parse().ok()) {
        ctx.page.set(n);
    }

    App::new()
        .title("ARDOR Edge Air Ultra")
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
