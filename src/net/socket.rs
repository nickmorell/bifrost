//! Websockets: a connection that stays open and brings text messages as they happen,
//! such as a live stream's chat. Only reading is needed; pings are answered as they
//! are read.

use super::Failure;
use futures_util::StreamExt;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

/// The largest message read; chat messages and stream updates are a few KB.
const MAX_MESSAGE: usize = 1024 * 1024;
/// How long connecting may take.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
/// A connection that sends nothing at all, not even a ping, for this long has
/// silently dropped (a sleeping laptop, a changed network) and is given up on.
const IDLE_TIMEOUT: Duration = Duration::from_secs(90);

/// An open websocket.
pub struct Socket(WebSocketStream<MaybeTlsStream<TcpStream>>);

impl std::fmt::Debug for Socket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Socket").finish_non_exhaustive()
    }
}

/// Opens a websocket to `url` (`wss://` or `ws://`).
pub async fn connect(url: &str) -> Result<Socket, Failure> {
    let mut request = url
        .into_client_request()
        .map_err(|e| Failure::Unreachable(e.to_string()))?;
    if let Ok(agent) = concat!("BiFrost/", env!("CARGO_PKG_VERSION")).parse() {
        request.headers_mut().insert("User-Agent", agent);
    }
    let config = WebSocketConfig::default()
        .max_message_size(Some(MAX_MESSAGE))
        .max_frame_size(Some(MAX_MESSAGE));
    let connecting = tokio_tungstenite::connect_async_with_config(request, Some(config), false);
    let (stream, _) = tokio::time::timeout(CONNECT_TIMEOUT, connecting)
        .await
        .map_err(|_| Failure::Unreachable("connecting took too long".into()))?
        .map_err(|e| Failure::Unreachable(e.to_string()))?;
    Ok(Socket(stream))
}

impl Socket {
    /// The next text message, or `None` once the connection has closed. A
    /// connection quiet for longer than [`IDLE_TIMEOUT`] gives an error.
    pub async fn next_text(&mut self) -> Option<Result<String, Failure>> {
        loop {
            let Ok(next) = tokio::time::timeout(IDLE_TIMEOUT, self.0.next()).await else {
                return Some(Err(Failure::Unreachable(
                    "the connection went quiet".into(),
                )));
            };
            match next? {
                Ok(Message::Text(text)) => return Some(Ok(text.to_string())),
                Ok(Message::Close(_)) => return None,
                // Pings are answered by the library; binary messages aren't used.
                Ok(_) => {}
                Err(e) => return Some(Err(Failure::Unreachable(e.to_string()))),
            }
        }
    }
}
