//! Images in posts: which GIF links play, and the adapters that hand
//! a post's images to the `media` component.

use super::util::picture_of;
use super::{App, Message, viewer};
use crate::components::{Picture, media};
use crate::model::ImageView;
use iced::Element;

/// One image as the viewer shows it.
#[derive(Debug, Clone)]
pub struct ViewerImage {
    pub full: String,
    pub thumb: String,
    pub alt: String,
}

impl From<&ImageView> for ViewerImage {
    fn from(i: &ImageView) -> Self {
        Self {
            full: i.fullsize.clone(),
            thumb: i.thumb.clone(),
            alt: i.alt.clone(),
        }
    }
}

/// The open image viewer.
#[derive(Debug, Clone)]
pub struct Viewer {
    pub images: Vec<ViewerImage>,
    pub index: usize,
}

impl Viewer {
    pub fn current(&self) -> &ViewerImage {
        &self.images[self.index.min(self.images.len() - 1)]
    }
}

pub use crate::components::Frames;

/// A file name for saving `url`: Bluesky's CDN marks the format with "@jpeg" etc.
pub fn file_name(url: &str) -> String {
    let last = url.rsplit('/').next().unwrap_or("image");
    let last = last.split('?').next().unwrap_or(last);
    let (stem, ext) = match last.rsplit_once('@') {
        Some((stem, fmt)) => (stem, if fmt == "jpeg" { "jpg" } else { fmt }),
        None => match last.rsplit_once('.') {
            Some((stem, ext)) => (stem, ext),
            None => (last, "jpg"),
        },
    };
    let stem: String = stem
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(40)
        .collect();
    let stem = if stem.is_empty() {
        "image".into()
    } else {
        stem
    };
    format!("{stem}.{ext}")
}

/// Whether a link points straight at a GIF on Bluesky's own servers, which plays
/// without asking. A GIF anywhere else plays only through an integration that is on
/// (GIPHY, Tenor, KLIPY), and otherwise shows as a normal link card, so a post can't
/// make every viewer download a file from a server of its choosing.
pub fn is_gif_url(url: &str) -> bool {
    url::Url::parse(url).is_ok_and(|u| {
        u.scheme() == "https"
            && u.path().to_ascii_lowercase().ends_with(".gif")
            && u.host_str() == Some("cdn.bsky.app")
    })
}

/// The width/height ratio a Tenor link records in its `ww`/`hh` parameters.
fn gif_ratio(url: &str) -> Option<f32> {
    let u = url::Url::parse(url).ok()?;
    let get = |k: &str| {
        u.query_pairs()
            .find(|(key, _)| key == k)
            .and_then(|(_, v)| v.parse::<f32>().ok())
    };
    Some(get("ww")? / get("hh")?.max(1.0))
}

/// A GIF link shown as the animation itself.
pub fn gif_view<'a>(app: &'a App, url: &str, alt: &'a str) -> Element<'a, Message> {
    let open = Message::Viewer(viewer::Msg::Open(
        vec![ViewerImage {
            full: url.to_string(),
            thumb: url.to_string(),
            alt: alt.to_string(),
        }],
        0,
    ));
    media::gif(picture_of(app, url), gif_ratio(url), alt, open)
}

/// A post's images, each opening in the viewer.
pub fn images_view<'a>(app: &'a App, images: &'a [ImageView]) -> Element<'a, Message> {
    let all: Vec<ViewerImage> = images.iter().map(ViewerImage::from).collect();
    let tiles = images
        .iter()
        .map(|i| media::Tile {
            picture: picture_of(app, &i.thumb),
            alt: &i.alt,
        })
        .collect();
    let ratio = images
        .first()
        .and_then(|i| i.aspect_ratio.as_ref())
        .map(|r| r.width as f32 / r.height.max(1) as f32);
    media::images(tiles, ratio, |i| {
        Message::Viewer(viewer::Msg::Open(all.clone(), i))
    })
}

/// A quoted post's images, as small squares that open in the viewer.
pub fn thumbs_view<'a>(app: &'a App, images: &'a [ImageView]) -> Element<'a, Message> {
    let all: Vec<ViewerImage> = images.iter().map(ViewerImage::from).collect();
    let pictures = images.iter().map(|i| picture_of(app, &i.thumb)).collect();
    media::thumbs(pictures, |i| {
        Message::Viewer(viewer::Msg::Open(all.clone(), i))
    })
}

/// The full-screen viewer, drawn over everything else.
pub fn viewer_view<'a>(app: &'a App, v: &'a Viewer) -> Element<'a, Message> {
    let cur = v.current();
    let shown = media::Shown {
        picture: match app.picture_at(&cur.full) {
            Picture::Waiting { .. } => app.picture_at(&cur.thumb),
            loaded => loaded,
        },
        index: v.index,
        count: v.images.len(),
        alt: &cur.alt,
    };
    media::viewer(
        shown,
        Message::Viewer(viewer::Msg::Save),
        Message::Viewer(viewer::Msg::Close),
        |a| Message::Viewer(viewer::Msg::Step(a)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gif_links() {
        assert!(is_gif_url("https://cdn.bsky.app/img/x/cat.gif"));
        assert!(!is_gif_url("https://example.com/page"));
        // Tenor, KLIPY and GIPHY play only through their integrations.
        assert!(!is_gif_url(
            "https://media.tenor.com/abc/cat.gif?hh=280&ww=498"
        ));
        assert!(!is_gif_url("https://static.klipy.com/ii/abc/cat.gif"));
        assert!(!is_gif_url("https://media.giphy.com/media/abc/giphy.gif"));
        assert!(!is_gif_url("https://example.com/cat.gif"));
        assert!(!is_gif_url("http://cdn.bsky.app/img/x/cat.gif"));
        assert_eq!(
            file_name("https://cdn.bsky.app/img/feed_fullsize/plain/did:plc:x/bafkreiabc@jpeg"),
            "bafkreiabc.jpg"
        );
        assert_eq!(
            file_name("https://media.tenor.com/a/cat.gif?hh=1"),
            "cat.gif"
        );
        let r = gif_ratio("https://media.tenor.com/abc/cat.gif?hh=200&ww=400").unwrap();
        assert!((r - 2.0).abs() < 1e-6);
    }
}
