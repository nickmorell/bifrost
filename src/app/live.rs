//! The app's side of the Live feature (`features::live`, Streamplace): passing it
//! messages, playing its streams in the video player, feeding it the open stream's
//! chat, and placing its views on screens.

use super::{App, Message, Route, account, nav, video};
use crate::api::streamplace;
use crate::features::Update;
use crate::features::live::{Event, Host, View};
use crate::integrations::streamplace::ID;
use crate::model::{Did, PostRecord};
use crate::theme::size;
use futures_util::StreamExt;
use iced::{Element, Subscription, Task};

pub use crate::features::live::{Live, Msg};

impl App {
    /// Whether Streamplace is turned on in Settings.
    pub(super) fn streamplace_on(&self) -> bool {
        self.settings.integration_enabled(ID)
    }

    pub(super) fn live_update(&mut self, msg: Msg) -> Task<Message> {
        let out = self.live.update(msg, &self.client);
        self.live_done(out)
    }

    /// Runs the feature's task and does what its events ask, in order.
    fn live_done(&mut self, out: Update<Msg, Event>) -> Task<Message> {
        let mut tasks = vec![out.task.map(Message::Live)];
        for event in out.events {
            tasks.push(match event {
                Event::Pictures(urls) => self.request_images(urls),
                Event::OpenChannel(actor) => self.push_route(Route::Channel(actor)),
                Event::OpenProfile(did) => {
                    self.handle(Message::Nav(nav::Msg::OpenProfile(did.to_string())))
                }
                Event::OpenUrl(url) => self.handle(Message::Nav(nav::Msg::Browse(url))),
                Event::Play { key, playlist, web } => {
                    let playing = self
                        .video
                        .as_ref()
                        .is_some_and(|v| v.post == key && v.plays(&playlist));
                    if playing {
                        Task::none()
                    } else {
                        self.video_update(video::Msg::Play {
                            post: key,
                            playlist,
                            web,
                        })
                    }
                }
                Event::Failed(e) => self.handle_session_error(e),
                Event::SignInAgain => self.handle(Message::Account(account::Msg::Changed(Ok(())))),
            });
        }
        Task::batch(tasks)
    }

    /// The streamer a stream.place link points at, when Streamplace is on.
    pub(super) fn streamplace_link(&self, url: &str) -> Option<String> {
        self.streamplace_on()
            .then(|| streamplace::streamer_in(url))
            .flatten()
    }

    /// The open stream's chat and viewer count, while its page is on screen.
    pub(super) fn live_subscription(&self) -> Option<Subscription<Message>> {
        let Route::Channel(actor) = self.route() else {
            return None;
        };
        let did = self.live.did_of(actor)?.clone();
        Some(Subscription::run_with(did, |did: &Did| {
            let did = did.clone();
            streamplace::events(did.clone())
                .map(move |e| Message::Live(Msg::Happened(did.clone(), Box::new(e))))
        }))
    }

    pub(super) fn live_title(&self, actor: &str) -> String {
        self.live.title(actor)
    }

    // ---- Views ---------------------------------------------------------------------

    fn live_views(&self) -> View<'_, Self, Message> {
        View {
            live: &self.live,
            host: self,
            now: chrono::Utc::now().to_rfc3339(),
            lift: Message::Live,
        }
    }

    pub(super) fn live_board_view(&self) -> Element<'_, Message> {
        self.live_views().board()
    }

    pub(super) fn channel_view<'a>(&'a self, actor: &'a str) -> Element<'a, Message> {
        self.live_views().channel(actor)
    }
}

impl Host<Message> for App {
    fn rich_text<'a>(&'a self, body: &'a PostRecord) -> Element<'a, Message> {
        super::post::post_text(self.look(), body, size::BODY)
    }

    fn playing(&self, key: &str) -> bool {
        self.video.as_ref().is_some_and(|v| v.post == key)
    }

    fn player<'a>(
        &'a self,
        key: &str,
        thumb: Option<&'a str>,
        watch: Message,
        w: f32,
        h: f32,
    ) -> Element<'a, Message> {
        video::stream_view(self, key, thumb, watch, w, h)
    }
}
