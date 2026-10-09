//! The app's side of OS notifications (`features::alerts`): checking while the window
//! is in the background and showing what the feature finds.

use super::{App, Message, chat, notifications};
use crate::features::alerts::{Ctx, Msg, Notice};
use crate::model::Notification;
use crate::platform::notify;
use iced::Task;

impl App {
    pub(super) fn alerts_update(&mut self, msg: Msg) -> Task<Message> {
        // What the background check fetched also serves the unread count and the chat
        // list, so they aren't fetched a second time.
        let shared = match &msg {
            Msg::Notifications(Ok(page)) => self.count_from(&page.items),
            Msg::Convos(Ok(page)) => {
                self.chat_update(chat::Msg::Listed(false, false, Ok(page.clone())))
            }
            _ => Task::none(),
        };
        let ctx = Ctx {
            kinds: &self.settings.alerts,
            me: &self.my_did,
            prefs: &self.prefs,
        };
        let out = self.alerts.update(msg, &ctx, &self.client);
        let mut tasks = vec![shared, out.task.map(Message::Alerts)];
        tasks.extend(out.events.into_iter().map(|n| self.notify(n)));
        Task::batch(tasks)
    }

    /// Whether the background check fetches your notifications and your chats.
    pub(super) fn alerts_fetch(&self) -> (bool, bool) {
        self.alerts.fetches(&self.settings.alerts)
    }

    /// The unread count from the newest notifications. When all of them are unread
    /// there may be more, so the server is asked.
    fn count_from(&mut self, newest: &[Notification]) -> Task<Message> {
        let unread = newest.iter().filter(|n| !n.is_read).count();
        let msg = if unread == newest.len() {
            notifications::Msg::Count
        } else {
            notifications::Msg::Counted(Ok(unread as u64))
        };
        self.notifications_update(msg)
    }

    /// From now on only what arrives after this moment is news: called whenever the
    /// window gains or loses focus, and when a session starts.
    pub(super) fn alerts_seen(&mut self) -> Task<Message> {
        self.alerts_update(Msg::Seen(chrono::Utc::now()))
    }

    /// Shows a notice, off the UI thread since the OS can take a moment. One that
    /// can't be shown (no notification service) is not worth interrupting anyone for.
    fn notify(&self, notice: Notice) -> Task<Message> {
        let notifier = self.notifier.clone();
        let notice = notify::Notice {
            title: notice.title,
            body: notice.body,
        };
        Task::future(async move {
            let _ = tokio::task::spawn_blocking(move || notifier.show(&notice)).await;
        })
        .discard()
    }
}
