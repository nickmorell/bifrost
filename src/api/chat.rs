//! Direct messages: Bluesky's chat service, reached through your PDS with the
//! `atproto-proxy` header.

use super::{
    AnyAgent, ApiError, Client, Debug, Display, PAGE_SIZE, StrongRef, Value, XrpcError, answer,
    conv, json, lenient_items, strong_ref, xrpc_err,
};
use crate::model::{AllowIncoming, ChatMessage, Convo, Did, Page};

/// Bluesky's chat service, which the PDS forwards chat calls to.
const CHAT_SERVICE: &str = "did:web:api.bsky.chat";
const CHAT_SERVICE_TYPE: &str = "bsky_chat";
/// Who can start a chat with you.
const DECLARATION: (&str, &str) = ("chat.bsky.actor.declaration", "self");

/// Chat errors. A refused token here means the sign-in was not given access to
/// chats, not that it expired, so it must not log the account out.
fn chat_err<E: Debug + Display>(e: XrpcError<E>) -> ApiError {
    if let XrpcError::XrpcResponse(resp) = &e {
        let (status, name, message) = answer(resp);
        if let Some(refused) = chat_refusal(status, &name, &message) {
            return refused;
        }
    }
    xrpc_err(e)
}

/// The chat service's own refusals: no access to chats, someone who doesn't take
/// messages from you, or a block. The service words the last two only in its message.
fn chat_refusal(status: u16, name: &str, message: &str) -> Option<ApiError> {
    let no_access = matches!(status, 401 | 403)
        || name == "InvalidToken"
        || message.contains("scope")
        || message.contains("Bad token");
    let message = message.to_lowercase();
    if no_access && name != "ExpiredToken" {
        Some(ApiError::ChatAccess)
    } else if name.starts_with("Blocked") || message.contains("block") {
        Some(ApiError::Blocked)
    } else if name.starts_with("NotFollowedBySender")
        || message.contains("recipient")
        || message.contains("disabled")
    {
        Some(ApiError::ChatRefused)
    } else {
        None
    }
}

#[cfg(test)]
#[test]
fn chat_refusals_are_typed() {
    let r = |status, name, message| chat_refusal(status, name, message);
    assert_eq!(r(403, "", ""), Some(ApiError::ChatAccess));
    assert_eq!(r(400, "ExpiredToken", ""), None);
    let disabled = "recipient has disabled incoming messages";
    assert_eq!(
        r(400, "InvalidRequest", disabled),
        Some(ApiError::ChatRefused)
    );
    assert_eq!(
        r(400, "NotFollowedBySender", ""),
        Some(ApiError::ChatRefused)
    );
    let blocked = "block between recipient and sender";
    assert_eq!(r(400, "InvalidRequest", blocked), Some(ApiError::Blocked));
    assert_eq!(
        r(400, "InvalidRequest", "Block exists"),
        Some(ApiError::Blocked)
    );
    assert_eq!(r(400, "InvalidRequest", "too long"), None);
}

/// Runs one call against the chat service.
macro_rules! chat {
    ($self:ident, |$api:ident| $call:expr) => {{
        let service: atrium_api::types::string::Did = CHAT_SERVICE
            .parse()
            .map_err(|e| ApiError::Other(format!("{e}")))?;
        match $self.agent().await? {
            AnyAgent::Password(agent, before) => {
                let $api = agent.api_with_proxy(service, CHAT_SERVICE_TYPE);
                let result = $call.await;
                $self.save_rotated_tokens(&agent, &before).await;
                result.map_err(chat_err)?
            }
            AnyAgent::OAuth(agent) => {
                let $api = agent.api_with_proxy(service, CHAT_SERVICE_TYPE);
                $call.await.map_err(chat_err)?
            }
        }
    }};
}

impl Client {
    /// Your chats, newest first. `requests` lists the ones waiting to be accepted
    /// instead.
    pub async fn convos(
        &self,
        requests: bool,
        cursor: Option<String>,
    ) -> Result<Page<Convo>, ApiError> {
        let status = if requests { "request" } else { "accepted" };
        let mut params = json!({"limit": PAGE_SIZE, "status": status});
        if let Some(c) = cursor {
            params["cursor"] = json!(c);
        }
        let out = chat!(self, |api| api
            .chat
            .bsky
            .convo
            .list_convos(conv(params.clone())?));
        Ok(Page {
            items: lenient_items(conv(&out.convos)?),
            cursor: out.cursor.clone(),
        })
    }

    /// A page of messages, newest first.
    pub async fn messages(
        &self,
        convo: &str,
        cursor: Option<String>,
    ) -> Result<Page<ChatMessage>, ApiError> {
        let mut params = json!({"convoId": convo, "limit": PAGE_SIZE});
        if let Some(c) = cursor {
            params["cursor"] = json!(c);
        }
        let out = chat!(self, |api| api
            .chat
            .bsky
            .convo
            .get_messages(conv(params.clone())?));
        Ok(Page {
            items: lenient_items(conv(&out.messages)?),
            cursor: out.cursor.clone(),
        })
    }

    /// The chat with exactly these people (you are added), started if there is none.
    pub async fn convo_for(&self, members: &[Did]) -> Result<Convo, ApiError> {
        let params = json!({ "members": members });
        let out = chat!(self, |api| api
            .chat
            .bsky
            .convo
            .get_convo_for_members(conv(params.clone())?));
        conv(&out.convo)
    }

    pub async fn convo(&self, id: &str) -> Result<Convo, ApiError> {
        let params = json!({ "convoId": id });
        let out = chat!(self, |api| api
            .chat
            .bsky
            .convo
            .get_convo(conv(params.clone())?));
        conv(&out.convo)
    }

    /// Sends a message, optionally sharing a post, and returns it as sent.
    pub async fn send_message(
        &self,
        convo: &str,
        text: &str,
        post: Option<StrongRef>,
    ) -> Result<ChatMessage, ApiError> {
        let mut message = json!({"text": text});
        let facets = self.facets(text).await;
        if !facets.is_empty() {
            message["facets"] = Value::Array(facets);
        }
        if let Some(p) = &post {
            message["embed"] = json!({"$type": "app.bsky.embed.record", "record": strong_ref(p)});
        }
        let input = json!({"convoId": convo, "message": message});
        let out = chat!(self, |api| api
            .chat
            .bsky
            .convo
            .send_message(conv(input.clone())?));
        conv(&out)
    }

    /// Accepts a chat request.
    pub async fn accept_convo(&self, id: &str) -> Result<(), ApiError> {
        let input = json!({ "convoId": id });
        chat!(self, |api| api
            .chat
            .bsky
            .convo
            .accept_convo(conv(input.clone())?));
        Ok(())
    }

    /// Leaves a chat, or declines a request. It disappears from your list.
    pub async fn leave_convo(&self, id: &str) -> Result<(), ApiError> {
        let input = json!({ "convoId": id });
        chat!(self, |api| api
            .chat
            .bsky
            .convo
            .leave_convo(conv(input.clone())?));
        Ok(())
    }

    pub async fn mute_convo(&self, id: &str, mute: bool) -> Result<(), ApiError> {
        let input = json!({ "convoId": id });
        if mute {
            chat!(self, |api| api
                .chat
                .bsky
                .convo
                .mute_convo(conv(input.clone())?));
        } else {
            chat!(self, |api| api
                .chat
                .bsky
                .convo
                .unmute_convo(conv(input.clone())?));
        }
        Ok(())
    }

    /// Marks everything in a chat as read.
    pub async fn mark_convo_read(&self, id: &str) -> Result<(), ApiError> {
        let input = json!({ "convoId": id });
        chat!(self, |api| api
            .chat
            .bsky
            .convo
            .update_read(conv(input.clone())?));
        Ok(())
    }

    /// Removes a message from your own view of the chat.
    pub async fn delete_message(&self, convo: &str, message: &str) -> Result<(), ApiError> {
        let input = json!({"convoId": convo, "messageId": message});
        chat!(self, |api| api
            .chat
            .bsky
            .convo
            .delete_message_for_self(conv(input.clone())?));
        Ok(())
    }

    /// Adds or removes your emoji reaction on a message.
    pub async fn react(
        &self,
        convo: &str,
        message: &str,
        value: &str,
        add: bool,
    ) -> Result<(), ApiError> {
        let input = json!({"convoId": convo, "messageId": message, "value": value});
        if add {
            chat!(self, |api| api
                .chat
                .bsky
                .convo
                .add_reaction(conv(input.clone())?));
        } else {
            chat!(self, |api| api
                .chat
                .bsky
                .convo
                .remove_reaction(conv(input.clone())?));
        }
        Ok(())
    }

    /// Who can start a chat with you.
    pub async fn allow_incoming(&self) -> Result<AllowIncoming, ApiError> {
        let (collection, rkey) = DECLARATION;
        Ok(match self.my_record(collection, rkey).await? {
            Some((v, _)) => AllowIncoming::parse(v["allowIncoming"].as_str().unwrap_or_default()),
            None => AllowIncoming::Following,
        })
    }

    pub async fn set_allow_incoming(&self, allow: AllowIncoming) -> Result<(), ApiError> {
        let (collection, rkey) = DECLARATION;
        let swap = self.my_record(collection, rkey).await?.map(|(_, cid)| cid);
        let record = json!({"$type": collection, "allowIncoming": allow.as_str()});
        self.put_record(collection, rkey, record, swap).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(did: &str, name: &str) -> Value {
        json!({"did": did, "handle": format!("{name}.test"), "displayName": name})
    }

    #[test]
    fn convos_parse_with_odd_last_messages() {
        let c: Convo = serde_json::from_value(json!({
            "id": "c1", "rev": "1", "muted": false, "unreadCount": 2, "status": "request",
            "members": [member("did:me", "Me"), member("did:a", "Ann")],
            "lastMessage": {"$type": "chat.bsky.convo.defs#somethingNew"}
        }))
        .unwrap();
        assert!(c.last_message.is_none());
        assert!(c.is_request());
        assert_eq!(c.title("did:me"), "Ann");
        assert!(!c.is_group("did:me"));
    }

    #[test]
    fn group_titles_shorten() {
        let c: Convo = serde_json::from_value(json!({
            "id": "c1",
            "members": [member("did:me", "Me"), member("did:a", "Ann"), member("did:b", "Bo"),
                        member("did:c", "Cy"), member("did:d", "Di")]
        }))
        .unwrap();
        assert!(c.is_group("did:me"));
        assert_eq!(c.title("did:me"), "Ann, Bo and 2 others");
    }

    #[test]
    fn messages_keep_text_facets_and_deletion() {
        let m: ChatMessage = serde_json::from_value(json!({
            "$type": "chat.bsky.convo.defs#messageView",
            "id": "m1", "rev": "r", "sentAt": "2026-10-05T10:00:00Z",
            "sender": {"did": "did:a"}, "text": "hi  there\nfriend",
            "facets": [{"index": {"byteStart": 0, "byteEnd": 2},
                        "features": [{"$type": "app.bsky.richtext.facet#tag", "tag": "hi"}]}],
            "reactions": [{"value": "❤️", "sender": {"did": "did:b"}, "createdAt": "x"}]
        }))
        .unwrap();
        assert_eq!(m.body.text, "hi  there\nfriend");
        assert_eq!(m.body.facets.len(), 1);
        assert_eq!(m.preview(), "hi there friend");
        assert_eq!(m.reactions[0].value, "❤️");

        let gone: ChatMessage = serde_json::from_value(json!({
            "$type": "chat.bsky.convo.defs#deletedMessageView",
            "id": "m2", "rev": "r", "sentAt": "x", "sender": {"did": "did:a"}
        }))
        .unwrap();
        assert!(gone.is_deleted());
        assert_eq!(gone.preview(), "Message deleted");
    }
}
