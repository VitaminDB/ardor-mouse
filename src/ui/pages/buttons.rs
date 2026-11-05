//! Страница назначения кнопок: схема мыши с живыми метками и список.

use crate::protocol::buttons::{ButtonAction, PHYSICAL};
use crate::ui::app::AppCtx;
use crate::ui::icons;
use crate::ui::mouse_view::{mouse_view, Place, Scene};
use crate::ui::widgets::{page_header, reactive};
use syngui::i18n::t;
use syngui::t;
use syngui::prelude::*;
use syngui::widgets::*;

pub fn view(ctx: AppCtx) -> impl Widget {
    let c_scheme = ctx.clone();
    Column::new()
        .gap(18.0)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(page_header(
            &t!("Кнопки"),
            &t!("Назначение шести кнопок мыши. Наведите на строку — кнопка подсветится на схеме."),
        ))
        .child(
            Row::new()
                .gap(18.0)
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .child(
                    DecoratedBox::new()
                        .child(reactive(move || {
                            let model = c_scheme.sink.edit.get().model;
                            let focus = c_scheme.focus_button.get();
                            mouse_view(model, Place::Scheme, Scene::Buttons { focus }, 0)
                        }))
                        .class("card scheme-card"),
                )
                .child(DecoratedBox::new().child(list(ctx)).class("grow")),
        )
        .class("page")
}

fn list(ctx: AppCtx) -> impl Widget {
    reactive(move || {
        let cfg = ctx.sink.edit.get();
        let snapshot = ctx.sink.snapshot.get_untracked();
        let mut col = Column::new().gap(10.0).cross_axis_alignment(CrossAxisAlignment::Stretch);
        for (n, (label, slot)) in PHYSICAL.into_iter().enumerate() {
            let cur = cfg.buttons[slot];
            let mut items: Vec<DropdownItem> =
                ButtonAction::CHOICES.into_iter().map(|a| DropdownItem::new(a.id(), a.label())).collect();
            if let ButtonAction::Other(_) = cur {
                items.push(DropdownItem::new(cur.id(), cur.label()));
            }
            let c = ctx.clone();
            let changed = snapshot.as_ref().is_some_and(|s| s.buttons[slot] != cur);
            let focus = ctx.focus_button;
            col = col.child(
                GestureDetector::new()
                    .on_hover_change(move |on| {
                        if on {
                            focus.set(Some(n));
                        } else if focus.get_untracked() == Some(n) {
                            focus.set(None);
                        }
                    })
                    .child(
                        DecoratedBox::new()
                            .child(
                                Row::new()
                                    .gap(14.0)
                                    .cross_axis_alignment(CrossAxisAlignment::Center)
                                    .child(
                                        DecoratedBox::new()
                                            .child(Center::new().child(Text::new((n + 1).to_string()).class("num-text")))
                                            .class("num-badge"),
                                    )
                                    .child({
                                        let mut info =
                                            Column::new().gap(3.0).child(Text::new(t(label)).class("row-title"));
                                        if changed {
                                            info = info.child(
                                                Text::new(t!("изменено, не сохранено")).class("row-desc changed"),
                                            );
                                        }
                                        info.class("grow")
                                    })
                                    .child(
                                        Dropdown::with_items(items)
                                            .selected(cur.id())
                                            .width(230.0)
                                            .on_change(move |id| {
                                                if let Some(a) = ButtonAction::from_id(id) {
                                                    c.edit(|cfg| cfg.buttons[slot] = a);
                                                }
                                            }),
                                    ),
                            )
                            .class(if changed { "btn-row btn-row-changed" } else { "btn-row" }),
                    ),
            );
        }
        if let Err(msg) = cfg.validate() {
            col = col.child(
                Row::new()
                    .gap(8.0)
                    .cross_axis_alignment(CrossAxisAlignment::Center)
                    .child(Icon::new(icons::ERROR).class("status-icon status-bad"))
                    .child(Text::new(msg).class("error-text")),
            );
        }
        col
    })
}
