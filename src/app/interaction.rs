//! Who can reply to a thread and whether a post can be quoted: the settings panel
//! used by the composer and by "Who can reply" on your own posts.

use super::{App, Message};
use crate::components::{self as ui, Icon, ghost, icon_button};
use crate::features::actions::who::{self, Event};
pub use crate::features::actions::who::{Msg, Who};
use crate::model::{Interaction, PostView, ReplyRules};
use crate::theme::{self, gap, size};
use iced::widget::{Column, button, column, container, row, space, text, toggler};
use iced::{Alignment, Element, Length, Task};

/// Your curation lists, which can be allowed to reply.
fn my_lists(app: &App) -> Vec<(&str, &str)> {
    match app.profile.lists(&app.my_did) {
        Some(Ok(lists)) => lists
            .iter()
            .filter(|l| l.purpose.ends_with("curatelist"))
            .map(|l| (l.uri.as_str(), l.name.as_str()))
            .collect(),
        _ => Vec::new(),
    }
}

/// The settings, with every change sent through `on_change`. `threads` is false for
/// a reply, whose thread's own rules apply.
pub fn panel<'a>(
    app: &'a App,
    current: &Interaction,
    threads: bool,
    on_change: &dyn Fn(Interaction) -> Message,
) -> Element<'a, Message> {
    let mut col = Column::new().spacing(gap::S);
    if threads {
        let everyone = current.replies.is_none();
        let nobody = current.replies.as_ref().is_some_and(ReplyRules::is_nobody);
        let with = |replies: Option<ReplyRules>| {
            on_change(Interaction {
                replies,
                ..current.clone()
            })
        };
        let choice = |label: &'a str, on: bool, msg: Message| {
            button(text(label).size(size::BODY).center())
                .width(Length::Fill)
                .padding([gap::S, gap::M])
                .style(theme::nav(on))
                .on_press(msg)
        };
        col = col
            .push(ui::eyebrow("Who can reply"))
            .push(
                row![
                    choice("Everyone", everyone, with(None)),
                    choice("Nobody", nobody, with(Some(ReplyRules::default()))),
                ]
                .spacing(gap::S),
            )
            .push(ui::faint("Or only these people:"));
        let rules = current.replies.clone().unwrap_or_default();
        let rule = |label: String, on: bool, flip: &dyn Fn(&mut ReplyRules)| {
            let mut next = rules.clone();
            flip(&mut next);
            let msg = with(Some(next));
            row![
                text(label).size(size::BODY).width(Length::Fill),
                toggler(on && !everyone)
                    .on_toggle(move |_| msg.clone())
                    .size(size::TOGGLE)
                    .style(theme::switch),
            ]
            .align_y(Alignment::Center)
        };
        col = col
            .push(rule("People you mention".into(), rules.mentioned, &|r| {
                r.mentioned = !r.mentioned;
            }))
            .push(rule("Your followers".into(), rules.followers, &|r| {
                r.followers = !r.followers;
            }))
            .push(rule("People you follow".into(), rules.following, &|r| {
                r.following = !r.following;
            }));
        for (uri, name) in my_lists(app) {
            let on = rules.lists.iter().any(|l| l == uri);
            col = col.push(rule(format!("Members of {name}"), on, &|r| {
                if on {
                    r.lists.retain(|l| l != uri);
                } else {
                    r.lists.push(uri.to_string().into());
                }
            }));
        }
    } else {
        col = col.push(ui::faint(
            "Replies follow the settings of the thread you're replying in.",
        ));
    }
    let quotes = Interaction {
        quotes: !current.quotes,
        ..current.clone()
    };
    let quotes = on_change(quotes);
    col.push(container(ui::divider()).padding([gap::XS, 0.0]))
        .push(
            row![
                text("Allow quote posts")
                    .size(size::BODY)
                    .width(Length::Fill),
                toggler(current.quotes)
                    .on_toggle(move |_| quotes.clone())
                    .size(size::TOGGLE)
                    .style(theme::switch),
            ]
            .align_y(Alignment::Center),
        )
        .into()
}

/// The dialog for changing who can reply to and quote a post you already made.
pub fn dialog<'a>(app: &'a App, e: &'a Who) -> Element<'a, Message> {
    let body: Element<'a, Message> = match &e.value {
        None => ui::faint("Loading…").into(),
        Some(v) => panel(app, v, e.is_root, &|a| {
            Message::Interaction(Msg::Changed(a))
        }),
    };
    container(
        column![
            row![
                text("Who can interact")
                    .size(size::TITLE)
                    .font(theme::SEMIBOLD),
                space::horizontal(),
                icon_button(Icon::Close, Message::Interaction(Msg::Close)),
            ]
            .align_y(Alignment::Center),
            body,
            row![
                space::horizontal(),
                ghost("Cancel", Message::Interaction(Msg::Close)),
                ui::primary(if e.saving { "Saving…" } else { "Save" }).on_press_maybe(
                    (e.value.is_some() && !e.saving).then_some(Message::Interaction(Msg::Save))
                ),
            ]
            .spacing(gap::S)
            .align_y(Alignment::Center),
        ]
        .spacing(gap::M),
    )
    .padding(gap::XL)
    .width(theme::layout::DIALOG)
    .style(theme::dialog)
    .into()
}

/// Who can reply to the thread `root` starts, as a sentence, if it is limited.
pub fn reply_note(root: &PostView) -> Option<String> {
    let gate = root.threadgate.as_ref()?;
    let rules = Interaction::from_records(Some(&gate.record), None).replies?;
    if rules.is_nobody() {
        return Some("Replies to this thread are off.".into());
    }
    let who = &root.author.handle;
    let mut parts = Vec::new();
    if rules.mentioned {
        parts.push("people mentioned".to_string());
    }
    if rules.followers {
        parts.push(format!("@{who}'s followers"));
    }
    if rules.following {
        parts.push(format!("people @{who} follows"));
    }
    for uri in &rules.lists {
        let name = gate
            .lists
            .iter()
            .find(|l| &l.uri == uri)
            .map_or("a list", |l| l.name.as_str());
        parts.push(format!("members of {name}"));
    }
    let list = match parts.len() {
        1 => parts.remove(0),
        n => {
            let last = parts.remove(n - 1);
            format!("{} and {last}", parts.join(", "))
        }
    };
    Some(format!("Only {list} can reply."))
}

impl App {
    pub(super) fn interaction_update(&mut self, msg: Msg) -> Task<Message> {
        if matches!(msg, Msg::Open(_)) {
            self.popups.menu = None;
        }
        let mut editing = self.interaction_edit.take();
        let out = who::update(&mut editing, msg, &*self, &self.client);
        self.interaction_edit = editing;
        let mut tasks = vec![out.task.map(Message::Interaction)];
        for event in out.events {
            match event {
                Event::NeedMyLists => tasks.push(self.load_my_lists()),
                Event::Gate(uri, gate) => self.with_post(&uri, |p| p.threadgate.clone_from(&gate)),
                Event::Toast(t) => self.toast = Some(t),
                Event::Failed(action, e) => tasks.push(self.action_failed(action, e)),
            }
        }
        Task::batch(tasks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn post(gate: Option<serde_json::Value>) -> PostView {
        serde_json::from_value(json!({
            "uri": "at://did:plc:a/app.bsky.feed.post/1",
            "cid": "c",
            "author": {"did": "did:plc:a", "handle": "a.test"},
            "record": {"text": "hi", "createdAt": "2026-01-01T00:00:00Z"},
            "threadgate": gate.map(|record| json!({
                "uri": "at://did:plc:a/app.bsky.feed.threadgate/1",
                "record": record,
                "lists": [{"uri": "at://l", "name": "Friends"}]
            })),
        }))
        .unwrap()
    }

    #[test]
    fn reply_notes_say_who_can_reply() {
        assert_eq!(reply_note(&post(None)), None);
        assert_eq!(
            reply_note(&post(Some(json!({"allow": []})))).as_deref(),
            Some("Replies to this thread are off.")
        );
        let gate = json!({"allow": [
            {"$type": "app.bsky.feed.threadgate#followerRule"},
            {"$type": "app.bsky.feed.threadgate#listRule", "list": "at://l"}
        ]});
        assert_eq!(
            reply_note(&post(Some(gate))).as_deref(),
            Some("Only @a.test's followers and members of Friends can reply.")
        );
    }
}
