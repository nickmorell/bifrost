//! FreeMix tracks: music published on the AT Protocol.

use super::{Integration, Shape, Shown, card, host, parts};
use url::Url;

pub struct FreeMix;

impl Integration for FreeMix {
    fn id(&self) -> &'static str {
        "freemix"
    }

    fn name(&self) -> &'static str {
        "FreeMix"
    }

    fn description(&self) -> &'static str {
        "Show FreeMix tracks as cards."
    }

    fn embed(&self, url: &Url, title: Option<&str>) -> Option<Shown> {
        let host = host(url)?;
        if host != "app.freemix.fm" && host != "staging.freemix.fm" {
            return None;
        }
        match parts(url).as_slice() {
            ["track", _, rkey] if is_tid(rkey) => {
                Some(card("FreeMix", "Track", "Listen", Shape::Cover).open(url, title))
            }
            _ => None,
        }
    }
}

/// A record key made from a timestamp: 13 characters of base32 (`2`-`7`, `a`-`z`).
fn is_tid(key: &str) -> bool {
    key.len() == 13
        && key
            .bytes()
            .all(|b| b.is_ascii_lowercase() || (b'2'..=b'7').contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_tracks() {
        let e = |u: &str| FreeMix.embed(&Url::parse(u).unwrap(), None).is_some();
        assert!(e("https://app.freemix.fm/track/alice.test/3mabcdefghijk"));
        assert!(e(
            "https://staging.freemix.fm/track/did:plc:x/3mabcdefghijk"
        ));
        assert!(!e("https://app.freemix.fm/track/alice.test/notatid"));
        assert!(!e("https://freemix.fm/track/alice.test/3mabcdefghijk"));
    }
}
