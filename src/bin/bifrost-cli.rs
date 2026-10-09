//! Command-line account tool, kept alongside the GUI for scripting and debugging.
//!
//!   bifrost-cli login <handle> [--service URL]   prompts for an app password
//!   bifrost-cli whoami                           restores and validates the active session
//!   bifrost-cli accounts | switch <handle> | refresh | logout [did]

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a command-line tool talks through stdout and stderr"
)]

use bifrost::auth::atrium::AtriumBackend;
use bifrost::auth::store::IndexLocation;
use bifrost::auth::{AccountManager, AuthError, Credentials, DEFAULT_SERVICE};
use bifrost::storage::secrets::KeyringStore;
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

async fn run(args: Vec<String>) -> Result<(), AuthError> {
    let manager = AccountManager::new(
        AtriumBackend::new(),
        KeyringStore::default(),
        IndexLocation::default_file()?,
    );
    let args: Vec<&str> = args.iter().map(String::as_str).collect();

    match args.as_slice() {
        ["login", identifier, rest @ ..] => {
            let service = match rest {
                ["--service", url] => url.to_string(),
                _ => DEFAULT_SERVICE.to_string(),
            };
            let password = prompt_secret("App password: ")?;
            let mut creds = Credentials {
                identifier: identifier.to_string(),
                password,
                auth_factor_token: None,
                service,
            };
            let session = match manager.login(&creds).await {
                Err(AuthError::AuthFactorRequired) => {
                    creds.auth_factor_token = Some(prompt_secret("Code sent to your email: ")?);
                    manager.login(&creds).await?
                }
                other => other?,
            };
            println!("Logged in as @{} ({})", session.handle, session.did);
        }
        ["whoami"] => {
            let s = manager.restore().await?;
            println!("@{} ({}) on {}", s.handle, s.did, s.pds_url);
        }
        ["accounts"] => {
            for a in manager.accounts()? {
                println!(
                    "{} @{} {}",
                    if a.active { "*" } else { " " },
                    a.handle,
                    a.did
                );
            }
        }
        ["switch", who] => manager.switch(who)?,
        ["refresh"] => {
            let s = manager.refresh().await?;
            println!("Refreshed @{}", s.handle);
        }
        ["logout"] => manager.logout(None).await?,
        ["logout", did] => manager.logout(Some(did)).await?,
        _ => eprintln!(
            "usage: bifrost-cli login <handle> [--service URL] | whoami | accounts | switch <handle> | refresh | logout [did]"
        ),
    }
    Ok(())
}

fn prompt_secret(prompt: &str) -> Result<String, AuthError> {
    rpassword::prompt_password(prompt)
        .map_err(|e| AuthError::Transport(format!("reading input: {e}")))
}
