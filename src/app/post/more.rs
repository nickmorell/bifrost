//! A post's "…" menu.

use super::{thread_root, web_url};
use crate::api::ReportSubject;
use crate::app::{App, Message, Toggle, actions, interaction, nav, popups};
use crate::components::{self as ui, Icon};
use crate::model::{PostView, StrongRef};
use iced::Element;

pub(super) fn menu<'a>(app: &'a App, post: &'a PostView) -> Element<'a, Message> {
    let a = &post.author;
    let mine = a.did == app.my_did;
    let root = thread_root(post).clone();
    let mut items = vec![
        ui::menu_item(
            Icon::Language,
            "Translate",
            Some(crate::app::translate::translate_post(post)),
        ),
        ui::menu_item(
            Icon::Copy,
            "Copy post text",
            Some(Message::Popup(popups::Msg::CopyText(
                post.record.text.clone(),
            ))),
        ),
        ui::menu_item(
            Icon::Quote,
            "Select text",
            Some(Message::Popup(popups::Msg::SelectText(
                post.record.text.clone(),
            ))),
        ),
        ui::menu_item(
            Icon::Link,
            "Copy link to post",
            Some(Message::Popup(popups::Msg::CopyText(web_url(post)))),
        ),
        ui::menu_item(
            Icon::External,
            "Open in browser",
            Some(Message::Nav(nav::Msg::OpenUrl(web_url(post)))),
        ),
        ui::menu_item(
            Icon::Send,
            "Send in a chat",
            Some(Message::Chat(crate::app::chat::Msg::SharePost(StrongRef {
                uri: post.uri.clone(),
                cid: post.cid.clone(),
            }))),
        ),
        ui::menu_divider(),
        if post.viewer.thread_muted {
            ui::menu_item(
                Icon::Bell,
                "Unmute thread",
                Some(Message::Action(actions::Msg::MuteThread(root, Toggle::Off))),
            )
        } else {
            ui::menu_item(
                Icon::BellOff,
                "Mute thread",
                Some(Message::Action(actions::Msg::MuteThread(root, Toggle::On))),
            )
        },
    ];
    items.extend(app.collection_items(post));
    items.push(ui::menu_divider());
    if mine {
        let pinned = post.viewer.pinned
            || app
                .me
                .as_ref()
                .and_then(|m| m.pinned_post.as_ref())
                .is_some_and(|p| p.uri == post.uri);
        items.push(if pinned {
            ui::menu_item(
                Icon::Pin,
                "Unpin from your profile",
                Some(Message::Action(actions::Msg::Pin(
                    post.uri.clone(),
                    Toggle::Off,
                ))),
            )
        } else {
            ui::menu_item(
                Icon::Pin,
                "Pin to your profile",
                Some(Message::Action(actions::Msg::Pin(
                    post.uri.clone(),
                    Toggle::On,
                ))),
            )
        });
        items.push(ui::menu_item(
            Icon::Lock,
            "Who can reply and quote",
            Some(Message::Interaction(interaction::Msg::Open(
                post.uri.clone(),
            ))),
        ));
        items.push(ui::menu_divider());
        items.push(if app.actions.confirm_delete() == Some(&post.uri) {
            ui::danger_item(
                Icon::Trash,
                "Really delete?",
                Message::Action(actions::Msg::Delete(post.uri.clone())),
            )
        } else {
            ui::danger_item(
                Icon::Trash,
                "Delete post",
                Message::Action(actions::Msg::AskDelete(post.uri.clone())),
            )
        });
    } else {
        items.push(if a.viewer.muted {
            ui::menu_item(
                Icon::VolumeOff,
                "Unmute account",
                Some(Message::Action(actions::Msg::Mute(
                    a.did.clone(),
                    Toggle::Off,
                ))),
            )
        } else {
            ui::menu_item(
                Icon::VolumeOff,
                "Mute account",
                Some(Message::Action(actions::Msg::Mute(
                    a.did.clone(),
                    Toggle::On,
                ))),
            )
        });
        items.push(ui::menu_divider());
        items.push(if a.viewer.blocking.is_some() {
            ui::menu_item(
                Icon::Ban,
                "Unblock account",
                Some(Message::Action(actions::Msg::Block(
                    a.did.clone(),
                    Toggle::Off,
                ))),
            )
        } else {
            ui::danger_item(
                Icon::Ban,
                "Block account",
                Message::Action(actions::Msg::Block(a.did.clone(), Toggle::On)),
            )
        });
        items.push(ui::danger_item(
            Icon::Flag,
            "Report post",
            Message::Moderation(crate::app::moderation::Msg::Report(ReportSubject::Post(
                StrongRef {
                    uri: post.uri.clone(),
                    cid: post.cid.clone(),
                },
            ))),
        ));
    }
    ui::menu(items)
}
