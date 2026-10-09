//! What the composer takes in ([`Msg`]) and what it asks of the app ([`Event`]).

use crate::api::{ApiError, LinkCard};
use crate::media::files::{Images, Video};
use crate::model::{AtUri, Author, Interaction};
use iced::widget::text_editor::Action;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum Msg {
    Edit(Action),
    Suggestions(String, Result<Vec<Author>, ApiError>),
    PickMention(String),
    PickTag(String),
    ToggleEmoji,
    Emoji(&'static str),
    ToggleWarnings,
    ToggleLabel(&'static str),
    /// Open or close who-can-reply settings.
    ToggleInteraction,
    Interaction(Interaction),
    /// "Use these settings for new posts".
    KeepInteraction(Interaction),
    CloseMenus,
    /// Ctrl+V: an image on the clipboard is attached, anything else is pasted as text.
    Paste,
    Pasted(Images),
    PasteText(Option<String>),
    /// Files dropped on the window.
    Dropped(Vec<PathBuf>),
    AddImages,
    ImagesPicked(Images),
    RemoveImage(usize),
    Alt(usize, String),
    AddVideo,
    VideoPicked(Video),
    RemoveVideo,
    VideoAlt(String),
    CardLoaded(String, Result<LinkCard, ApiError>),
    RemoveCard,
    Lang(String),
    /// Send long text as a thread of numbered posts, or as one post.
    ToggleThread,
    /// Move the image at this index to the next post of the thread.
    ImagePart(usize),
    ToggleSchedule,
    ScheduleText(String),
    /// Schedule for the time typed in the Schedule menu.
    ScheduleTyped,
    Schedule(chrono::DateTime<chrono::Utc>),
    Submit,
    /// Answers to "Post without alt text?" after Submit.
    PostAnyway,
    AddAlt,
    Posted(Result<AtUri, ApiError>),
    Cancel,
    /// Answers to "Save this post as a draft?" after Cancel.
    SaveDraft,
    Discard,
    KeepEditing,
}

/// Something the app has to do for the composer, in the order given.
#[derive(Debug)]
pub enum Event {
    /// Close the composer, dropping the post.
    Close,
    /// Close the composer, keeping the post in Drafts.
    SaveDraft,
    /// Remember these hashtags for suggestions.
    Tags(Vec<String>),
    /// The post was sent: close the composer and show it. Carries the post it
    /// replies to.
    Posted(Option<AtUri>),
    /// Fetch these pictures (avatars, link card thumbnails).
    Images(Vec<String>),
    /// Who-can-reply settings opened or closed; they offer your lists.
    NeedMyLists,
    /// Make these the who-can-reply settings for new posts.
    KeepInteraction(Interaction),
    /// Keep the post in Drafts, to be sent at this time.
    Schedule(chrono::DateTime<chrono::Utc>),
}
