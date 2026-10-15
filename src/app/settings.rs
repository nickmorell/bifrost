//! The Settings screen, wired to the app: the preferences it draws come from
//! `crate::storage::settings`, and what it asks for is applied and saved here.

use super::{App, Message, Route, account, nav, window};
use crate::api::ApiError;
use crate::components::{self as ui, ghost};
use crate::features::settings::{self, Event, Msg, Prefs, Service};
use crate::integrations::Group;
use crate::storage::image_cache;
use crate::storage::settings::{Appearance, Settings, ZOOM_STEPS};
use crate::theme::{self, gap, size};
use iced::widget::{Column, column, container, row, text};
use iced::{Alignment, Element, Length, Task};

impl App {
    /// Handles one message from the Settings screen and applies what it asks for.
    pub(super) fn settings_update(&mut self, msg: Msg) -> Task<Message> {
        let out = self.settings_screen.update(msg);
        let mut tasks = vec![out.task.map(Message::Settings)];
        for event in out.events {
            tasks.push(match event {
                Event::Zoom(z) => self.handle(Message::Window(window::Msg::Zoom(z))),
                Event::Appearance(i) => {
                    self.settings.appearance = Appearance::ALL[i];
                    self.save_settings();
                    Task::none()
                }
                Event::SansReading(on) => {
                    self.settings.sans_reading = on;
                    self.save_settings();
                    Task::none()
                }
                Event::ReduceMotion(on) => {
                    self.settings.reduce_motion = on;
                    self.save_settings();
                    Task::none()
                }
                Event::Deck(on) => self.handle(Message::Deck(super::deck::Msg::Toggle(on))),
                Event::PostLang(code) => {
                    self.settings.post_lang = Some(code);
                    self.save_settings();
                    Task::none()
                }
                Event::Integration(id, on) => {
                    self.settings.integrations.insert(id.to_string(), on);
                    self.save_settings();
                    self.integration_thumbnails()
                }
                Event::Alert(kind, on) => {
                    self.settings.alerts.set(kind, on);
                    self.save_settings();
                    Task::none()
                }
                Event::MessageText(on) => {
                    self.settings.alerts.message_text = on;
                    self.save_settings();
                    Task::none()
                }
                Event::CloseToTray(on) => {
                    self.settings.close_to_tray = on;
                    self.save_settings();
                    Task::none()
                }
                Event::AltReminder(on) => {
                    self.settings.alt_reminder = on;
                    self.save_settings();
                    Task::none()
                }
                Event::DataSaver(on) => {
                    self.settings.data_saver = on;
                    self.images.set_lazy(on);
                    self.save_settings();
                    Task::none()
                }
                Event::Moderation => {
                    self.handle(Message::Nav(nav::Msg::OpenRoute(Route::Moderation)))
                }
                Event::CheckUpdates(on) => self.set_check_updates(on),
                Event::ReportProblem => self.report_problem(),
                Event::Browse(url) => self.handle(Message::Nav(nav::Msg::Browse(url))),
                Event::Wipe => self.remove_all_data(),
            });
        }
        Task::batch(tasks)
    }

    /// Saves the preferences, saying so if it fails.
    pub(super) fn save_settings(&mut self) {
        if let Err(e) = self.settings.save() {
            self.toast = Some(format!("Could not save settings: {e}"));
        }
    }

    /// Newly enabled integrations may want thumbnails for posts already loaded.
    fn integration_thumbnails(&mut self) -> Task<Message> {
        let mut urls = Vec::new();
        for p in self.timelines.posts() {
            self.integration_images(p, &mut urls);
        }
        for p in self.threads.posts() {
            self.integration_images(p, &mut urls);
        }
        for p in self.notifications.posts().chain(self.search.posts()) {
            self.integration_images(p, &mut urls);
        }
        self.request_images(urls)
    }

    /// Signs out everywhere and deletes what BiFrost keeps on this computer.
    fn remove_all_data(&mut self) -> Task<Message> {
        self.drafts.store.delete_everything();
        image_cache::clear();
        let keys = self.translate_update(super::translate::Msg::ForgetKeys);
        self.settings = Settings::default();
        Settings::delete_file();
        self.reading.places = crate::storage::places::Places::default();
        self.reading.dirty = false;
        self.archive = crate::storage::archive::Archive::default();
        self.collections = super::collections::Collections::default();
        self.atstore = super::atstore::Store::default();
        let client = self.client.clone();
        let dids: Vec<String> = self.accounts_list().iter().map(|a| a.did.clone()).collect();
        let signed_out = Task::perform(
            async move {
                let mut result = Ok(());
                for did in dids {
                    if let Err(e) = client.logout(&did).await {
                        result = Err(e);
                    }
                }
                // Last, so nothing signing out writes a file back afterwards.
                if let Err(e) = crate::platform::paths::remove_all() {
                    result = Err(ApiError::Other(format!(
                        "Some of BiFrost's files could not be deleted: {e}"
                    )));
                }
                crate::platform::notify::forget();
                result
            },
            |a| Message::Account(account::Msg::Changed(a)),
        );
        Task::batch([keys, signed_out])
    }

    /// The preferences as the Settings screen draws them.
    fn prefs(&self) -> Prefs {
        Prefs {
            appearance: Appearance::ALL
                .iter()
                .position(|a| *a == self.settings.appearance)
                .unwrap_or(0),
            appearances: Appearance::ALL.iter().map(|a| a.label()).collect(),
            zoom: self.settings.zoom,
            zooms: &ZOOM_STEPS,
            sans_reading: self.settings.sans_reading,
            reduce_motion: self.settings.reduce_motion,
            alt_reminder: self.settings.alt_reminder,
            data_saver: self.settings.data_saver,
            deck: self.settings.deck,
            post_lang: self.post_lang(),
            muted_words: self.prefs.muted_words.len(),
            media: self.services(Group::Media),
            services: self.services(Group::Service),
            alerts: self.settings.alerts,
            close_to_tray: self
                .tray
                .icon
                .is_some()
                .then_some(self.settings.close_to_tray),
            version: crate::api::github::VERSION,
            update: self.update_check(),
        }
    }

    /// The integrations listed under `group`, each with its switch.
    fn services(&self, group: Group) -> Vec<Service> {
        self.integrations
            .iter()
            .filter(|i| i.group() == group)
            .map(|i| Service {
                id: i.id(),
                name: i.name(),
                description: i.description(),
                on: self.settings.integration_enabled(i.id()),
            })
            .collect()
    }

    /// The Settings screen.
    pub(super) fn settings_view(&self) -> Element<'_, Message> {
        let body = settings::View {
            screen: &self.settings_screen,
            prefs: self.prefs(),
            host: self,
            lift: Message::Settings,
        }
        .body();
        self.scroll(body)
    }
}

impl settings::Host<Message> for App {
    fn translation(&self) -> Element<'_, Message> {
        self.translate_settings()
    }

    fn privacy(&self) -> Element<'_, Message> {
        self.privacy_settings()
    }

    fn chats(&self) -> Element<'_, Message> {
        self.chat_settings()
    }

    fn accounts(&self) -> Element<'_, Message> {
        let list = self.accounts_list();
        let mut col = Column::new().spacing(gap::XS);
        for a in list {
            let mut r = row![
                column![
                    text(format!("@{}", a.handle)).size(size::BODY),
                    ui::faint(a.did.clone()).size(size::CAPTION),
                ]
                .spacing(gap::XXS)
                .width(Length::Fill)
            ]
            .spacing(gap::S)
            .align_y(Alignment::Center);
            if a.active {
                r = r.push(
                    container(text("Active").size(size::CAPTION).font(theme::MEDIUM))
                        .padding([2.0, 8.0])
                        .style(theme::badge),
                );
            } else {
                r = r.push(ghost(
                    "Switch",
                    Message::Account(account::Msg::Switch(a.did.clone())),
                ));
            }
            r = r.push(ghost(
                "Log out",
                Message::Account(account::Msg::Logout(a.did.clone())),
            ));
            col = col.push(container(r).padding([gap::S, gap::M]));
        }
        let mut out = Column::new().spacing(gap::M);
        if !list.is_empty() {
            out = out.push(ui::card(col).padding(gap::XS));
        }
        out.push(ui::secondary("Add another account").on_press(Message::Account(account::Msg::Add)))
            .into()
    }
}
