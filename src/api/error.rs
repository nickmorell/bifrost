//! The one error type every API call returns. What went wrong is decided once, here,
//! from the server's status and error name; callers match on the variant and never
//! search the error's text.

use crate::auth::AuthError;
use atrium_api::xrpc::error::Error as XrpcError;
use std::fmt::{Debug, Display};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    Auth(String),
    #[error("session expired; please log in again")]
    SessionExpired,
    /// Signing in needs the code the server just emailed.
    #[error("a sign-in code was sent to the account's email")]
    NeedsCode,
    /// The sign-in was not given access to chats.
    #[error("this sign-in can't open your chats")]
    ChatAccess,
    /// The person only takes messages from some people, or has chats turned off.
    #[error("they only accept messages from some people")]
    ChatRefused,
    /// The post, account, feed or list asked for does not exist.
    #[error("not found: {0}")]
    NotFound(String),
    /// A block between you and the account.
    #[error("blocked")]
    Blocked,
    #[error("rate limited")]
    RateLimited,
    /// The server could not be reached at all.
    #[error("can't reach the server: {0}")]
    Unreachable(String),
    /// Any other answer outside 200-299: its status, error name and message.
    #[error("{status} {name} {message}")]
    Server {
        status: u16,
        name: String,
        message: String,
    },
    #[error("could not read the server's response: {0}")]
    Decode(String),
    #[error("{0}")]
    Other(String),
}

impl From<AuthError> for ApiError {
    fn from(e: AuthError) -> Self {
        match e {
            AuthError::SessionExpired | AuthError::NoActiveAccount => ApiError::SessionExpired,
            AuthError::AuthFactorRequired => ApiError::NeedsCode,
            other => ApiError::Auth(other.to_string()),
        }
    }
}

/// Error names the servers use for something that isn't there.
const NOT_FOUND: [&str; 6] = [
    "NotFound",
    "RecordNotFound",
    "UnknownFeed",
    "UnknownList",
    "ConvoNotFound",
    "ProfileNotFound",
];

impl ApiError {
    /// Classifies a server's answer by its status, its error name (such as
    /// "RecordNotFound") and its message.
    pub fn from_server(status: u16, name: &str, message: &str) -> Self {
        let name = name.split('(').next().unwrap_or_default();
        if NOT_FOUND.contains(&name)
            || name.ends_with("NotFound")
            || message.starts_with("Could not locate record")
            || (status == 404 && name.is_empty())
        {
            ApiError::NotFound(message.to_string())
        } else if matches!(name, "BlockedActor" | "BlockedByActor") {
            ApiError::Blocked
        } else if name == "RateLimitExceeded" || status == 429 {
            ApiError::RateLimited
        } else {
            ApiError::Server {
                status,
                name: name.to_string(),
                message: message.to_string(),
            }
        }
    }

    /// The error as a short sentence for the user.
    pub fn friendly(&self) -> String {
        match self {
            ApiError::NotFound(_) => "This no longer exists. It may have been deleted.".into(),
            ApiError::Blocked => "Not available because of a block.".into(),
            ApiError::RateLimited => {
                "Bluesky is rate limiting requests. Try again in a minute.".into()
            }
            ApiError::Unreachable(_) => "Can't reach Bluesky. Check your connection.".into(),
            ApiError::ChatRefused => "They only accept messages from some people.".into(),
            ApiError::Server {
                status,
                name,
                message,
            } => {
                if !message.is_empty() {
                    message.clone()
                } else if !name.is_empty() {
                    format!("Bluesky said {name} ({status}).")
                } else {
                    format!("Bluesky answered with an error ({status}).")
                }
            }
            other => other.to_string(),
        }
    }

    /// Whether the server said the thing asked for does not exist.
    pub fn is_not_found(&self) -> bool {
        matches!(self, ApiError::NotFound(_))
    }
}

/// A server's error answer: its status, error name and message.
pub(super) fn answer<E: Debug>(
    resp: &atrium_api::xrpc::error::XrpcError<E>,
) -> (u16, String, String) {
    use atrium_api::xrpc::error::XrpcErrorKind;
    let (name, message) = match &resp.error {
        Some(XrpcErrorKind::Undefined(body)) => (
            body.error.clone().unwrap_or_default(),
            body.message.clone().unwrap_or_default(),
        ),
        // Lexicon-defined errors, e.g. `NotFound(Some("..."))`.
        Some(XrpcErrorKind::Custom(e)) => (format!("{e:?}"), String::new()),
        None => (String::new(), String::new()),
    };
    (resp.status.as_u16(), name, message)
}

pub(super) fn xrpc_err<E: Debug + Display>(e: XrpcError<E>) -> ApiError {
    match e {
        XrpcError::XrpcResponse(resp) => {
            let (status, name, message) = answer(&resp);
            match crate::auth::atrium::map_err(XrpcError::XrpcResponse(resp)) {
                AuthError::Transport(_) => ApiError::from_server(status, &name, &message),
                signed_out => signed_out.into(),
            }
        }
        XrpcError::SerdeJson(e) => ApiError::Decode(e.to_string()),
        XrpcError::UnexpectedResponseType => ApiError::Decode(e.to_string()),
        XrpcError::HttpClient(e) => ApiError::Unreachable(e.to_string()),
        other => ApiError::Other(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_answers_are_classified_by_name_and_status() {
        let e = ApiError::from_server(400, "RecordNotFound", "Could not locate record");
        assert!(e.is_not_found());
        assert!(e.friendly().contains("no longer exists"));
        assert!(ApiError::from_server(400, "NotFound(Some(\"gone\"))", "").is_not_found());
        assert!(ApiError::from_server(400, "UnknownFeed", "").is_not_found());
        assert!(
            ApiError::from_server(400, "InvalidRequest", "Could not locate record: x")
                .is_not_found()
        );
        assert_eq!(
            ApiError::from_server(400, "BlockedActor", ""),
            ApiError::Blocked
        );
        assert_eq!(ApiError::from_server(429, "", ""), ApiError::RateLimited);
        assert_eq!(
            ApiError::from_server(400, "RateLimitExceeded", ""),
            ApiError::RateLimited
        );
        // A message that merely mentions a number is not a rate limit.
        let e = ApiError::from_server(400, "InvalidRequest", "post 429 is too long");
        assert!(matches!(e, ApiError::Server { status: 400, .. }));
        assert_eq!(e.friendly(), "post 429 is too long");
        assert!(!ApiError::Other("NotFound".into()).is_not_found());
    }

    #[test]
    fn auth_errors_carry_over() {
        assert_eq!(
            ApiError::from(AuthError::NoActiveAccount),
            ApiError::SessionExpired
        );
        assert_eq!(
            ApiError::from(AuthError::AuthFactorRequired),
            ApiError::NeedsCode
        );
    }
}
