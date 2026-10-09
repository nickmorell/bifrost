//! Preference changes: shown at once, saved one at a time, re-read if refused.

use super::*;
use std::future::ready;

/// Saves are never run in these tests; they only count as tasks.
#[derive(Clone)]
struct Fake;

impl PrefsApi for Fake {
    fn preferences(&self) -> impl Future<Output = Result<Preferences, ApiError>> + Send {
        ready(Ok(Preferences::default()))
    }
    fn edit_preferences(
        &self,
        _: Vec<PrefEdit>,
    ) -> impl Future<Output = Result<Preferences, ApiError>> + Send {
        ready(Ok(Preferences::default()))
    }
}

fn changed(out: &Out) -> bool {
    matches!(out.events.as_slice(), [Event::Changed])
}

#[test]
fn an_edit_shows_at_once_and_saves_one_at_a_time() {
    let (mut saver, mut prefs) = (Saver::default(), Preferences::default());
    let out = saver.update(Msg::Edit(PrefEdit::AdultContent(true)), &mut prefs, &Fake);
    assert!(changed(&out) && prefs.adult_content_enabled);
    assert!(saver.saving && saver.queue.is_empty());

    // A second edit while the first saves waits its turn.
    let out = saver.update(Msg::Edit(PrefEdit::MuteWord("x".into())), &mut prefs, &Fake);
    assert!(changed(&out));
    assert_eq!(saver.queue.len(), 1);
    assert_eq!(prefs.muted_words.len(), 1);

    // The first save comes back without the second edit; it stays shown and is sent.
    let out = saver.update(Msg::Saved(Ok(Preferences::default())), &mut prefs, &Fake);
    assert!(changed(&out));
    assert_eq!(prefs.muted_words.len(), 1);
    assert!(!prefs.adult_content_enabled);
    assert!(saver.saving && saver.queue.is_empty());
}

#[test]
fn a_refused_save_rereads_the_preferences() {
    let (mut saver, mut prefs) = (Saver::default(), Preferences::default());
    let _ = saver.update(Msg::Edit(PrefEdit::AdultContent(true)), &mut prefs, &Fake);
    let refused = Msg::Saved(Err(ApiError::Other("no".into())));
    let out = saver.update(refused, &mut prefs, &Fake);
    assert!(matches!(out.events.as_slice(), [Event::Failed(_)]));
    assert!(!saver.saving);

    let out = saver.update(Msg::Reloaded(Ok(Preferences::default())), &mut prefs, &Fake);
    assert!(changed(&out) && !prefs.adult_content_enabled);
}

#[test]
fn a_reread_does_not_overwrite_newer_edits() {
    let (mut saver, mut prefs) = (Saver::default(), Preferences::default());
    let _ = saver.update(Msg::Edit(PrefEdit::AdultContent(true)), &mut prefs, &Fake);
    let out = saver.update(Msg::Reloaded(Ok(Preferences::default())), &mut prefs, &Fake);
    assert!(out.events.is_empty() && prefs.adult_content_enabled);
}
