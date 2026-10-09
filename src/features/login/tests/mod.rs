//! The login form: what each way of signing in asks the app to do, and what the
//! form shows after a failed attempt.

use super::*;
use crate::api::{ApiError, DEFAULT_SERVICE};
use iced_test::simulator;

fn typed(login: &mut Login, msgs: impl IntoIterator<Item = Msg>) {
    for m in msgs {
        assert!(login.update(m).is_none());
    }
}

#[test]
fn submit_needs_a_handle_and_a_password() {
    let mut login = Login::default();
    assert!(login.update(Msg::Submit).is_none());
    typed(&mut login, [Msg::Identifier(" @alice.test ".into())]);
    assert!(login.update(Msg::Submit).is_none());
    typed(&mut login, [Msg::Password("abcd-efgh".into())]);
    let Some(Event::Password(creds)) = login.update(Msg::Submit) else {
        panic!("expected a password sign-in");
    };
    assert_eq!(creds.identifier, "alice.test");
    assert_eq!(creds.password, "abcd-efgh");
    assert_eq!(creds.service, DEFAULT_SERVICE);
    assert_eq!(creds.auth_factor_token, None);
    // A second press while signing in does nothing.
    assert!(login.update(Msg::Submit).is_none());
}

#[test]
fn plain_http_servers_are_refused() {
    let mut login = Login::default();
    typed(
        &mut login,
        [
            Msg::Identifier("alice.test".into()),
            Msg::Password("pw".into()),
            Msg::Service("http://pds.example".into()),
        ],
    );
    assert!(login.update(Msg::Submit).is_none());
    assert!(
        login
            .error
            .as_deref()
            .is_some_and(|e| e.contains("https://"))
    );
}

#[test]
fn the_browser_signs_in_with_the_handle_or_the_server() {
    let mut login = Login::default();
    assert!(matches!(
        login.update(Msg::Browser),
        Some(Event::Browser(s)) if s == DEFAULT_SERVICE
    ));
    assert!(login.update(Msg::Browser).is_none(), "already waiting");
    assert!(matches!(
        login.update(Msg::CancelBrowser),
        Some(Event::CancelBrowser)
    ));
    typed(&mut login, [Msg::Identifier("bob.test".into())]);
    assert!(matches!(
        login.update(Msg::Browser),
        Some(Event::Browser(s)) if s == "bob.test"
    ));
}

#[test]
fn an_email_code_is_asked_for_and_sent_along() {
    let mut login = Login::default();
    typed(
        &mut login,
        [
            Msg::Identifier("alice.test".into()),
            Msg::Password("pw".into()),
        ],
    );
    let _ = login.update(Msg::Submit);
    login.failed(&ApiError::NeedsCode);
    let mut ui = simulator(login.view());
    assert!(ui.find("Email sign-in code").is_ok());
    drop(ui);
    typed(&mut login, [Msg::Code(" 12345 ".into())]);
    let Some(Event::Password(creds)) = login.update(Msg::Submit) else {
        panic!("expected a password sign-in");
    };
    assert_eq!(creds.auth_factor_token.as_deref(), Some("12345"));
}

#[test]
fn the_form_shows_why_and_its_buttons_send_messages() {
    let mut login = Login::expired();
    login.failed(&ApiError::Auth("Wrong password.".into()));
    let mut ui = simulator(login.view());
    assert!(
        ui.find("Your session expired. Please log in again.")
            .is_ok()
    );
    assert!(ui.find("Wrong password.").is_ok());
    assert!(ui.find("Email sign-in code").is_err());
    ui.click("Sign in with your browser").unwrap();
    ui.click("Other hosting provider").unwrap();
    assert!(matches!(
        &ui.into_messages().collect::<Vec<_>>()[..],
        [Msg::Browser, Msg::ToggleAdvanced]
    ));
}
