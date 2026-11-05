//! Живая мышь: фото из утилиты производителя и холст поверх него, на котором
//! покадрово рисуется свечение — эффект подсветки, цвет ступени DPI, метки
//! кнопок. Как в Lenovo Legion Space: мышь в приложении светится так же, как
//! будет светиться настоящая.
//!
//! Координаты зон заданы в долях картинки (0…1), поэтому не зависят от
//! размера, с которым она показана.

use crate::protocol::led::{LedConfig, LedMode, Rgb, LEVELS};
use crate::protocol::model::Model;
use std::f32::consts::TAU;
use syngui::core::canvas::CanvasContext;
use syngui::prelude::*;

/// Картинка мыши: исходник, кадр (пиксели исходника) и размер на экране
/// (совпадает с MSS-классом).
#[derive(Clone, Copy)]
pub struct Art {
    key: &'static str,
    png: &'static [u8],
    class: &'static str,
    /// Кадр `x0, y0, x1, y1` в пикселях исходника — без пустых полей.
    crop: [f32; 4],
    w: f32,
    h: f32,
}

impl Art {
    /// Точка исходника → точка на экране.
    fn map(&self, x: f32, y: f32) -> (f32, f32) {
        let [x0, y0, x1, y1] = self.crop;
        ((x - x0) / (x1 - x0) * self.w, (y - y0) / (y1 - y0) * self.h)
    }

    /// Масштаб исходник → экран.
    fn scale(&self) -> f32 {
        self.w / (self.crop[2] - self.crop[0])
    }
}

const EDGE_LED: &[u8] = include_bytes!("../../assets/skins/0806/mouse_led.png");
const EDGE_SCHEME: &[u8] = include_bytes!("../../assets/skins/0806/mouse_nr.png");
const RUKH: &[u8] = include_bytes!("../../assets/skins/rukh/mouse.png");
const RUKH_SCHEME: &[u8] = include_bytes!("../../assets/skins/rukh/mouse_nr.png");

/// Кадры: Edge 3/4 — целиком, Edge сверху и Rukh — без пустых полей.
const EDGE_LED_CROP: [f32; 4] = [0.0, 0.0, 400.0, 250.0];
const EDGE_SCHEME_CROP: [f32; 4] = [140.0, 70.0, 400.0, 440.0];
const RUKH_CROP: [f32; 4] = [36.0, 62.0, 404.0, 316.0];

/// Где показывается мышь.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Place {
    /// Боковая панель.
    Hero,
    /// Превью подсветки и индикатора DPI.
    Preview,
    /// Схема кнопок с номерами.
    Scheme,
}

pub fn art(model: Model, place: Place) -> Art {
    let a = |key, png, class, crop, w, h| Art { key, png, class, crop, w, h };
    match (model, place) {
        (Model::EdgeAirUltra, Place::Hero) => a("edge-led", EDGE_LED, "art-hero-edge", EDGE_LED_CROP, 200.0, 125.0),
        (Model::EdgeAirUltra, Place::Preview) => {
            a("edge-led", EDGE_LED, "art-preview-edge", EDGE_LED_CROP, 384.0, 240.0)
        }
        (Model::EdgeAirUltra, Place::Scheme) => {
            a("edge-scheme", EDGE_SCHEME, "art-scheme-edge", EDGE_SCHEME_CROP, 260.0, 370.0)
        }
        (Model::Rukh, Place::Hero) => a("rukh", RUKH, "art-hero-rukh", RUKH_CROP, 196.0, 135.0),
        (Model::Rukh, Place::Preview) => a("rukh", RUKH, "art-preview-rukh", RUKH_CROP, 348.0, 240.0),
        (Model::Rukh, Place::Scheme) => {
            a("rukh-scheme", RUKH_SCHEME, "art-scheme-rukh", RUKH_CROP, 420.0, 290.0)
        }
    }
}

/// Светящаяся зона: центр и полуоси в пикселях исходной картинки.
#[derive(Clone, Copy)]
struct Zone {
    x: f32,
    y: f32,
    rx: f32,
    ry: f32,
}

const fn z(x: f32, y: f32, rx: f32, ry: f32) -> Zone {
    Zone { x, y, rx, ry }
}

/// Зоны подсветки Edge Air Ultra на `mouse_led.png` (400×250) в порядке
/// «бегущего огня»: колесо → соты от носа к хвосту → задняя полоса.
const EDGE_ZONES: [Zone; 12] = [
    z(100.0, 112.0, 20.0, 34.0),
    z(232.0, 30.0, 12.0, 9.0),
    z(262.0, 168.0, 14.0, 12.0),
    z(276.0, 52.0, 12.0, 10.0),
    z(300.0, 140.0, 16.0, 14.0),
    z(306.0, 92.0, 14.0, 12.0),
    z(330.0, 168.0, 14.0, 12.0),
    z(340.0, 112.0, 16.0, 14.0),
    z(356.0, 80.0, 12.0, 10.0),
    z(352.0, 136.0, 12.0, 10.0),
    z(330.0, 60.0, 12.0, 9.0),
    z(370.0, 148.0, 18.0, 12.0),
];

/// Индикатор DPI: у Edge — колесо, у Rukh — светодиод у кнопки DPI.
fn dpi_zone(model: Model) -> Zone {
    match model {
        Model::EdgeAirUltra => EDGE_ZONES[0],
        Model::Rukh => z(214.0, 96.0, 22.0, 14.0),
    }
}

/// Номера кнопок на схеме (пиксели исходника), порядок — [`crate::protocol::buttons::PHYSICAL`].
fn markers(model: Model) -> [(f32, f32); 6] {
    match model {
        Model::EdgeAirUltra => {
            [(220.0, 138.0), (333.0, 130.0), (272.0, 103.0), (168.0, 203.0), (172.0, 270.0), (283.0, 210.0)]
        }
        Model::Rukh => {
            [(205.0, 168.0), (72.0, 150.0), (125.0, 186.0), (258.0, 197.0), (312.0, 160.0), (200.0, 112.0)]
        }
    }
}

/// Что рисуется поверх мыши.
#[derive(Clone, Copy, PartialEq)]
pub enum Scene {
    /// Эффект подсветки корпуса.
    Lighting(LedConfig),
    /// Индикатор ступени DPI: цвет и «вспышка» при смене ступени.
    Dpi(Rgb),
    /// Метки кнопок; `focus` — подсвеченная кнопка (наведение в списке).
    Buttons { focus: Option<usize> },
}

/// Мышь со свечением. `replay` — ключ, при смене которого анимация
/// начинается заново (вспышка при выборе ступени DPI).
pub fn mouse_view(model: Model, place: Place, scene: Scene, replay: u64) -> impl Widget {
    let a = art(model, place);
    let [x0, y0, x1, y1] = a.crop;
    let animated = !matches!(scene, Scene::Lighting(LedConfig { mode: LedMode::Off, .. }));
    Stack::new()
        .child(
            Image::from_bytes(a.key, a.png.to_vec())
                .crop(x0, y0, x1 - x0, y1 - y0)
                .fit(ImageFit::Contain)
                .class(a.class),
        )
        .child(
            Canvas::new(move |cx, t| draw(cx, model, scene, t, &a))
                .size(a.w, a.h)
                .animated(animated)
                .replay(replay),
        )
}

fn draw(cx: &mut CanvasContext, model: Model, scene: Scene, t: f32, a: &Art) {
    match scene {
        Scene::Lighting(led) => {
            if model.spec().body_led {
                lighting(cx, led, t, a);
            }
        }
        Scene::Dpi(rgb) => {
            let zone = dpi_zone(model);
            // вспышка 0,6 с при смене ступени, потом ровное дыхание
            let flash = (1.0 - t / 0.6).max(0.0);
            let calm = 0.75 + 0.2 * (t * 1.6).sin();
            glow(cx, zone, a, rgb, (calm + flash * 0.8).min(1.5) * 1.15, 1.0 + flash * 1.6);
        }
        Scene::Buttons { focus } => {
            let k = a.scale() / 0.727; // кольца рассчитаны на схему шириной 400 px
            for (i, (x, y)) in markers(model).into_iter().enumerate() {
                let (x, y) = a.map(x, y);
                ring(cx, x, y, k, t + i as f32 * 0.35, focus == Some(i));
            }
        }
    }
}

/// Эффект подсветки во времени `t` — по смыслу режимов утилиты.
fn lighting(cx: &mut CanvasContext, led: LedConfig, t: f32, a: &Art) {
    let bri = 0.35 + 0.65 * (led.brightness.clamp(1, LEVELS) as f32 / LEVELS as f32);
    // период цикла: уровень скорости 1 — медленно, 10 — быстро
    let period = 6.0 - 4.8 * (led.speed.clamp(1, LEVELS) - 1) as f32 / (LEVELS - 1) as f32;
    let phase = t / period;
    let n = EDGE_ZONES.len() as f32;
    for (i, &zone) in EDGE_ZONES.iter().enumerate() {
        let k = i as f32 / n;
        let (rgb, level) = match led.mode {
            LedMode::Off => return,
            LedMode::Steady => (led.color, 1.0),
            LedMode::Breathing => (led.color, breath(phase)),
            LedMode::Streaming => (hue(phase + k), 1.0),
            LedMode::Neon => (hue(phase * 0.5), 1.0),
            LedMode::Scrolling => {
                // яркое пятно бежит по зонам, за ним тянется хвост
                let d = (phase * 1.5 - k).rem_euclid(1.0);
                (led.color, (1.0 - d * 3.0).max(0.12))
            }
            LedMode::ColorBreathing => {
                const SEQ: [Rgb; 7] = [
                    Rgb::new(255, 0, 0),
                    Rgb::new(0, 255, 0),
                    Rgb::new(0, 0, 255),
                    Rgb::new(255, 255, 0),
                    Rgb::new(0, 255, 255),
                    Rgb::new(255, 0, 255),
                    Rgb::new(255, 255, 255),
                ];
                (SEQ[phase.floor() as usize % SEQ.len()], breath(phase))
            }
        };
        glow(cx, zone, a, rgb, level * bri, 1.0);
    }
}

/// Дыхание: плавный подъём и спад за период.
fn breath(phase: f32) -> f32 {
    let s = (phase * TAU).sin() * 0.5 + 0.5;
    s * s * 0.95 + 0.05
}

/// Цвет радуги по доле круга.
fn hue(h: f32) -> Rgb {
    let h = h.rem_euclid(1.0) * 6.0;
    let x = 1.0 - (h % 2.0 - 1.0).abs();
    let (r, g, b) = match h as u32 {
        0 => (1.0, x, 0.0),
        1 => (x, 1.0, 0.0),
        2 => (0.0, 1.0, x),
        3 => (0.0, x, 1.0),
        4 => (x, 0.0, 1.0),
        _ => (1.0, 0.0, x),
    };
    Rgb::new((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

/// Мягкое свечение: стопка эллипсов с растущей к центру плотностью.
fn glow(cx: &mut CanvasContext, zone: Zone, a: &Art, rgb: Rgb, level: f32, spread: f32) {
    if level <= 0.01 {
        return;
    }
    let (x, y) = a.map(zone.x, zone.y);
    let k = a.scale() * spread;
    let (rx, ry) = (zone.rx * k, zone.ry * k);
    const LAYERS: usize = 7;
    for i in 0..LAYERS {
        let f = 1.0 - i as f32 / LAYERS as f32; // 1 → снаружи
        let a = (0.05 + 0.11 * (1.0 - f)) * level;
        cx.set_color(Color::from_srgb(rgb.r, rgb.g, rgb.b, a.min(1.0)));
        cx.fill_polygon(&ellipse(x, y, rx * (0.35 + 0.65 * f) * 1.5, ry * (0.35 + 0.65 * f) * 1.5));
    }
    // светящееся ядро — почти белое в центре, как пересвеченный светодиод
    let core = |c: u8| (c as f32 + (255.0 - c as f32) * 0.45) as u8;
    cx.set_color(Color::from_srgb(core(rgb.r), core(rgb.g), core(rgb.b), (0.55 * level).min(0.9)));
    cx.fill_polygon(&ellipse(x, y, rx * 0.45, ry * 0.45));
}

/// Пульсирующее кольцо вокруг номера кнопки.
fn ring(cx: &mut CanvasContext, x: f32, y: f32, k: f32, t: f32, focus: bool) {
    let p = (t / if focus { 1.1 } else { 2.4 }).fract();
    let r = (13.0 + p * if focus { 22.0 } else { 12.0 }) * k;
    let a = (1.0 - p) * if focus { 0.9 } else { 0.35 };
    cx.set_stroke_width(if focus { 3.0 } else { 2.0 });
    cx.set_color(Color::from_srgb(255, 59, 78, a));
    cx.stroke_circle(x, y, r);
    if focus {
        cx.set_color(Color::from_srgb(255, 59, 78, 0.28));
        cx.fill_circle(x, y, 17.0 * k);
        cx.set_stroke_width(2.5);
        cx.set_color(Color::from_srgb(255, 255, 255, 0.95));
        cx.stroke_circle(x, y, 15.0 * k);
    }
}

fn ellipse(x: f32, y: f32, rx: f32, ry: f32) -> Vec<(f32, f32)> {
    const N: usize = 28;
    (0..N)
        .map(|i| {
            let a = i as f32 / N as f32 * TAU;
            (x + rx * a.cos(), y + ry * a.sin())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hue_wheel() {
        assert_eq!(hue(0.0), Rgb::new(255, 0, 0));
        assert_eq!(hue(1.0 / 3.0).g, 255);
        assert_eq!(hue(2.0 / 3.0).b, 255);
        assert_eq!(hue(1.0), hue(0.0));
    }

    #[test]
    fn breath_stays_in_range() {
        for i in 0..100 {
            let v = breath(i as f32 / 37.0);
            assert!((0.0..=1.0).contains(&v));
        }
    }

    #[test]
    fn markers_inside_frame() {
        for m in Model::ALL {
            let a = art(m, Place::Scheme);
            for (x, y) in markers(m) {
                let (x, y) = a.map(x, y);
                assert!((10.0..a.w - 10.0).contains(&x) && (10.0..a.h - 10.0).contains(&y), "{x} {y}");
            }
        }
    }

    #[test]
    fn frames_keep_aspect() {
        for m in Model::ALL {
            for p in [Place::Hero, Place::Preview, Place::Scheme] {
                let a = art(m, p);
                let [x0, y0, x1, y1] = a.crop;
                let (src, dst) = ((x1 - x0) / (y1 - y0), a.w / a.h);
                assert!((src - dst).abs() < 0.03, "{} {src} vs {dst}", a.class);
            }
        }
    }
}
