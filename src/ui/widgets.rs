//! Общие элементы страниц.

use super::icons;
use crate::worker::Link;
use syngui::prelude::*;
use syngui::widgets::*;

/// Заголовок страницы.
pub fn page_header(title: &str, subtitle: &str) -> impl Widget {
    Column::new()
        .gap(4.0)
        .child(Text::new(title).class("page-title"))
        .child(Text::new(subtitle).class("page-sub"))
}

/// Карточка-секция с заголовком.
pub fn card<M>(title: &str, hint: &str, body: impl IntoWidget<M>) -> impl Widget {
    let mut head = Column::new().gap(3.0).child(Text::new(title).class("card-title"));
    if !hint.is_empty() {
        head = head.child(Text::new(hint).class("card-hint"));
    }
    DecoratedBox::new()
        .child(
            Column::new()
                .gap(16.0)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .child(head)
                .child(body),
        )
        .class("card")
}

/// Строка «название + пояснение … контрол».
pub fn setting_row(title: &str, desc: &str, control: Box<dyn Widget>) -> impl Widget {
    Row::new()
        .gap(16.0)
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .child(
            Column::new()
                .gap(3.0)
                .child(Text::new(title).class("row-title"))
                .child(Text::new(desc).class("row-desc"))
                .class("grow"),
        )
        .children(vec![control])
}

/// Подпись над контролом со значением справа.
pub fn labeled(label: &str, value: String) -> impl Widget {
    Row::new()
        .gap(8.0)
        .child(Text::new(label).class("field-label"))
        .child(DecoratedBox::new().class("grow"))
        .child(Text::new(value).class("field-value"))
}

/// Экран, пока профиль мыши не прочитан.
pub fn placeholder(link: Link) -> impl Widget {
    let (icon, title, hint, extra): (&str, &str, String, Option<String>) = match link {
        Link::Searching | Link::Ready => (
            icons::SEARCH,
            "Подключение к мыши…",
            "Читаю настройки из памяти мыши".into(),
            None,
        ),
        Link::NotFound => (
            icons::USB,
            "Приёмник не найден",
            "Вставьте 2.4G-приёмник или подключите мышь кабелем.\n\
             Без мыши интерфейс можно посмотреть так:  ardor_mouse --simulate"
                .into(),
            None,
        ),
        Link::NoAccess(path) => (
            icons::LOCK,
            "Нет доступа к устройству",
            format!("У пользователя нет прав на {path}. Установите udev-правило и переподключите приёмник:"),
            Some(
                "sudo cp 99-ardor-mouse.rules /etc/udev/rules.d/\n\
                 sudo udevadm control --reload-rules && sudo udevadm trigger"
                    .into(),
            ),
        ),
        Link::Asleep => (
            icons::SLEEP,
            "Мышь спит",
            "Приёмник на связи, но мышь не отвечает. Пошевелите мышью или нажмите любую кнопку — \
             настройки загрузятся автоматически."
                .into(),
            None,
        ),
        Link::Error(e) => (icons::ERROR, "Ошибка связи", e, None),
    };
    let mut col = Column::new()
        .gap(12.0)
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .child(
            DecoratedBox::new()
                .child(Center::new().child(Icon::new(icon).class("ph-icon")))
                .class("ph-badge"),
        )
        .child(Text::new(title).class("ph-title"))
        .child(Text::new(hint).class("ph-hint"));
    if let Some(code) = extra {
        col = col.child(DecoratedBox::new().child(Text::new(code).selectable(true).class("code")).class("code-box"));
    }
    Center::new().child(DecoratedBox::new().child(col).class("ph-card"))
}

/// Цветной кружок.
pub fn swatch_dot(r: u8, g: u8, b: u8, class: &str) -> impl Widget {
    DecoratedBox::new()
        .class(class)
        .style("background-color", Color::from_srgb(r, g, b, 1.0))
}

/// Реактивный участок: пересобирается при изменении прочитанных сигналов.
pub fn reactive<W: Widget + 'static>(f: impl Fn() -> W + Send + Sync + 'static) -> Reactive {
    Reactive::new(move || vec![Box::new(f()) as Box<dyn Widget>])
}

/// То же для ветвлений с разными типами виджетов.
pub fn reactive_box(f: impl Fn() -> Box<dyn Widget> + Send + Sync + 'static) -> Reactive {
    Reactive::new(move || vec![f()])
}
