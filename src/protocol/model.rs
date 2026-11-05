//! Поддерживаемые мыши. Протокол JM03 (Compx) и карта EEPROM у них общие —
//! здесь одной таблицей [`Spec`] собрано только то, чем модели отличаются.

use super::dpi::Sensor;
use super::eeprom::PollingRate;
use super::packet::{REPORT_ID, RESPONSE_ID};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Model {
    /// ARDOR GAMING Edge Air Ultra — 25a7:fa7c / fa7b.
    EdgeAirUltra,
    /// ARDOR GAMING Rukh — 3554:f53e / f53d.
    Rukh,
}

/// Каким HID-репортом уходит команда.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandReport {
    /// Feature report 0x08 — `ioctl(HIDIOCSFEATURE)`, как `HidD_SetFeature`.
    Feature,
    /// Output report 0x08 — `write()`, как `WriteFile` в `HIDUsb.dll` утилиты Rukh.
    Output,
}

/// Вариант высоты отрыва: значение в EEPROM 0x0A и высота в десятых мм.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lod {
    pub hw: u8,
    pub tenths_mm: u8,
}

/// Всё, чем мышь отличается от соседей по протоколу.
#[derive(Debug)]
pub struct Spec {
    pub name: &'static str,
    pub vid: u16,
    /// 2.4G-приёмник.
    pub pid_dongle: u16,
    /// Проводное подключение. У Rukh — `M_PID` из `Config.ini` утилиты; на
    /// проверенном экземпляре кабель только заряжает и на шине не появляется.
    pub pid_wired: u16,
    pub command: CommandReport,
    /// Report ID ответа: у Edge — отдельный input report 0x09, у Rukh ответ
    /// приходит input-репортом с тем же ID 0x08, что и запрос.
    pub response_id: u8,
    pub sensor: Sensor,
    /// Ступеней DPI в интерфейсе: у Rukh утилита даёт максимум 6
    /// (`DPIMaxGrade=6`), хотя таблица в EEPROM на 8.
    pub max_dpi_stages: u8,
    /// Подсветка корпуса (EEPROM 0xA0). У Rukh её нет (`DisplayLight=0`) —
    /// участок не разбирается и не пишется.
    pub body_led: bool,
    /// Частоты опроса. Rukh — до 2000 Гц (`REPORT_RATE.R_2000` утилиты).
    pub polling_rates: &'static [PollingRate],
    /// Высота отрыва. У PAW3950 утилита Rukh добавляет 0,7 мм
    /// (`customComboBox_LOD`: 3/1/2 в языковом файле).
    pub lod: &'static [Lod],
    /// Motion sync (0xAB): `SUPPORT_MOTION_SYNC 3395,3950` в `driver_sensor.h`.
    pub motion_sync: bool,
    /// Режим сенсора LP/HP (0xB9). В `driver_sensor.h` он есть и у 3370, но
    /// утилита Edge поле не показывает и на Edge оно не проверено.
    pub sensor_mode: bool,
    /// Писать таблицы (DPI, цвета, кнопки) по одной изменённой 4-байтной
    /// записи, как `CS_ProtocolDataCompareUpdate` в `HIDUsb.dll` Rukh. Edge
    /// проверен на записи участка целиком — его поведение не меняется.
    pub per_record_writes: bool,
    /// Сколько байт профиля читать — как оригинальная утилита
    /// (`OemDrv.exe` — 0xB5, `HIDUsb.dll` `ReadAllFlashData` — 0xC0).
    pub profile_len: usize,
}

const LOD_1_2: &[Lod] = &[Lod { hw: 1, tenths_mm: 10 }, Lod { hw: 2, tenths_mm: 20 }];
const LOD_07_1_2: &[Lod] =
    &[Lod { hw: 3, tenths_mm: 7 }, Lod { hw: 1, tenths_mm: 10 }, Lod { hw: 2, tenths_mm: 20 }];

static EDGE: Spec = Spec {
    name: "Edge Air Ultra",
    vid: 0x25A7,
    pid_dongle: 0xFA7C,
    pid_wired: 0xFA7B,
    command: CommandReport::Feature,
    response_id: RESPONSE_ID,
    sensor: Sensor::Pmw3370,
    max_dpi_stages: 8,
    body_led: true,
    polling_rates: &[PollingRate::Hz125, PollingRate::Hz250, PollingRate::Hz500, PollingRate::Hz1000],
    lod: LOD_1_2,
    motion_sync: false,
    sensor_mode: false,
    per_record_writes: false,
    profile_len: 0xB5,
};

static RUKH: Spec = Spec {
    name: "Rukh",
    vid: 0x3554,
    pid_dongle: 0xF53E,
    pid_wired: 0xF53D,
    command: CommandReport::Output,
    response_id: REPORT_ID,
    sensor: Sensor::Paw3950,
    max_dpi_stages: 6,
    body_led: false,
    polling_rates: &PollingRate::ALL,
    lod: LOD_07_1_2,
    motion_sync: true,
    sensor_mode: true,
    per_record_writes: true,
    profile_len: 0xC0,
};

impl Model {
    pub const ALL: [Model; 2] = [Model::EdgeAirUltra, Model::Rukh];

    pub fn spec(self) -> &'static Spec {
        match self {
            Model::EdgeAirUltra => &EDGE,
            Model::Rukh => &RUKH,
        }
    }

    pub fn name(self) -> &'static str {
        self.spec().name
    }

    pub fn sensor(self) -> Sensor {
        self.spec().sensor
    }

    pub fn from_usb(vid: u16, pid: u16) -> Option<Model> {
        Self::ALL.into_iter().find(|m| {
            let s = m.spec();
            s.vid == vid && (pid == s.pid_dongle || pid == s.pid_wired)
        })
    }

    /// `--simulate edge|rukh`.
    pub fn from_arg(s: &str) -> Option<Model> {
        match s.to_ascii_lowercase().as_str() {
            "edge" | "edge-air-ultra" => Some(Model::EdgeAirUltra),
            "rukh" => Some(Model::Rukh),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usb_ids() {
        assert_eq!(Model::from_usb(0x25A7, 0xFA7C), Some(Model::EdgeAirUltra));
        assert_eq!(Model::from_usb(0x25A7, 0xFA7B), Some(Model::EdgeAirUltra));
        assert_eq!(Model::from_usb(0x3554, 0xF53E), Some(Model::Rukh));
        assert_eq!(Model::from_usb(0x3554, 0xF53D), Some(Model::Rukh));
        // чужие мыши на тех же VID (VXE / ATK — 3554:f58x)
        assert_eq!(Model::from_usb(0x3554, 0xF58A), None);
        assert_eq!(Model::from_usb(0x25A7, 0xF53E), None);
    }

    #[test]
    fn specs_are_consistent() {
        for m in Model::ALL {
            let s = m.spec();
            assert!(s.max_dpi_stages as usize <= super::super::eeprom::DPI_SLOTS);
            assert!(s.polling_rates.contains(&PollingRate::Hz1000));
            assert!(s.lod.iter().any(|l| l.hw == 1), "заводской LOD 1 мм");
            // поля за пределами читаемого профиля не используются
            if s.sensor_mode {
                assert!(s.profile_len > super::super::eeprom::addr::SENSOR_MODE as usize + 1);
            }
        }
    }
}
