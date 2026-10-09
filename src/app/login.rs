//! The app's side of the login screen (`features::login`): signing in the way the
//! form asks.

use super::{App, Message, Phase, account};
use crate::api::ApiError;
use crate::features::login::Event;

pub use crate::features::login::{Login, Msg};
use iced::Task;

impl App {
    pub(super) fn login_update(&mut self, msg: Msg) -> Task<Message> {
        let Phase::Login(state) = &mut self.phase else {
            return Task::none();
        };
        match state.update(msg) {
            Some(Event::Browser(handle)) => {
                let client = self.client.clone();
                let (task, abort) = Task::perform(
                    async move { client.sign_in_with_browser(&handle).await },
                    |a| Message::Account(account::Msg::LoggedIn(a)),
                )
                .abortable();
                self.sign_in_abort = Some(abort);
                task
            }
            Some(Event::CancelBrowser) => {
                if let Some(abort) = self.sign_in_abort.take() {
                    abort.abort();
                }
                Task::none()
            }
            Some(Event::Password(creds)) => {
                let accounts = self.accounts.clone();
                Task::perform(
                    async move {
                        accounts
                            .login(&creds)
                            .await
                            .map(|s| s.did)
                            .map_err(ApiError::from)
                    },
                    |a| Message::Account(account::Msg::LoggedIn(a)),
                )
            }
            None => Task::none(),
        }
    }
}
