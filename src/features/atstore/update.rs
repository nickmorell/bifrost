//! Reading the directory and listing pages, and changing what the list shows.

use super::{Event, Msg, Out, PAGE, Reading, STALE_HOURS, Store};
use crate::api::service::AtstoreApi;
use crate::features::Update;
use crate::model::AtUri;
use iced::Task;

impl Store {
    pub fn update(&mut self, msg: Msg, api: &impl AtstoreApi) -> Out {
        match msg {
            Msg::Open {
                now,
                offline,
                data_saver,
            } => {
                // The kept directory does while it is fresh, while offline, and with the
                // data saver on; "Refresh" still reads it again.
                let kept_will_do = self.has_catalog()
                    && (offline || data_saver || !self.catalog.older_than(STALE_HOURS, &now));
                if kept_will_do || self.loading {
                    Update::event(self.pictures())
                } else {
                    self.fetch(api)
                }
            }
            Msg::Refresh if self.loading => Update::none(),
            Msg::Refresh => self.fetch(api),
            Msg::Loaded(result) => {
                self.loading = false;
                match result {
                    Ok(catalog) => {
                        self.catalog.replace(*catalog);
                        self.error = None;
                        if self.tag.as_ref().is_some_and(|t| !self.has_tag(t)) {
                            self.tag = None;
                        }
                        Update::event(self.save()).and(self.pictures())
                    }
                    Err(e) => {
                        self.error = Some(e.friendly());
                        Update::none()
                    }
                }
            }
            Msg::Query(q) => {
                self.query = q;
                self.shown = PAGE;
                Update::event(self.pictures())
            }
            Msg::Tag(tag) => {
                self.tag = tag;
                self.shown = PAGE;
                Update::event(self.pictures())
            }
            Msg::Sort(sort) => {
                self.sort = sort;
                self.shown = PAGE;
                Update::event(self.pictures())
            }
            Msg::More => {
                self.shown += PAGE;
                Update::event(self.pictures())
            }
            Msg::Select(uri) => Update::event(Event::OpenListing(uri)),
            Msg::Show { uri, offline } => {
                let kept = self.catalog.details.contains_key(&uri);
                let busy = self.pages.get(&uri) == Some(&Reading::Loading);
                if kept || busy || (offline && self.listing(&uri).is_some()) {
                    Update::event(self.page_pictures(&uri))
                } else {
                    self.fetch_page(uri, api)
                }
            }
            Msg::Reload(uri) if self.pages.get(&uri) == Some(&Reading::Loading) => Update::none(),
            Msg::Reload(uri) => self.fetch_page(uri, api),
            Msg::Page(uri, result) => match result {
                Ok(detail) => {
                    self.pages.remove(&uri);
                    let detail = *detail;
                    if let Some(l) = self.catalog.listings.iter_mut().find(|l| l.uri == uri) {
                        *l = detail.listing.clone();
                    }
                    self.catalog.details.insert(uri.clone(), detail);
                    Update::event(self.save()).and(self.page_pictures(&uri))
                }
                Err(e) => {
                    self.pages.insert(uri, Reading::Failed(e.friendly()));
                    Update::none()
                }
            },
            Msg::OpenUrl(url) => Update::event(Event::OpenUrl(url)),
            Msg::OpenProfile(handle) => Update::event(Event::OpenProfile(handle)),
        }
    }

    fn fetch(&mut self, api: &impl AtstoreApi) -> Out {
        self.loading = true;
        let api = api.clone();
        Update::task(Task::perform(
            async move { api.catalog().await.map(Box::new) },
            Msg::Loaded,
        ))
    }

    fn fetch_page(&mut self, uri: AtUri, api: &impl AtstoreApi) -> Out {
        self.pages.insert(uri.clone(), Reading::Loading);
        let pictures = self.page_pictures(&uri);
        let api = api.clone();
        let u = uri.clone();
        Update::task(Task::perform(
            async move { api.listing(&u).await.map(Box::new) },
            move |r| Msg::Page(uri.clone(), r),
        ))
        .and(pictures)
    }

    fn save(&self) -> Event {
        Event::Save(Box::new(self.catalog.clone()))
    }

    fn has_tag(&self, tag: &str) -> bool {
        self.catalog
            .listings
            .iter()
            .any(|l| l.app_tags.iter().any(|t| t == tag))
    }

    /// The icons of the listings shown.
    fn pictures(&self) -> Event {
        Event::Pictures(
            self.matching()
                .into_iter()
                .take(self.shown)
                .filter_map(|l| l.icon_url.clone())
                .collect(),
        )
    }

    /// A listing's icon, wide picture and screenshots.
    fn page_pictures(&self, uri: &AtUri) -> Event {
        let mut urls = Vec::new();
        if let Some(l) = self.listing(uri) {
            urls.extend(l.icon_url.clone());
            urls.extend(l.hero_image_url.clone());
        }
        if let Some(d) = self.catalog.details.get(uri) {
            urls.extend(d.screenshots.iter().cloned());
        }
        Event::Pictures(urls)
    }
}
