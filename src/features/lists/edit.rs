//! Handling the lists feature's messages for making, editing and deleting lists and
//! packs, and for "Add to lists".

use super::message::{Ctx, Event, Msg};
use super::state::{AddTo, Editing, Editor, Lists, MAX_DESCRIPTION, MAX_NAME};
use super::{Out, effects};
use crate::api::service::ListsApi;
use crate::features::Update;
use crate::model::{CURATE, ListDetails, MAX_PACK_FEEDS, MODLIST};
use iced::Task;
use std::collections::HashSet;

impl Lists {
    pub(super) fn edit_update<A: ListsApi>(&mut self, msg: Msg, ctx: &Ctx<'_>, api: &A) -> Out {
        match msg {
            Msg::NewList(moderation) => {
                self.editor = Some(Editor {
                    moderation,
                    ..Editor::new(Editing::NewList, String::new(), String::new())
                });
                Update::none()
            }
            Msg::NewPack => {
                let name = ctx
                    .my_name
                    .map(|n| format!("{n}'s starter pack"))
                    .unwrap_or_default();
                self.editor = Some(Editor::new(Editing::NewPack, name, String::new()));
                Update::none()
            }
            Msg::Edit(uri) => {
                if let Some(l) = self.list(&uri) {
                    let description = l.description.clone().unwrap_or_default();
                    self.editor = Some(Editor {
                        moderation: l.is_mod(),
                        ..Editor::new(Editing::List(uri.clone()), l.name.clone(), description)
                    });
                }
                Update::none()
            }
            Msg::EditPack(uri) => {
                if let Some(Ok(p)) = self.packs.get(&uri) {
                    let kind = Editing::Pack {
                        uri: uri.clone(),
                        list: p.list.as_ref().map(|l| l.uri.clone()),
                    };
                    let description = p.record.description.clone().unwrap_or_default();
                    self.editor = Some(Editor {
                        feeds: p.record.feeds.iter().map(|f| f.uri.clone()).collect(),
                        ..Editor::new(kind, p.record.name.clone(), description)
                    });
                }
                Update::none()
            }
            Msg::Name(n) => {
                if let Some(e) = &mut self.editor {
                    e.name = n.chars().take(MAX_NAME).collect();
                }
                Update::none()
            }
            Msg::Description(d) => {
                if let Some(e) = &mut self.editor {
                    e.description = d.chars().take(MAX_DESCRIPTION).collect();
                }
                Update::none()
            }
            Msg::ToggleFeed(f) => {
                if let Some(e) = &mut self.editor {
                    if let Some(i) = e.feeds.iter().position(|x| *x == f) {
                        e.feeds.remove(i);
                    } else if e.feeds.len() < MAX_PACK_FEEDS {
                        e.feeds.push(f);
                    }
                }
                Update::none()
            }
            Msg::Save => {
                let Some(e) = &mut self.editor else {
                    return Update::none();
                };
                if e.saving || e.name.trim().is_empty() {
                    return Update::none();
                }
                e.saving = true;
                e.error = None;
                let details = ListDetails {
                    name: e.name.clone(),
                    description: e.description.clone(),
                    purpose: if e.moderation { MODLIST } else { CURATE }.into(),
                };
                Update::task(Task::perform(
                    effects::save(api.clone(), e.kind.clone(), details, e.feeds.clone()),
                    Msg::Saved,
                ))
            }
            Msg::Saved(Ok(uri)) => {
                let Some(e) = self.editor.take() else {
                    return Update::none();
                };
                // Your lists and packs changed; fetch them again when next shown.
                self.actor_packs.remove(&**ctx.me);
                let done = Update::event(Event::MyListsChanged);
                match e.kind {
                    Editing::NewList => done.and(Event::OpenList(uri)),
                    Editing::NewPack => done
                        .and(Event::Toast(
                            "Starter pack made. Add people from its page.".into(),
                        ))
                        .and(Event::OpenPack(uri)),
                    Editing::List(uri) => {
                        self.screens.remove(&uri);
                        Update {
                            task: self.ensure_list(api, uri),
                            ..done
                        }
                    }
                    Editing::Pack { uri, list } => {
                        self.packs.remove(&uri);
                        if let Some(l) = list {
                            self.screens.remove(&l);
                        }
                        Update {
                            task: self.load_pack(api, uri),
                            ..done
                        }
                    }
                }
            }
            Msg::Saved(Err(err)) => {
                if let Some(e) = &mut self.editor {
                    e.saving = false;
                    e.error = Some(err.friendly());
                }
                Update::event(Event::ErrorShown(err))
            }
            Msg::CloseEditor => {
                if !self.editor.as_ref().is_some_and(|e| e.saving) {
                    self.editor = None;
                }
                Update::none()
            }

            Msg::Delete(uri) => {
                self.confirm_delete = Some(uri);
                Update::none()
            }
            Msg::CancelDelete => {
                self.confirm_delete = None;
                Update::none()
            }
            Msg::ConfirmDelete => {
                let Some(uri) = self.confirm_delete.take() else {
                    return Update::none();
                };
                let pack_list = match self.packs.get(&uri) {
                    Some(Ok(p)) => p.list.as_ref().map(|l| l.uri.clone()),
                    _ => None,
                };
                Update::task(Task::perform(
                    effects::delete(api.clone(), uri.clone(), pack_list),
                    move |r| Msg::Deleted(uri.clone(), r),
                ))
            }
            Msg::Deleted(uri, Ok(())) => {
                self.actor_packs.remove(&**ctx.me);
                self.screens.remove(&uri);
                self.packs.remove(&uri);
                let toast = if super::state::is_pack(&uri) {
                    "Starter pack deleted."
                } else {
                    "List deleted."
                };
                Update::event(Event::MyListsChanged)
                    .and(Event::Toast(toast.into()))
                    .and(Event::Deleted(uri))
            }
            Msg::Deleted(_, Err(e)) => Update::event(Event::Error(e)),

            Msg::AddTo(did, name) => {
                self.add_to = Some(AddTo {
                    did: did.clone(),
                    name,
                    rows: None,
                    busy: HashSet::new(),
                });
                let api = api.clone();
                let d = did.clone();
                Update::task(Task::perform(
                    async move { api.my_lists_for(&d).await },
                    move |r| Msg::Memberships(did.clone(), r),
                ))
            }
            Msg::Memberships(did, result) => {
                let Some(a) = self.add_to.as_mut().filter(|a| a.did == did) else {
                    return Update::none();
                };
                match result {
                    Ok(rows) => {
                        a.rows = Some(Ok(rows));
                        Update::none()
                    }
                    Err(e) => {
                        a.rows = Some(Err(e.friendly()));
                        Update::event(Event::ErrorShown(e))
                    }
                }
            }
            Msg::Toggle(list) => {
                let Some(a) = &mut self.add_to else {
                    return Update::none();
                };
                let Some(Ok(rows)) = &a.rows else {
                    return Update::none();
                };
                let Some(row) = rows.iter().find(|r| r.list.uri == list) else {
                    return Update::none();
                };
                if !a.busy.insert(list.clone()) {
                    return Update::none();
                }
                Update::task(Task::perform(
                    effects::toggle(api.clone(), list.clone(), a.did.clone(), row.item.clone()),
                    move |r| Msg::Toggled(list.clone(), r),
                ))
            }
            Msg::Toggled(list, result) => {
                if let Some(a) = &mut self.add_to {
                    a.busy.remove(&list);
                    if let (Ok(item), Some(Ok(rows))) = (&result, &mut a.rows)
                        && let Some(row) = rows.iter_mut().find(|r| r.list.uri == list)
                    {
                        row.item.clone_from(item);
                    }
                }
                // The list's own screen is out of date now.
                self.screens.remove(&list);
                match result {
                    Ok(_) => Update::none(),
                    Err(e) => Update::event(Event::Error(e)),
                }
            }
            Msg::CloseAddTo => {
                self.add_to = None;
                Update::none()
            }
            // The rest are handled in `update`.
            _ => Update::none(),
        }
    }
}
