//! Поддерживаемые мыши. Протокол JM03 (Compx) и карта EEPROM у них общие —
//! здесь собрано только то, чем модели отличаются.

use super::dpi::Sensor;
use super::eeprom::PollingRate;
use super::packet::{REPORT_ID, RESPONSE_ID};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

impl Model {
    pub const ALL: [Model; 2] = [Model::EdgeAirUltra, Model::Rukh];

    pub fn name(self) -> &'static str {
        match self {
            Model::EdgeAirUltra => "Edge Air Ultra",
            Model::Rukh => "Rukh",
        }
    }

    pub fn vid(self) -> u16 {
        match self {
            Model::EdgeAirUltra => 0x25A7,
            Model::Rukh => 0x3554,
        }
    }

    /// Проводное подключение. У Rukh — `M_PID` из `Config.ini` утилиты; на
    /// проверенном экземпляре кабель только заряжает и на шине не появляется.
    pub fn pid_wired(self) -> u16 {
        match self {
            Model::EdgeAirUltra => 0xFA7B,
            Model::Rukh => 0xF53D,
        }
    }

    /// 2.4G-приёмник.
    pub fn pid_dongle(self) -> u16 {
        match self {
            Model::EdgeAirUltra => 0xFA7C,
            Model::Rukh => 0xF53E,
        }
    }

    pub fn from_usb(vid: u16, pid: u16) -> Option<Model> {
        Self::ALL
            .into_iter()
            .find(|m| m.vid() == vid && (pid == m.pid_dongle() || pid == m.pid_wired()))
    }

    pub fn command_report(self) -> CommandReport {
        match self {
            Model::EdgeAirUltra => CommandReport::Feature,
            Model::Rukh => CommandReport::Output,
        }
    }

    /// Report ID ответа: у Edge — отдельный input report 0x09, у Rukh ответ
    /// приходит input-репортом с тем же ID 0x08, что и запрос.
    pub fn response_id(self) -> u8 {
        match self {
            Model::EdgeAirUltra => RESPONSE_ID,
            Model::Rukh => REPORT_ID,
        }
    }

    pub fn sensor(self) -> Sensor {
        match self {
            Model::EdgeAirUltra => Sensor::Pmw3370,
            Model::Rukh => Sensor::Paw3950,
        }
    }

    /// Ступеней DPI в интерфейсе: у Rukh утилита даёт максимум 6
    /// (`DPIMaxGrade=6`), хотя таблица в EEPROM на 8.
    pub fn max_dpi_stages(self) -> u8 {
        match self {
            Model::EdgeAirUltra => 8,
            Model::Rukh => 6,
        }
    }

    /// Есть ли подсветка корпуса (EEPROM 0xA0). У Rukh её нет
    /// (`DisplayLight=0`) — участок не разбирается и не пишется.
    pub fn has_body_led(self) -> bool {
        match self {
            Model::EdgeAirUltra => true,
            Model::Rukh => false,
        }
    }

    /// Частоты опроса. Rukh — до 2000 Гц (характеристики ardor-gaming.com и
    /// `REPORT_RATE.R_2000` утилиты).
    pub fn polling_rates(self) -> &'static [PollingRate] {
        match self {
            Model::EdgeAirUltra => &PollingRate::ALL[..4],
            Model::Rukh => &PollingRate::ALL,
        }
    }

    /// Варианты высоты отрыва: (значение в EEPROM 0x0A, подпись). У PAW3950
    /// утилита Rukh добавляет 0,7 мм (`customComboBox_LOD`: 3/1/2 в языковом
    /// файле, для прочих сенсоров первый пункт удаляется).
    pub fn lod_options(self) -> &'static [(u8, &'static str)] {
        match self.sensor() {
            Sensor::Pmw3370 => &[(1, "1 мм"), (2, "2 мм")],
            Sensor::Paw3950 => &[(3, "0,7 мм"), (1, "1 мм"), (2, "2 мм")],
        }
    }

    /// Motion sync: в утилите Rukh — `SUPPORT_MOTION_SYNC 3395,3950`
    /// (`driver_sensor.h`), у PMW3370 его нет.
    pub fn has_motion_sync(self) -> bool {
        match self.sensor() {
            Sensor::Paw3950 => true,
            Sensor::Pmw3370 => false,
        }
    }

    /// Выбор режима сенсора LP/HP (EEPROM 0xB9). В `driver_sensor.h` утилиты
    /// Rukh `SUPPORT_SENSOR_MODE_SEL` есть и 3370, но утилита Edge этого поля
    /// не показывает и на Edge оно не проверено — включено только для Rukh.
    pub fn has_sensor_mode(self) -> bool {
        match self {
            Model::EdgeAirUltra => false,
            Model::Rukh => true,
        }
    }

    /// Писать таблицы (DPI, цвета, кнопки) по одной изменённой 4-байтной
    /// записи, как `CS_ProtocolDataCompareUpdate` в `HIDUsb.dll` Rukh. Edge Air
    /// Ultra проверен на записи участка целиком — его поведение не меняется.
    pub fn per_record_writes(self) -> bool {
        match self {
            Model::EdgeAirUltra => false,
            Model::Rukh => true,
        }
    }

    /// Сколько байт профиля читать: столько же, сколько оригинальная утилита
    /// (`OemDrv.exe` — 0xB5, `HIDUsb.dll` `ReadAllFlashData` — 0xC0).
    pub fn profile_len(self) -> usize {
        match self {
            Model::EdgeAirUltra => 0xB5,
            Model::Rukh => 0xC0,
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
}
