//! ardor_mouse — настройка мышей ARDOR GAMING на контроллере JM03 (Compx) в Linux:
//! Edge Air Ultra (PMW3370) и Rukh (PAW3950).
//!
//! `ardor_mouse`                       — работа с реальной мышью через /dev/hidraw*;
//! `ardor_mouse --simulate [edge|rukh]` — демо-режим без устройства (по умолчанию Edge);
//! `ardor_mouse --read-only`           — в мышь уходят только команды чтения;
//! `ardor_mouse --dump FILE`           — сохранить сырой образ профиля, прочитанный первым;
//! `ardor_mouse --lang en`             — язык интерфейса на этот запуск;
//! `ardor_mouse --page N`              — открыть раздел (0 DPI, 1 подсветка, 2 кнопки, 3 сенсор);
//! `ardor_mouse --screenshot OUT.png`  — снимок окна без экрана (с `--simulate`), `--frames N`
//!                                       — серия кадров анимации (`{n}` в имени);
//! `ardor_mouse --probe [--write-test]` — проверка мыши в терминале (чтение, сверка, запись с откатом);
//! `ARDOR_TRACE=1`                     — печатать пакеты обмена в stderr.

mod probe;
mod protocol;
mod settings;
mod transport;
mod ui;
mod worker;

use protocol::model::Model;
use std::time::Duration;
use syngui::prelude::*;
use syngui::text::icon_fonts::material;
use ui::app::{build_root, AppCtx};
use worker::Options;

const STYLES: &str = include_str!("../styles/app.mss");
const ICON: &[u8] = include_bytes!("../assets/icon/ardor-mouse-256.png");

/// Каталоги переводов; исходный язык строк в коде — русский.
pub const CATALOGS: &[&str] = &[
    include_str!("../i18n/ru.lang"),
    include_str!("../i18n/en.lang"),
    include_str!("../i18n/uk.lang"),
    include_str!("../i18n/kk.lang"),
    include_str!("../i18n/de.lang"),
];

/// Значение после флага `name`, если оно не начинается с `--`.
fn arg_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    let i = args.iter().position(|a| a == name)?;
    args.get(i + 1).map(String::as_str).filter(|v| !v.starts_with("--"))
}

fn has_flag(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if has_flag(&args, "--help") || has_flag(&args, "-h") {
        print!("{}", HELP);
        return;
    }

    syngui::i18n::set_source_language("ru");
    syngui::i18n::register_catalogs(CATALOGS);
    let prefs = settings::Settings::load();
    let lang = arg_value(&args, "--lang")
        .map(String::from)
        .or(prefs.language.clone())
        .unwrap_or_else(|| syngui::i18n::system_language().to_string());
    syngui::i18n::set_language(lang.as_str());

    let simulate = has_flag(&args, "--simulate").then(|| {
        arg_value(&args, "--simulate").and_then(Model::from_arg).unwrap_or(Model::EdgeAirUltra)
    });
    if has_flag(&args, "--probe") {
        std::process::exit(probe::run(simulate, has_flag(&args, "--write-test")));
    }
    let opts = Options {
        simulate,
        read_only: has_flag(&args, "--read-only"),
        dump: arg_value(&args, "--dump").map(Into::into),
    };
    // Сигналы создаются один раз, до run().
    let ctx = AppCtx::new(opts, prefs);
    if let Some(n) = arg_value(&args, "--page").and_then(|v| v.parse().ok()) {
        ctx.page.set(n);
    }

    if let Some(out) = arg_value(&args, "--screenshot") {
        std::process::exit(screenshot(ctx, &args, out));
    }

    App::new()
        .title("ARDOR GAMING")
        .app_id("ardor-mouse")
        .with_window_icon_png(ICON)
        .size(1220, 820)
        .min_size(1040, 700)
        .frameless()
        .transparent(true)
        .background(Color::from_srgb(0, 0, 0, 0.0))
        .with_icon_font(material::FONT_DATA)
        .with_styles_str(STYLES)
        .with_window_state(ctx.window)
        .run(move |_| {
            ctx.start_worker();
            Box::new(build_root(ctx.clone()))
        });
}

/// `--screenshot OUT`: окно с симулятором, нарисованное без экрана.
fn screenshot(ctx: AppCtx, args: &[String], out: &str) -> i32 {
    let model = ctx.simulated_model().unwrap_or(Model::EdgeAirUltra);
    ctx.preload_simulator(model);
    // `--demo-led N` — эффект подсветки для снимка (индекс в LedMode::ALL)
    if let Some(m) = arg_value(args, "--demo-led").and_then(|v| v.parse::<usize>().ok()) {
        if let Some(&mode) = protocol::led::LedMode::ALL.get(m) {
            ctx.edit(|c| c.led.mode = mode);
            ctx.sink.snapshot.set(Some(ctx.sink.edit.get_untracked()));
        }
    }
    let (w, h) = arg_value(args, "--size")
        .and_then(|s| s.split_once('x'))
        .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
        .unwrap_or((1220, 820));
    let scale = arg_value(args, "--scale").and_then(|v| v.parse().ok()).unwrap_or(1.0);
    let frames = arg_value(args, "--frames").and_then(|v| v.parse().ok()).unwrap_or(1);
    let interval = arg_value(args, "--interval").and_then(|v| v.parse().ok()).unwrap_or(80);
    let opts = syngui::shot::ShotOptions {
        frames,
        interval: Duration::from_millis(interval),
        settle: Duration::from_millis(600),
        ..Default::default()
    };
    let res = syngui::shot::render_png_with(
        move || Box::new(build_root(ctx)),
        STYLES,
        (w, h),
        scale,
        out,
        opts,
    );
    match res {
        Ok((w, h)) => {
            println!("{out}: {w}x{h}");
            0
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

const HELP: &str = "\
ardor-mouse — configure ARDOR GAMING mice (Edge Air Ultra, Rukh)

  --simulate [edge|rukh]  demo mode with a simulated mouse
  --read-only             send only read commands to the mouse
  --dump FILE             save the raw profile image read first
  --lang TAG              interface language for this run (ru, en, uk, kk, de)
  --page N                open a section: 0 DPI, 1 lighting, 2 buttons, 3 sensor
  --probe [--write-test]  check the mouse from the terminal: read and verify the
                          profile; --write-test also writes and reverts debounce
  --screenshot OUT.png    render the window off-screen (with --simulate);
                          --size WxH, --scale S, --frames N, --interval MS
  ARDOR_TRACE=1           print every packet exchanged with the mouse
";

#[cfg(test)]
mod tests {
    #[test]
    fn catalogs_parse() {
        for src in super::CATALOGS {
            let c = syngui::i18n::Catalog::parse(src).expect("каталог разбирается");
            let _ = c;
        }
    }
}
