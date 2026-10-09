//! Tenor and KLIPY: the GIF pickers Bluesky posts from. Their GIF links play as the
//! animation itself, loaded from the service's own servers.

use super::{Integration, Shown};
use url::Url;

pub struct Tenor;

impl Integration for Tenor {
    fn id(&self) -> &'static str {
        "tenor"
    }

    fn name(&self) -> &'static str {
        "Tenor"
    }

    fn description(&self) -> &'static str {
        "Play Tenor GIFs, as posted from Bluesky's GIF picker. Loads them from tenor.com."
    }

    fn embed(&self, url: &Url, _title: Option<&str>) -> Option<Shown> {
        gif_on(url, "tenor.com")
    }
}

pub struct Klipy;

impl Integration for Klipy {
    fn id(&self) -> &'static str {
        "klipy"
    }

    fn name(&self) -> &'static str {
        "KLIPY"
    }

    fn description(&self) -> &'static str {
        "Play KLIPY GIFs, as posted from Bluesky's GIF picker. Loads them from klipy.com."
    }

    fn embed(&self, url: &Url, _title: Option<&str>) -> Option<Shown> {
        gif_on(url, "klipy.com")
    }
}

/// The link itself, when it is an https link straight to a GIF on `domain` or one of
/// its subdomains.
fn gif_on(url: &Url, domain: &str) -> Option<Shown> {
    let host = url.host_str()?.to_ascii_lowercase();
    let ours = host == domain
        || host
            .strip_suffix(domain)
            .is_some_and(|sub| sub.ends_with('.'));
    let gif = url.path().to_ascii_lowercase().ends_with(".gif");
    (url.scheme() == "https" && ours && gif).then(|| Shown::Gif(url.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tenor(u: &str) -> Option<Shown> {
        Tenor.embed(&Url::parse(u).unwrap(), None)
    }

    #[test]
    fn plays_gif_files_from_the_service_only() {
        let gif = "https://media.tenor.com/abc/cat.gif?hh=280&ww=498";
        assert_eq!(tenor(gif), Some(Shown::Gif(gif.into())));
        assert!(tenor("https://media1.tenor.com/m/abc/cat.gif").is_some());
        assert!(tenor("https://tenor.com/view/cat-123").is_none());
        // Lookalike hosts and plain http stay link cards.
        assert!(tenor("https://eviltenor.com/cat.gif").is_none());
        assert!(tenor("https://tenor.com.evil.example/cat.gif").is_none());
        assert!(tenor("http://media.tenor.com/abc/cat.gif").is_none());
        let klipy = Url::parse("https://static.klipy.com/ii/abc/cat.gif").unwrap();
        assert!(Klipy.embed(&klipy, None).is_some());
        assert!(Tenor.embed(&klipy, None).is_none());
    }
}
