//! Making each change: showing it at once, saving it, and putting it back if the
//! server refuses.

use super::Actions;
use super::change::{ActorChange, PostChange};
use super::message::{Event, Known, Made, Msg, Toggle, Undo};
use crate::api::service::ActionsApi;
use crate::features::Update;
use crate::model::{AtUri, Did, Mine, StrongRef};
use iced::Task;

type Out = Update<Msg, Event>;

impl Actions {
    pub fn update(&mut self, msg: Msg, known: &impl Known, api: &impl ActionsApi) -> Out {
        match msg {
            Msg::Like(uri, toggle) => record(uri, true, toggle, known, api),
            Msg::Repost(uri, toggle) => record(uri, false, toggle, known, api),
            Msg::Bookmark(uri, toggle) => {
                let Some(post) = known.post(&uri) else {
                    return Update::none();
                };
                let on = toggle.on();
                if post.viewer.bookmarked == on {
                    return Update::none();
                }
                let (api, subject) = (api.clone(), strong(&post.uri, &post.cid));
                let undo = Undo::Post(
                    uri.clone(),
                    PostChange::Bookmarked(!on),
                    if on {
                        "bookmark the post"
                    } else {
                        "remove the bookmark"
                    },
                );
                write(
                    Update::event(Event::Post(uri, PostChange::Bookmarked(on))),
                    undo,
                    async move {
                        if on {
                            api.bookmark(&subject).await
                        } else {
                            api.unbookmark(&subject.uri).await
                        }
                    },
                )
            }
            Msg::MuteThread(root, toggle) => {
                let on = toggle.on();
                let api = api.clone();
                let undo = Undo::Thread(
                    root.clone(),
                    PostChange::ThreadMuted(!on),
                    if on {
                        "mute the thread"
                    } else {
                        "unmute the thread"
                    },
                );
                let toast = if on {
                    "You won't get notifications from this thread."
                } else {
                    "Thread unmuted."
                };
                let shown = Update::event(Event::Thread(root.clone(), PostChange::ThreadMuted(on)))
                    .and(Event::Toast(toast.into()));
                write(shown, undo, async move { api.mute_thread(&root, on).await })
            }
            Msg::Pin(uri, toggle) => {
                let Some(post) = known.post(&uri) else {
                    return Update::none();
                };
                let pin = toggle.on().then(|| strong(&post.uri, &post.cid));
                let api = api.clone();
                write(
                    Update::event(Event::ShowPinned(pin.clone())),
                    Undo::Pin(known.pinned()),
                    async move { api.pin(pin).await },
                )
            }
            Msg::AskDelete(uri) => {
                self.confirm_delete = Some(uri);
                Update::none()
            }
            Msg::Delete(uri) => {
                self.confirm_delete = None;
                let api = api.clone();
                let u = uri.clone();
                write(
                    Update::event(Event::Remove(uri.clone())),
                    Undo::Delete(uri),
                    async move { api.delete(&u).await },
                )
            }
            Msg::Follow(did, toggle) => relation(did, true, toggle, known, api),
            Msg::Block(did, toggle) => relation(did, false, toggle, known, api),
            Msg::Mute(did, toggle) => {
                let on = toggle.on();
                let was = known.actor(&did).is_some_and(|v| v.muted);
                let api = api.clone();
                let undo = Undo::Actor(
                    did.clone(),
                    ActorChange::Muted(was),
                    if on { "mute" } else { "unmute" },
                );
                let d = did.clone();
                write(
                    Update::event(Event::Actor(did, ActorChange::Muted(on))),
                    undo,
                    async move { api.mute(&d, on).await },
                )
            }
            Msg::Created(made, Ok(uri)) => Update::event(match made {
                Made::Like(post) => {
                    Event::Post(post, PostChange::Like(Some(Mine::Saved(uri)), None))
                }
                Made::Repost(post) => {
                    Event::Post(post, PostChange::Repost(Some(Mine::Saved(uri)), None))
                }
                Made::Follow(did) => {
                    Event::Actor(did, ActorChange::Following(Some(Mine::Saved(uri))))
                }
                Made::Block(did) => {
                    Event::Actor(did, ActorChange::Blocking(Some(Mine::Saved(uri))))
                }
            }),
            Msg::Created(made, Err(e)) => {
                let (event, action) = match made {
                    Made::Like(post) => (
                        Event::Post(post, PostChange::Like(None, Some(false))),
                        "like",
                    ),
                    Made::Repost(post) => (
                        Event::Post(post, PostChange::Repost(None, Some(false))),
                        "repost",
                    ),
                    Made::Follow(did) => {
                        (Event::Actor(did, ActorChange::Following(None)), "follow")
                    }
                    Made::Block(did) => (Event::Actor(did, ActorChange::Blocking(None)), "block"),
                };
                Update::event(event).and(Event::Failed(action, e))
            }
            Msg::Done(Undo::Delete(uri), Ok(())) => Update::event(Event::Removed(uri)),
            Msg::Done(Undo::Pin(_), Ok(())) => Update::event(Event::PinSaved),
            Msg::Done(_, Ok(())) => Update::none(),
            Msg::Done(undo, Err(e)) => {
                let (event, action) = match undo {
                    Undo::Post(uri, change, action) => (Event::Post(uri, change), action),
                    Undo::Thread(root, change, action) => (Event::Thread(root, change), action),
                    Undo::Actor(did, change, action) => (Event::Actor(did, change), action),
                    Undo::Delete(uri) => (Event::PutBack(uri), "delete the post"),
                    Undo::Pin(before) => (Event::ShowPinned(before), "change the pinned post"),
                };
                Update::event(event).and(Event::Failed(action, e))
            }
        }
    }
}

fn strong(uri: &AtUri, cid: &str) -> StrongRef {
    StrongRef {
        uri: uri.clone(),
        cid: cid.to_string(),
    }
}

/// Shows a change at once, then saves it, putting it back with `undo` if refused.
fn write(
    shown: Out,
    undo: Undo,
    save: impl Future<Output = Result<(), crate::api::ApiError>> + Send + 'static,
) -> Out {
    Out {
        task: Task::perform(save, move |r| Msg::Done(undo.clone(), r)),
        events: shown.events,
    }
}

/// Likes (`like`) or reposts a post, or takes the like or repost back.
fn record(
    uri: AtUri,
    like: bool,
    toggle: Toggle,
    known: &impl Known,
    api: &impl ActionsApi,
) -> Out {
    let Some(post) = known.post(&uri) else {
        return Update::none();
    };
    let change = |mine, up| {
        if like {
            PostChange::Like(mine, up)
        } else {
            PostChange::Repost(mine, up)
        }
    };
    let existing = if like {
        post.viewer.like
    } else {
        post.viewer.repost
    };
    let api = api.clone();
    match (toggle, existing) {
        (Toggle::On, None) => {
            let subject = strong(&post.uri, &post.cid);
            let made = if like {
                Made::Like(uri.clone())
            } else {
                Made::Repost(uri.clone())
            };
            Out {
                task: Task::perform(
                    async move {
                        if like {
                            api.like(&subject).await
                        } else {
                            api.repost(&subject).await
                        }
                    },
                    move |r| Msg::Created(made.clone(), r),
                ),
                events: vec![Event::Post(uri, change(Some(Mine::Saving), Some(true)))],
            }
        }
        // Still being saved: there is no record to delete yet.
        (Toggle::Off, Some(Mine::Saved(record))) => {
            let action = if like { "unlike" } else { "undo the repost" };
            let undo = Undo::Post(
                uri.clone(),
                change(Some(Mine::Saved(record.clone())), Some(true)),
                action,
            );
            write(
                Update::event(Event::Post(uri, change(None, Some(false)))),
                undo,
                async move { api.delete(&record).await },
            )
        }
        _ => Update::none(),
    }
}

/// Follows (`follow`) or blocks an account, or undoes it.
fn relation(
    did: Did,
    follow: bool,
    toggle: Toggle,
    known: &impl Known,
    api: &impl ActionsApi,
) -> Out {
    let change = |mine| {
        if follow {
            ActorChange::Following(mine)
        } else {
            ActorChange::Blocking(mine)
        }
    };
    let api = api.clone();
    match toggle {
        Toggle::On => {
            let made = if follow {
                Made::Follow(did.clone())
            } else {
                Made::Block(did.clone())
            };
            let d = did.clone();
            Out {
                task: Task::perform(
                    async move {
                        if follow {
                            api.follow(&d).await
                        } else {
                            api.block(&d).await
                        }
                    },
                    move |r| Msg::Created(made.clone(), r),
                ),
                events: vec![Event::Actor(did, change(Some(Mine::Saving)))],
            }
        }
        Toggle::Off => {
            let viewer = known.actor(&did);
            let existing = viewer.and_then(|v| if follow { v.following } else { v.blocking });
            let Some(Mine::Saved(record)) = existing else {
                return Update::none();
            };
            let action = if follow { "unfollow" } else { "unblock" };
            let undo = Undo::Actor(
                did.clone(),
                change(Some(Mine::Saved(record.clone()))),
                action,
            );
            write(
                Update::event(Event::Actor(did, change(None))),
                undo,
                async move { api.delete(&record).await },
            )
        }
    }
}
