//! Actions: each change shows at once, is saved, and is put back if refused.

use super::*;
use crate::api::ApiError;
use crate::api::service::ActionsApi;
use crate::model::{ActorViewer, AtUri, Did, Interaction, Mine, PostView, StrongRef};
use serde_json::json;
use std::future::{Ready, ready};

/// Writes are never run in these tests; they only count as tasks.
#[derive(Clone)]
struct Fake;

fn nothing<T>() -> Ready<Result<T, ApiError>> {
    ready(Err(ApiError::Other("not run in tests".into())))
}

impl ActionsApi for Fake {
    fn like(&self, _: &StrongRef) -> impl Future<Output = Result<AtUri, ApiError>> + Send {
        nothing()
    }
    fn repost(&self, _: &StrongRef) -> impl Future<Output = Result<AtUri, ApiError>> + Send {
        nothing()
    }
    fn delete(&self, _: &AtUri) -> impl Future<Output = Result<(), ApiError>> + Send {
        nothing()
    }
    fn bookmark(&self, _: &StrongRef) -> impl Future<Output = Result<(), ApiError>> + Send {
        nothing()
    }
    fn unbookmark(&self, _: &AtUri) -> impl Future<Output = Result<(), ApiError>> + Send {
        nothing()
    }
    fn mute_thread(&self, _: &AtUri, _: bool) -> impl Future<Output = Result<(), ApiError>> + Send {
        nothing()
    }
    fn pin(&self, _: Option<StrongRef>) -> impl Future<Output = Result<(), ApiError>> + Send {
        nothing()
    }
    fn follow(&self, _: &str) -> impl Future<Output = Result<AtUri, ApiError>> + Send {
        nothing()
    }
    fn block(&self, _: &str) -> impl Future<Output = Result<AtUri, ApiError>> + Send {
        nothing()
    }
    fn mute(&self, _: &str, _: bool) -> impl Future<Output = Result<(), ApiError>> + Send {
        nothing()
    }
    fn post_interaction(
        &self,
        _: &str,
    ) -> impl Future<Output = Result<Interaction, ApiError>> + Send {
        nothing()
    }
    fn set_post_interaction(
        &self,
        _: &str,
        _: Interaction,
        _: bool,
    ) -> impl Future<Output = Result<(), ApiError>> + Send {
        nothing()
    }
}

/// One post and one account, as the app has them cached.
struct Cache {
    post: PostView,
    actor: ActorViewer,
}

impl Known for Cache {
    fn post(&self, uri: &AtUri) -> Option<PostView> {
        (self.post.uri == *uri).then(|| self.post.clone())
    }
    fn actor(&self, _: &Did) -> Option<ActorViewer> {
        Some(self.actor.clone())
    }
    fn pinned(&self) -> Option<StrongRef> {
        None
    }
}

const URI: &str = "at://did:plc:a/app.bsky.feed.post/1";

fn cache() -> Cache {
    let post = serde_json::from_value(json!({
        "uri": URI,
        "cid": "c",
        "author": {"did": "did:plc:a", "handle": "a.test"},
        "record": {"text": "hi", "createdAt": "2026-01-01T00:00:00Z"},
        "likeCount": 3,
    }))
    .unwrap();
    Cache {
        post,
        actor: ActorViewer::default(),
    }
}

/// Runs `msg` and applies what it reports to the cache, as the app does.
fn run(a: &mut Actions, c: &mut Cache, msg: Msg) -> (usize, Vec<Event>) {
    let out = a.update(msg, c, &Fake);
    for e in &out.events {
        match e {
            Event::Post(_, change) | Event::Thread(_, change) => change.apply(&mut c.post),
            Event::Actor(_, change) => change.apply(&mut c.actor),
            _ => {}
        }
    }
    (out.task.units(), out.events)
}

fn uri() -> AtUri {
    AtUri::from(URI)
}

#[test]
fn a_like_shows_at_once_and_waits_for_its_record() {
    let (mut a, mut c) = (Actions::default(), cache());
    let (tasks, _) = run(&mut a, &mut c, Msg::Like(uri(), Toggle::On));
    assert_eq!(tasks, 1);
    assert_eq!(c.post.viewer.like, Some(Mine::Saving));
    assert_eq!(c.post.like_count, 4);
    // Unliking before the like is saved does nothing: there is no record yet.
    let (tasks, events) = run(&mut a, &mut c, Msg::Like(uri(), Toggle::Off));
    assert_eq!((tasks, events.len()), (0, 0));
    let record = AtUri::from("at://did:plc:me/app.bsky.feed.like/1");
    let _ = run(
        &mut a,
        &mut c,
        Msg::Created(message::Made::Like(uri()), Ok(record.clone())),
    );
    assert_eq!(c.post.viewer.like, Some(Mine::Saved(record)));
    assert_eq!(c.post.like_count, 4);
}

#[test]
fn a_refused_like_is_taken_back_and_reported() {
    let (mut a, mut c) = (Actions::default(), cache());
    let _ = run(&mut a, &mut c, Msg::Like(uri(), Toggle::On));
    let (_, events) = run(
        &mut a,
        &mut c,
        Msg::Created(
            message::Made::Like(uri()),
            Err(ApiError::Other("no".into())),
        ),
    );
    assert_eq!(c.post.viewer.like, None);
    assert_eq!(c.post.like_count, 3);
    assert!(matches!(events.last(), Some(Event::Failed("like", _))));
}

#[test]
fn a_refused_unlike_puts_the_like_back() {
    let (mut a, mut c) = (Actions::default(), cache());
    let record = AtUri::from("at://did:plc:me/app.bsky.feed.like/1");
    c.post.viewer.like = Some(Mine::Saved(record.clone()));
    let out = a.update(Msg::Like(uri(), Toggle::Off), &c, &Fake);
    for e in &out.events {
        if let Event::Post(_, ch) = e {
            ch.apply(&mut c.post);
        }
    }
    assert_eq!((c.post.viewer.like.clone(), c.post.like_count), (None, 2));
    // The task would answer with Done; a refusal puts it back.
    let undo = message::Undo::Post(
        uri(),
        PostChange::Like(Some(Mine::Saved(record.clone())), Some(true)),
        "unlike",
    );
    let (_, events) = run(
        &mut a,
        &mut c,
        Msg::Done(undo, Err(ApiError::Other("no".into()))),
    );
    assert_eq!(c.post.viewer.like, Some(Mine::Saved(record)));
    assert_eq!(c.post.like_count, 3);
    assert!(matches!(events.last(), Some(Event::Failed("unlike", _))));
}

#[test]
fn following_muting_and_bookmarking() {
    let (mut a, mut c) = (Actions::default(), cache());
    let did = Did::from("did:plc:a");
    let _ = run(&mut a, &mut c, Msg::Follow(did.clone(), Toggle::On));
    assert_eq!(c.actor.following, Some(Mine::Saving));
    let _ = run(&mut a, &mut c, Msg::Mute(did, Toggle::On));
    assert!(c.actor.muted);
    let _ = run(&mut a, &mut c, Msg::Bookmark(uri(), Toggle::On));
    assert!(c.post.viewer.bookmarked);
    // Already bookmarked: nothing to do.
    let (tasks, _) = run(&mut a, &mut c, Msg::Bookmark(uri(), Toggle::On));
    assert_eq!(tasks, 0);
}

#[test]
fn deleting_asks_first_and_puts_back_on_refusal() {
    let (mut a, mut c) = (Actions::default(), cache());
    let _ = run(&mut a, &mut c, Msg::AskDelete(uri()));
    assert_eq!(a.confirm_delete(), Some(&uri()));
    let (tasks, events) = run(&mut a, &mut c, Msg::Delete(uri()));
    assert_eq!(tasks, 1);
    assert!(matches!(&events[..], [Event::Remove(u)] if *u == uri()));
    assert_eq!(a.confirm_delete(), None);
    let (_, events) = run(
        &mut a,
        &mut c,
        Msg::Done(
            message::Undo::Delete(uri()),
            Err(ApiError::Other("no".into())),
        ),
    );
    assert!(matches!(
        &events[..],
        [Event::PutBack(_), Event::Failed("delete the post", _)]
    ));
}

#[test]
fn muting_a_thread_says_so() {
    let (mut a, mut c) = (Actions::default(), cache());
    let (_, events) = run(&mut a, &mut c, Msg::MuteThread(uri(), Toggle::On));
    assert!(c.post.viewer.thread_muted);
    assert!(events.iter().any(|e| matches!(e, Event::Toast(_))));
}

#[test]
fn who_can_reply_loads_saves_and_reports_the_new_rules() {
    use crate::model::ReplyRules;
    let c = cache();
    let mut editing = None;
    let out = who::update(&mut editing, who::Msg::Open(uri()), &c, &Fake);
    assert!(matches!(out.events[..], [who::Event::NeedMyLists]));
    assert!(
        editing
            .as_ref()
            .is_some_and(|e| e.is_root && e.value.is_none())
    );
    // Nothing to save until the settings have loaded.
    let _ = who::update(&mut editing, who::Msg::Save, &c, &Fake);
    assert!(editing.as_ref().is_some_and(|e| !e.saving));

    let loaded = who::Msg::Loaded(uri(), Ok(Interaction::default()));
    let _ = who::update(&mut editing, loaded, &c, &Fake);
    let nobody = Interaction {
        replies: Some(ReplyRules::default()),
        quotes: true,
    };
    let _ = who::update(&mut editing, who::Msg::Changed(nobody), &c, &Fake);
    let out = who::update(&mut editing, who::Msg::Save, &c, &Fake);
    assert_eq!(out.task.units(), 1);
    // While saving it can't be closed or changed.
    let _ = who::update(&mut editing, who::Msg::Close, &c, &Fake);
    assert!(editing.as_ref().is_some_and(|e| e.saving));

    let out = who::update(&mut editing, who::Msg::Saved(Ok(())), &c, &Fake);
    assert!(editing.is_none());
    assert!(matches!(
        out.events[..],
        [who::Event::Gate(_, Some(_)), who::Event::Toast(_)]
    ));
}
