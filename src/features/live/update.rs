//! Loading streams and streamer pages, keeping the chat in step, and sending to it.

use super::state::{Channel, MAX_CHAT, MAX_MESSAGE};
use super::{Event, Live, Msg, Out, player_key};
use crate::api::richtext::grapheme_len;
use crate::api::service::LiveApi;
use crate::api::streamplace;
use crate::features::Update;
use crate::model::{Did, LiveChat, LiveEvent};
use iced::Task;

impl Live {
    pub fn update(&mut self, msg: Msg, api: &impl LiveApi) -> Out {
        match msg {
            Msg::OpenBoard if self.board.loaded => Update::none(),
            Msg::OpenBoard | Msg::RefreshBoard => {
                if self.board.loading {
                    return Update::none();
                }
                self.board.loading = true;
                let api = api.clone();
                Update::task(Task::perform(
                    async move { api.live_now().await },
                    Msg::Board,
                ))
            }
            Msg::Board(result) => {
                let b = &mut self.board;
                b.loading = false;
                b.loaded = true;
                match result {
                    Ok((streams, authors)) => {
                        let mut urls: Vec<String> =
                            streams.iter().filter_map(|s| s.thumb.clone()).collect();
                        urls.extend(authors.iter().filter_map(|a| a.avatar.clone()));
                        b.streams = streams;
                        b.authors = authors.into_iter().map(|a| (a.did.clone(), a)).collect();
                        b.error = None;
                        Update::event(pictures(urls))
                    }
                    Err(e) => {
                        b.error = Some(e.friendly());
                        Update::event(Event::Failed(e))
                    }
                }
            }
            Msg::Open(actor) | Msg::Refresh(actor) => self.load(actor, api),
            Msg::Loaded(actor, result) => self.loaded(actor, result, api),
            Msg::MorePast(actor) => self.load_past(actor, true, api),
            Msg::Past(actor, append, result) => {
                let c = self.channels.entry(actor).or_default();
                c.past_loading = false;
                match result {
                    Ok(page) => {
                        if !append {
                            c.past.clear();
                        }
                        c.past.extend(page.items);
                        c.past_cursor = page.cursor;
                        Update::none()
                    }
                    // The page itself still shows; only the past streams are missing.
                    Err(_) => Update::none(),
                }
            }
            Msg::Happened(did, event) => self.happened(&did, *event),
            Msg::Draft(actor, text) => {
                let c = self.channels.entry(actor).or_default();
                c.draft = text;
                c.send_error = None;
                Update::none()
            }
            Msg::Send(actor) => self.send(actor, api),
            Msg::Sent(actor, result) => {
                let c = self.channels.entry(actor).or_default();
                c.sending = false;
                match result {
                    // The message comes back through the chat, like everyone else's.
                    Ok(_) => Update::none(),
                    Err(e) => {
                        c.send_error = Some(e.friendly());
                        Update::event(Event::Failed(e))
                    }
                }
            }
            Msg::Watch(actor) => self.watch(&actor),
            Msg::WatchPast(actor, video) => match self.did_of(&actor) {
                Some(did) => Update::event(Event::Play {
                    key: player_key(did),
                    playlist: streamplace::past_playlist(&video),
                    web: streamplace::web_page(&actor),
                }),
                None => Update::none(),
            },
            Msg::OpenStream(actor) => Update::event(Event::OpenChannel(actor)),
            Msg::OpenProfile(did) => Update::event(Event::OpenProfile(did)),
            Msg::OpenWeb(actor) => Update::event(Event::OpenUrl(streamplace::web_page(&actor))),
            Msg::SignInAgain => Update::event(Event::SignInAgain),
        }
    }

    fn load(&mut self, actor: String, api: &impl LiveApi) -> Out {
        let c = self.channels.entry(actor.clone()).or_default();
        if c.loading {
            return Update::none();
        }
        c.loading = true;
        c.error = None;
        let api = api.clone();
        let a = actor.clone();
        Update::task(Task::perform(
            async move { api.channel(&a).await.map(Box::new) },
            move |r| Msg::Loaded(actor.clone(), r),
        ))
    }

    fn loaded(
        &mut self,
        actor: String,
        result: Result<Box<crate::api::service::Channel>, crate::api::ApiError>,
        api: &impl LiveApi,
    ) -> Out {
        let c = self.channels.entry(actor.clone()).or_default();
        c.loading = false;
        let info = match result {
            Ok(info) => *info,
            Err(e) => {
                c.error = Some(e.friendly());
                return Update::event(Event::Failed(e));
            }
        };
        let mut urls: Vec<String> = info.profile.avatar.iter().cloned().collect();
        urls.extend(info.latest.as_ref().and_then(|s| s.thumb.clone()));
        c.profile = Some(info.profile);
        c.latest = info.latest;
        c.error = None;
        let mut out = Update::event(pictures(urls));
        // A stream that is on starts playing as the page opens.
        if c.is_live(&now()) {
            out = out.then(self.watch(&actor));
        }
        out.then(self.load_past(actor, false, api))
    }

    fn load_past(&mut self, actor: String, append: bool, api: &impl LiveApi) -> Out {
        let c = self.channels.entry(actor.clone()).or_default();
        if c.past_loading || (append && c.past_cursor.is_none()) {
            return Update::none();
        }
        c.past_loading = true;
        let cursor = if append { c.past_cursor.clone() } else { None };
        let api = api.clone();
        let a = actor.clone();
        Update::task(Task::perform(
            async move { api.past(&a, cursor).await },
            move |r| Msg::Past(actor.clone(), append, r),
        ))
    }

    fn watch(&self, actor: &str) -> Out {
        match self.did_of(actor) {
            Some(did) => Update::event(Event::Play {
                key: player_key(did),
                playlist: streamplace::live_playlist(did),
                web: streamplace::web_page(actor),
            }),
            None => Update::none(),
        }
    }

    fn send(&mut self, actor: String, api: &impl LiveApi) -> Out {
        let c = self.channels.entry(actor.clone()).or_default();
        let text = c.draft.trim().to_string();
        let Some(did) = c.did().cloned() else {
            return Update::none();
        };
        if c.sending || text.is_empty() || grapheme_len(&text) > MAX_MESSAGE {
            return Update::none();
        }
        c.sending = true;
        c.draft.clear();
        c.send_error = None;
        let api = api.clone();
        Update::task(Task::perform(
            async move { api.send(&did, &text).await },
            move |r| Msg::Sent(actor.clone(), r),
        ))
    }

    /// Applies something that happened in `did`'s stream to their page.
    fn happened(&mut self, did: &Did, event: LiveEvent) -> Out {
        let Some(c) = self.channels.values_mut().find(|c| c.did() == Some(did)) else {
            return Update::none();
        };
        match event {
            LiveEvent::Chat(m) if c.blocked.contains(&m.author.did) => Update::none(),
            LiveEvent::Chat(m) => {
                let urls: Vec<String> = m.author.avatar.iter().cloned().collect();
                add(c, m);
                Update::event(pictures(urls))
            }
            LiveEvent::Removed(uri) => {
                c.chat.retain(|m| m.uri != uri);
                if c.pinned.as_ref().is_some_and(|p| p.uri == uri) {
                    c.pinned = None;
                }
                Update::none()
            }
            LiveEvent::Blocked(who) => {
                c.chat.retain(|m| m.author.did != who);
                c.blocked.insert(who);
                Update::none()
            }
            LiveEvent::Viewers(n) => {
                c.viewers = Some(n);
                Update::none()
            }
            LiveEvent::Stream(s) => {
                let urls: Vec<String> = s.thumb.iter().cloned().collect();
                c.latest = Some(s);
                Update::event(pictures(urls))
            }
            LiveEvent::Pinned(m) => {
                c.pinned = m;
                Update::none()
            }
            LiveEvent::Connected(on) => {
                c.connected = on;
                Update::none()
            }
        }
    }
}

/// Adds a chat message in time order, replacing an earlier copy of it.
fn add(c: &mut Channel, m: LiveChat) {
    if let Some(old) = c.chat.iter_mut().find(|o| o.uri == m.uri) {
        *old = m;
        return;
    }
    // Most arrive last; the history sent on connecting may come in any order.
    let at = c
        .chat
        .iter()
        .rposition(|o| o.body.created_at <= m.body.created_at)
        .map_or(0, |i| i + 1);
    c.chat.insert(at, m);
    while c.chat.len() > MAX_CHAT {
        c.chat.pop_front();
    }
}

/// Hosts pictures in Streamplace data may come from: Bluesky's image server and the
/// Streamplace node. A chat message's author comes from whoever wrote it, so an
/// avatar anywhere else isn't fetched, and can't tell another server who is watching.
const PICTURE_HOSTS: [&str; 2] = ["cdn.bsky.app", "stream.place"];

/// Asks for the pictures at `urls` that are on a trusted host over https.
pub(super) fn pictures(urls: Vec<String>) -> Event {
    let trusted = |u: &String| {
        url::Url::parse(u).is_ok_and(|u| {
            u.scheme() == "https"
                && u.host_str().is_some_and(|h| {
                    PICTURE_HOSTS
                        .iter()
                        .any(|t| h == *t || h.strip_suffix(t).is_some_and(|p| p.ends_with('.')))
                })
        })
    };
    Event::Pictures(urls.into_iter().filter(trusted).collect())
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}
