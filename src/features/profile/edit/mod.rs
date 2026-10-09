//! The "Edit profile" dialog: display name, bio, profile picture and banner.

mod view;

use crate::api::richtext::grapheme_len;
use crate::api::service::ProfileApi;
use crate::api::{ApiError, ImageChange, ProfileEdit};
use crate::media::MediaError;
use crate::media::files::Attachments;
use crate::media::image::Prepared;
use crate::model::Profile;
use iced::Task;
use iced::widget::image;
use iced::widget::text_editor::{Action, Content};

/// Bluesky's limits, in graphemes.
const MAX_NAME: usize = 64;
const MAX_BIO: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pic {
    Avatar,
    Banner,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Name(String),
    Bio(Action),
    Pick(Pic),
    Picked(Pic, Option<Result<Prepared, MediaError>>),
    Remove(Pic),
    Save,
    Saved(Result<(), ApiError>),
    Cancel,
}

/// What a message did to the dialog as a whole.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Outcome {
    None,
    Close,
    /// Saved: reload your profile.
    Saved,
}

/// A picture as it will be saved.
enum Picture {
    /// The one on the profile now, if any.
    Current(Option<String>),
    New(Prepared, image::Handle),
    Removed,
}

impl Picture {
    fn change(&self) -> ImageChange {
        match self {
            Picture::Current(_) => ImageChange::Keep,
            Picture::New(p, _) => ImageChange::Set(p.bytes.clone()),
            Picture::Removed => ImageChange::Remove,
        }
    }

    fn present(&self) -> bool {
        matches!(self, Picture::New(..) | Picture::Current(Some(_)))
    }
}

pub struct Editor {
    name: String,
    bio: Content,
    avatar: Picture,
    banner: Picture,
    saving: bool,
    error: Option<String>,
}

impl Editor {
    pub fn new(p: &Profile) -> Self {
        Self {
            name: p.display_name.clone().unwrap_or_default(),
            bio: Content::with_text(p.description.as_deref().unwrap_or_default()),
            avatar: Picture::Current(p.avatar.clone()),
            banner: Picture::Current(p.banner.clone()),
            saving: false,
            error: None,
        }
    }

    fn bio(&self) -> String {
        let t = self.bio.text();
        t.strip_suffix('\n').unwrap_or(&t).to_string()
    }

    fn valid(&self) -> bool {
        grapheme_len(&self.name) <= MAX_NAME && grapheme_len(&self.bio()) <= MAX_BIO
    }

    pub(super) fn update(
        &mut self,
        msg: Msg,
        api: &impl ProfileApi,
        files: &impl Attachments,
    ) -> (Task<Msg>, Outcome) {
        match msg {
            Msg::Name(v) => self.name = v.replace('\n', " "),
            Msg::Bio(action) => self.bio.perform(action),
            Msg::Pick(pic) => {
                let (title, aspect, width) = match pic {
                    Pic::Avatar => ("Choose a profile picture", 1.0, 1000),
                    Pic::Banner => ("Choose a banner", 3.0, 3000),
                };
                let files = files.clone();
                let task = Task::perform(
                    async move { files.pick_cropped(title, aspect, width).await },
                    move |r| Msg::Picked(pic, r),
                );
                return (task, Outcome::None);
            }
            Msg::Picked(_, None) => {}
            Msg::Picked(_, Some(Err(e))) => {
                self.error = Some(format!("Couldn't use that image: {e}"));
            }
            Msg::Picked(pic, Some(Ok(img))) => {
                self.error = None;
                let preview = image::Handle::from_bytes(img.bytes.clone());
                *self.slot(pic) = Picture::New(img, preview);
            }
            Msg::Remove(pic) => *self.slot(pic) = Picture::Removed,
            Msg::Save => {
                if self.saving || !self.valid() {
                    return (Task::none(), Outcome::None);
                }
                self.saving = true;
                self.error = None;
                let edit = ProfileEdit {
                    display_name: self.name.clone(),
                    description: self.bio(),
                    avatar: self.avatar.change(),
                    banner: self.banner.change(),
                };
                let api = api.clone();
                let task = Task::perform(async move { api.save_profile(edit).await }, Msg::Saved);
                return (task, Outcome::None);
            }
            Msg::Saved(Ok(())) => return (Task::none(), Outcome::Saved),
            Msg::Saved(Err(e)) => {
                self.saving = false;
                self.error = Some(format!("Couldn't save your profile. {}", e.friendly()));
            }
            Msg::Cancel if !self.saving => return (Task::none(), Outcome::Close),
            Msg::Cancel => {}
        }
        (Task::none(), Outcome::None)
    }

    fn slot(&mut self, pic: Pic) -> &mut Picture {
        match pic {
            Pic::Avatar => &mut self.avatar,
            Pic::Banner => &mut self.banner,
        }
    }

    fn picture(&self, pic: Pic) -> &Picture {
        match pic {
            Pic::Avatar => &self.avatar,
            Pic::Banner => &self.banner,
        }
    }
}
