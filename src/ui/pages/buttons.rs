//! Страница назначения кнопок.

use crate::protocol::buttons::{ButtonAction, PHYSICAL};
use crate::ui::app::AppCtx;
use crate::ui::icons;
use crate::ui::widgets::{page_header, reactive};
use syngui::prelude::*;

const SCHEME_PNG: &[u8] = include_bytes!("../../../assets/skins/0806/mouse_nr.png");

pub fn view(ctx: AppCtx) -> impl Widget {
    Column::new()
        .gap(18.0)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(page_header(
            "Кнопки",
            "Назначение шести кнопок мыши. Номера совпадают со схемой слева.",
        ))
        .child(
            Row::new()
                .gap(18.0)
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .child(
                    DecoratedBox::new()
                        .child(
                            Image::from_bytes("buttons-scheme", SCHEME_PNG.to_vec())
                                .fit(ImageFit::Contain)
                                .class("scheme-img"),
                        )
                        .class("card scheme-card"),
                )
                .child(DecoratedBox::new().child(list(ctx)).class("grow")),
        )
        .class("page")
}

fn list(ctx: AppCtx) -> impl Widget {
    reactive(move || {
        let cfg = ctx.sink.edit.get();
        let mut col = Column::new().gap(10.0).cross_axis_alignment(CrossAxisAlignment::Stretch);
        for (n, (label, slot)) in PHYSICAL.into_iter().enumerate() {
            let cur = cfg.buttons[slot];
            let mut items: Vec<DropdownItem> = ButtonAction::CHOICES
                .into_iter()
                .map(|a| DropdownItem::new(a.id(), a.label()))
                .collect();
            if let ButtonAction::Other(_) = cur {
                items.push(DropdownItem::new(cur.id(), cur.label()));
            }
            let c = ctx.clone();
            let changed = cfg.buttons[slot] != ctx.sink.snapshot.get_untracked().map_or(cur, |s| s.buttons[slot]);
            col = col.child(
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
                                let mut info = Column::new().gap(3.0).child(Text::new(label).class("row-title"));
                                if changed {
                                    info = info.child(Text::new("изменено, не сохранено").class("row-desc changed"));
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
