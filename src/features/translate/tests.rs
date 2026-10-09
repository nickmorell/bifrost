//! Translating posts and keeping keys, against a fake service.

use super::*;
use crate::api::service::TranslateApi;
use iced_test::simulator;
use serde_json::json;
use std::future::ready;

/// Calls are never run in these tests; they only count as tasks.
#[derive(Clone)]
struct Fake;

impl TranslateApi for Fake {
    fn translate(
        &self,
        _: Service,
        _: String,
        _: Option<String>,
        _: String,
        _: String,
    ) -> impl Future<Output = Result<Translation, ApiError>> + Send {
        ready(Err(ApiError::Other("not run".into())))
    }
    fn has_key(&self, _: Service) -> impl Future<Output = bool> + Send {
        ready(false)
    }
    fn set_key(&self, _: Service, _: String) -> impl Future<Output = Result<(), ApiError>> + Send {
        ready(Ok(()))
    }
}

fn ctx(service: Service) -> Ctx {
    Ctx {
        service,
        to: "en".into(),
        server: String::new(),
    }
}

fn post(text: &str, lang: &str) -> PostView {
    serde_json::from_value(json!({
        "uri": "at://did:plc:a/app.bsky.feed.post/1", "cid": "c",
        "author": {"did": "did:plc:a", "handle": "a.test"},
        "record": {"text": text, "langs": [lang], "createdAt": "2026-01-01T00:00:00Z"},
    }))
    .unwrap()
}

#[test]
fn only_posts_in_other_languages_offer_a_translation() {
    assert!(is_foreign(&post("hola", "es-MX"), "en"));
    assert!(!is_foreign(&post("hi", "en-GB"), "en"));
    assert!(!is_foreign(&post("  ", "es"), "en"));

    let (t, p) = (Translations::default(), post("hola", "es"));
    let mut ui = simulator(under_post(&t, &p, 15.0, &ctx(Service::MyMemory)).unwrap());
    ui.click("Translate").unwrap();
    assert!(matches!(
        ui.into_messages().collect::<Vec<_>>()[..],
        [Msg::Translate { ref lang, .. }] if lang.as_deref() == Some("es")
    ));
    assert!(under_post(&t, &post("hi", "en"), 15.0, &ctx(Service::MyMemory)).is_none());
}

#[test]
fn a_translation_loads_once_and_stays_hidden_if_hidden() {
    let (mut t, ctx) = (Translations::default(), ctx(Service::MyMemory));
    let p = post("hola", "es");
    assert_eq!(t.update(translate(&p), &ctx, &Fake).task.units(), 1);
    assert_eq!(
        t.update(translate(&p), &ctx, &Fake).task.units(),
        0,
        "already loading"
    );

    let done = |text: &str| {
        Msg::Done(
            p.uri.clone(),
            Ok(Translation {
                text: text.into(),
                from: Some("es".into()),
            }),
        )
    };
    let _ = t.update(done("hello"), &ctx, &Fake);
    {
        let mut ui = simulator(under_post(&t, &p, 15.0, &ctx).unwrap());
        assert!(ui.find("hello").is_ok());
        assert!(ui.find("Translated from Spanish by MyMemory").is_ok());
    }

    let _ = t.update(Msg::Hide(p.uri.clone()), &ctx, &Fake);
    let _ = t.update(done("late"), &ctx, &Fake);
    assert!(t.is_empty());
}

#[test]
fn the_browser_service_opens_google_translate() {
    let mut t = Translations::default();
    let out = t.update(
        translate(&post("hola", "es")),
        &ctx(Service::Browser),
        &Fake,
    );
    assert!(matches!(&out.events[..], [Event::OpenUrl(u)] if u.contains("tl=en")));
    assert!(t.is_empty());
}

#[test]
fn choosing_another_language_or_service_drops_old_translations() {
    let (mut t, c) = (Translations::default(), ctx(Service::MyMemory));
    let _ = t.update(translate(&post("hola", "es")), &c, &Fake);
    let out = t.update(Msg::To("fr".into()), &c, &Fake);
    assert_eq!(out.events, [Event::To("fr".into())]);
    assert!(t.is_empty());

    let _ = t.update(Msg::KeyInput("secret".into()), &c, &Fake);
    let out = t.update(Msg::Service(Service::DeepL), &c, &Fake);
    assert_eq!(out.events, [Event::Service(Service::DeepL)]);
    assert!(t.key_input.is_empty());
}

#[test]
fn saving_a_key_runs_off_the_window_thread_and_says_so() {
    let (mut t, c) = (Translations::default(), ctx(Service::DeepL));
    let _ = t.update(Msg::KeyInput("abc".into()), &c, &Fake);
    let out = t.update(Msg::SaveKey, &c, &Fake);
    assert_eq!((out.task.units(), out.events.len()), (1, 0));
    assert!(t.key_input.is_empty());

    let out = t.update(Msg::KeySaved(Service::DeepL, true, Ok(())), &c, &Fake);
    assert_eq!(out.events, [Event::Toast("DeepL key saved.".into())]);
    {
        let mut ui = simulator(settings(&t, &c));
        let saved = "API key (saved; type a new one to replace it, or save empty to remove it)";
        assert!(ui.find(saved).is_ok());
    }

    let out = t.update(
        Msg::KeySaved(Service::DeepL, false, Err(ApiError::Other("locked".into()))),
        &c,
        &Fake,
    );
    assert_eq!(
        out.events,
        [Event::Toast("Couldn't save the key: locked".into())]
    );
    assert_eq!(t.update(Msg::ReadKeys, &c, &Fake).task.units(), 1);
}
