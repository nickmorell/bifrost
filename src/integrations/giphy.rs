//! GIPHY: GIF pages and files play as the animation itself.

use super::{Integration, Shown, host, parts};
use url::Url;

pub struct Giphy;

impl Integration for Giphy {
    fn id(&self) -> &'static str {
        "giphy"
    }

    fn name(&self) -> &'static str {
        "GIPHY"
    }

    fn description(&self) -> &'static str {
        "Play GIPHY links as GIFs. Loads them from media.giphy.com."
    }

    fn embed(&self, url: &Url, _title: Option<&str>) -> Option<Shown> {
        let id = gif_id(url)?;
        Some(Shown::Gif(format!(
            "https://media.giphy.com/media/{id}/giphy.gif"
        )))
    }
}

/// The GIF's id from a GIPHY page (`/gifs/funny-cat-<id>`) or file link.
fn gif_id(url: &Url) -> Option<String> {
    let host = host(url)?;
    let segs = parts(url);
    let id = match (host.as_str(), segs.as_slice()) {
        ("giphy.com", ["gifs", slug, ..]) => slug.rsplit('-').next()?,
        ("i.giphy.com", ["media", file] | [file]) => file.split('.').next()?,
        (h, ["media", id, ..]) if is_media_host(h) => id,
        _ => return None,
    };
    (!id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric())).then(|| id.to_string())
}

/// `media.giphy.com` and its numbered mirrors, `media0` to `media4`.
fn is_media_host(host: &str) -> bool {
    host.strip_suffix(".giphy.com")
        .and_then(|h| h.strip_prefix("media"))
        .is_some_and(|n| matches!(n, "" | "0" | "1" | "2" | "3" | "4"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(u: &str) -> Option<String> {
        gif_id(&Url::parse(u).unwrap())
    }

    #[test]
    fn finds_the_gif_in_pages_and_files() {
        let x = Some("3o7aD2saalBwwftBIY".to_string());
        assert_eq!(id("https://giphy.com/gifs/cat-funny-3o7aD2saalBwwftBIY"), x);
        assert_eq!(id("https://giphy.com/gifs/3o7aD2saalBwwftBIY"), x);
        assert_eq!(
            id("https://media.giphy.com/media/3o7aD2saalBwwftBIY/giphy.gif"),
            x
        );
        assert_eq!(
            id("https://media3.giphy.com/media/3o7aD2saalBwwftBIY/200.gif"),
            x
        );
        assert_eq!(id("https://i.giphy.com/media/3o7aD2saalBwwftBIY.gif"), x);
        assert_eq!(id("https://i.giphy.com/3o7aD2saalBwwftBIY.webp"), x);
        assert_eq!(
            id("https://media9.giphy.com/media/3o7aD2saalBwwftBIY/x.gif"),
            None
        );
        assert_eq!(id("https://giphy.com/explore/cats"), None);
    }

    #[test]
    fn plays_the_gif_file() {
        let url = Url::parse("https://giphy.com/gifs/cat-3o7aD2saalBwwftBIY").unwrap();
        assert_eq!(
            Giphy.embed(&url, None),
            Some(Shown::Gif(
                "https://media.giphy.com/media/3o7aD2saalBwwftBIY/giphy.gif".into()
            ))
        );
    }
}
