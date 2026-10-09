//! Loading lists of accounts a page at a time.

use super::state::List;
use super::{Event, Msg, Out, People};
use crate::api::service::PeopleApi;
use crate::features::Update;
use crate::model::PeopleKind;
use iced::Task;

impl People {
    pub fn update(&mut self, msg: Msg, api: &impl PeopleApi) -> Out {
        match msg {
            Msg::Open(actor, kind) if self.contains(&actor, kind) => Update::none(),
            Msg::Open(actor, kind) | Msg::Refresh(actor, kind) => {
                self.load(actor, kind, false, api)
            }
            Msg::More(actor, kind) => self.load(actor, kind, true, api),
            Msg::Retry(actor, kind) => {
                // A failed first page starts over; a failed later page tries it again.
                let append = self
                    .lists
                    .get(&(actor.clone(), kind))
                    .is_some_and(|l| !l.items.is_empty());
                self.load(actor, kind, append, api)
            }
            Msg::Loaded(actor, kind, append, result) => {
                let list = self.lists.entry((actor, kind)).or_default();
                list.loading = false;
                match result {
                    Ok(page) => {
                        let urls: Vec<String> =
                            page.items.iter().filter_map(|a| a.avatar.clone()).collect();
                        if !append {
                            list.items.clear();
                        }
                        list.items.extend(page.items);
                        list.cursor = page.cursor;
                        list.error = None;
                        Update::event(Event::Pictures(urls))
                    }
                    Err(e) => {
                        list.error = Some(e.friendly());
                        Update::event(Event::Failed(e))
                    }
                }
            }
            Msg::Profile(did) => Update::event(Event::OpenProfile(did)),
            Msg::Mute(did, on) => Update::event(Event::Mute(did, on)),
            Msg::Block(did, on) => Update::event(Event::Block(did, on)),
            Msg::SignInAgain => Update::event(Event::SignInAgain),
        }
    }

    fn load(&mut self, actor: String, kind: PeopleKind, append: bool, api: &impl PeopleApi) -> Out {
        let list: &mut List = self.lists.entry((actor.clone(), kind)).or_default();
        if list.loading || (append && list.cursor.is_none()) {
            return Update::none();
        }
        list.loading = true;
        list.error = None;
        let cursor = if append { list.cursor.clone() } else { None };
        let api = api.clone();
        let a = actor.clone();
        Update::task(Task::perform(
            async move { api.people(&a, kind, cursor).await },
            move |r| Msg::Loaded(actor.clone(), kind, append, r),
        ))
    }
}
