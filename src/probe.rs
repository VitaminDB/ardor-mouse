//! `--probe`: проверка мыши из терминала без окна — для отладки и для
//! проверки новой версии на железе.
//!
//! * чтение профиля, разбор, сверка: участки, собранные из разобранного
//!   профиля, должны совпасть с прочитанными байтами;
//! * `--write-test` — обратимая запись: debounce меняется на 1 мс, профиль
//!   перечитывается, затем возвращается исходное значение, и образ памяти
//!   должен совпасть с исходным байт в байт.

use crate::protocol::eeprom::DeviceConfig;
use crate::protocol::packet::to_hex;
use crate::transport::hidraw::HidrawTransport;
use crate::transport::sim::SimTransport;
use crate::transport::{Device, Result};
use crate::protocol::model::Model;

pub fn run(simulate: Option<Model>, write_test: bool) -> i32 {
    let dev = match simulate {
        Some(m) => Ok(Device::new(Box::new(SimTransport::with_model(m)))),
        None => HidrawTransport::open().map(|t| Device::new(Box::new(t))),
    };
    let mut dev = match dev {
        Ok(d) => d,
        Err(e) => {
            eprintln!("ошибка: {e}");
            return 2;
        }
    };
    match probe(&mut dev, write_test) {
        Ok(true) => 0,
        Ok(false) => 1,
        Err(e) => {
            eprintln!("ошибка: {e}");
            2
        }
    }
}

fn probe(dev: &mut Device, write_test: bool) -> Result<bool> {
    let model = dev.model();
    println!("устройство: {} ({})", dev.name(), model.name());
    println!("связь с мышью: {}", if dev.is_connected()? { "есть" } else { "нет (спит?)" });
    let b = dev.battery()?;
    println!("батарея: {}%{}", b.percent, if b.charging { ", заряжается" } else { "" });

    let mem = dev.read_profile()?;
    let (cfg, warn) = DeviceConfig::decode(&mem, model);
    println!("профиль: {} байт", mem.len());
    for chunk in mem.chunks(16).enumerate() {
        println!("  {:02X}: {}", chunk.0 * 16, to_hex(chunk.1));
    }
    print_config(&cfg);
    let mut ok = true;
    if !warn.is_empty() {
        println!("ПРЕДУПРЕЖДЕНИЯ: {}", warn.join(", "));
        ok = false;
    }
    for r in cfg.regions() {
        let a = r.addr as usize;
        if mem.get(a..a + r.data.len()) != Some(&r.data[..]) {
            println!("НЕ СОВПАЛО: участок «{}» @0x{:02X}", r.name, r.addr);
            ok = false;
        }
    }
    println!("сверка участков: {}", if ok { "байты воспроизводятся 1:1" } else { "есть расхождения" });

    if write_test && ok {
        ok &= reversible_write(dev, &cfg, &mem)?;
    }
    Ok(ok)
}

fn reversible_write(dev: &mut Device, orig: &DeviceConfig, mem: &[u8]) -> Result<bool> {
    let mut ok = true;
    // одиночная пара (debounce) и таблица DPI — запись в несколько пакетов
    let sensor = orig.model.sensor();
    let tests: [(&str, Box<dyn Fn(&mut DeviceConfig)>); 2] = [
        ("debounce", Box::new(|c| c.debounce_ms = if c.debounce_ms < 30 { c.debounce_ms + 1 } else { c.debounce_ms - 1 })),
        (
            "DPI ступени 1",
            Box::new(move |c| {
                let d = c.dpi[0].dpi;
                c.dpi[0].dpi = sensor.snap(if d + 100 <= sensor.max() { d + 100 } else { d - 100 });
            }),
        ),
    ];
    for (name, change) in tests {
        ok &= reversible(dev, orig, mem, name, change.as_ref())?;
    }
    Ok(ok)
}

fn reversible(
    dev: &mut Device,
    orig: &DeviceConfig,
    mem: &[u8],
    name: &str,
    change: &dyn Fn(&mut DeviceConfig),
) -> Result<bool> {
    let model = dev.model();
    let mut test = orig.clone();
    change(&mut test);
    let regions = test.changed_regions(orig);
    let bytes: usize = regions.iter().map(|r| r.data.len()).sum();
    println!("запись «{name}»: {} участок(ов), {bytes} байт", regions.len());
    dev.write_regions(&regions, |_, _, _| {})?;
    let (back, _) = DeviceConfig::decode(&dev.read_profile()?, model);
    let written = back == test;
    println!("  прочитано обратно: {}", if written { "совпало с записанным" } else { "НЕ СОВПАЛО" });

    dev.write_regions(&orig.changed_regions(&test), |_, _, _| {})?;
    let same = dev.read_profile()? == mem;
    println!("  откат: образ памяти {}", if same { "совпал с исходным байт в байт" } else { "ОТЛИЧАЕТСЯ от исходного" });
    Ok(written && same)
}

fn print_config(c: &DeviceConfig) {
    println!("  частота опроса: {} Гц", c.polling.hz());
    let stages: Vec<String> = c.dpi[..c.dpi_count as usize]
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let star = if i == c.dpi_active as usize { "*" } else { "" };
            format!("{}{star} {}", l.dpi, l.color.to_hex())
        })
        .collect();
    println!("  DPI: {}", stages.join(" | "));
    println!("  LOD: {}, debounce: {} мс, сон: {} с", c.lod, c.debounce_ms, c.sleep_x10s as u32 * 10);
    println!("  angle snapping: {}, ripple: {}", c.angle_snap, c.ripple);
    if c.model.spec().body_led {
        println!(
            "  подсветка: {:?} {} скорость {} яркость {}, гасить при движении: {}",
            c.led.mode,
            c.led.color.to_hex(),
            c.led.speed,
            c.led.brightness,
            c.led_off_moving
        );
    }
    if c.model.spec().motion_sync {
        println!("  motion sync: {}", c.motion_sync);
    }
    if c.model.spec().sensor_mode {
        println!("  режим сенсора: {}", if c.sensor_hp { "HP" } else { "LP" });
    }
    let buttons: Vec<String> = crate::protocol::buttons::PHYSICAL
        .iter()
        .map(|&(_, slot)| format!("{:?}", c.buttons[slot]))
        .collect();
    println!("  кнопки: {}", buttons.join(", "));
}
