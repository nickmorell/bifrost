//! Reading HLS playlists: the master playlist that lists the qualities a video comes
//! in, and the media playlist that lists one quality's segments.

use url::Url;

#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    pub uri: String,
    pub bandwidth: u64,
    pub height: u32,
    /// The audio rendition group, when the sound is in a playlist of its own.
    pub audio_group: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub uri: String,
    /// Seconds.
    pub duration: f64,
    /// Its number in the stream; a live playlist drops old ones as new ones come.
    pub sequence: u64,
    /// The bytes of `uri` it takes up, as (offset, length), when it is part of a file.
    pub range: Option<(u64, u64)>,
}

/// An init segment: where fragmented MP4 says what its tracks are.
#[derive(Debug, Clone, PartialEq)]
pub struct Map {
    pub uri: String,
    pub range: Option<(u64, u64)>,
}

#[derive(Debug, Default, PartialEq)]
pub struct Master {
    pub variants: Vec<Variant>,
    /// Audio renditions: (group id, playlist URI).
    pub audio: Vec<(String, String)>,
}

#[derive(Debug, Default, PartialEq)]
pub struct Media {
    pub segments: Vec<Segment>,
    /// The init segment of fragmented MP4 segments; transport streams have none.
    pub map: Option<Map>,
    /// No more segments will come (`#EXT-X-ENDLIST`). A live stream's playlist has
    /// no end yet and is fetched again for new segments.
    pub ended: bool,
    /// The longest a segment may be, in seconds.
    pub target: f64,
}

pub fn is_master(text: &str) -> bool {
    text.contains("#EXT-X-STREAM-INF")
}

fn resolve(base: &Url, uri: &str) -> String {
    base.join(uri.trim())
        .map_or_else(|_| uri.trim().to_string(), String::from)
}

/// `KEY=value,KEY="quoted, value"` pairs.
fn attributes(list: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut key = String::new();
    let mut value = String::new();
    let mut in_value = false;
    let mut quoted = false;
    for c in list.chars() {
        match c {
            '"' if in_value => quoted = !quoted,
            '=' if !in_value => in_value = true,
            ',' if !quoted => {
                out.push((key.trim().to_string(), std::mem::take(&mut value)));
                key.clear();
                in_value = false;
            }
            c if in_value => value.push(c),
            c => key.push(c),
        }
    }
    if !key.trim().is_empty() {
        out.push((key.trim().to_string(), value));
    }
    out
}

fn attr<'a>(attrs: &'a [(String, String)], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.as_str())
}

pub fn parse_master(text: &str, base: &Url) -> Master {
    let mut master = Master::default();
    let mut pending: Option<Vec<(String, String)>> = None;
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if let Some(rest) = line.strip_prefix("#EXT-X-STREAM-INF:") {
            pending = Some(attributes(rest));
        } else if let Some(rest) = line.strip_prefix("#EXT-X-MEDIA:") {
            let a = attributes(rest);
            if attr(&a, "TYPE") == Some("AUDIO")
                && let (Some(group), Some(uri)) = (attr(&a, "GROUP-ID"), attr(&a, "URI"))
            {
                let rendition = (group.to_string(), resolve(base, uri));
                // The group's default goes first, so it is the one picked.
                if attr(&a, "DEFAULT") == Some("YES") {
                    master.audio.insert(0, rendition);
                } else {
                    master.audio.push(rendition);
                }
            }
        } else if !line.starts_with('#')
            && let Some(a) = pending.take()
        {
            let height = attr(&a, "RESOLUTION")
                .and_then(|r| r.split('x').nth(1))
                .and_then(|h| h.parse().ok())
                .unwrap_or(0);
            master.variants.push(Variant {
                uri: resolve(base, line),
                bandwidth: attr(&a, "BANDWIDTH")
                    .and_then(|b| b.parse().ok())
                    .unwrap_or(0),
                height,
                audio_group: attr(&a, "AUDIO").map(str::to_string),
            });
        }
    }
    master
}

pub fn parse_media(text: &str, base: &Url) -> Media {
    let mut media = Media::default();
    let mut duration = None;
    let mut range = None;
    let mut sequence = 0;
    // Where the last byte range ended, for one that leaves out its offset.
    let mut next_offset = 0;
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if let Some(rest) = line.strip_prefix("#EXTINF:") {
            duration = rest.split(',').next().and_then(|d| d.trim().parse().ok());
        } else if let Some(rest) = line.strip_prefix("#EXT-X-BYTERANGE:") {
            range = byte_range(rest, next_offset);
            next_offset = range.map_or(0, |(o, n)| o.saturating_add(n));
        } else if let Some(rest) = line.strip_prefix("#EXT-X-MEDIA-SEQUENCE:") {
            sequence = rest.parse().unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("#EXT-X-TARGETDURATION:") {
            media.target = rest.parse().unwrap_or(0.0);
        } else if let Some(rest) = line.strip_prefix("#EXT-X-MAP:") {
            let a = attributes(rest);
            media.map = attr(&a, "URI").map(|uri| Map {
                uri: resolve(base, uri),
                range: attr(&a, "BYTERANGE").and_then(|r| byte_range(r, 0)),
            });
        } else if line == "#EXT-X-ENDLIST" {
            media.ended = true;
        } else if !line.starts_with('#') {
            media.segments.push(Segment {
                uri: resolve(base, line),
                duration: duration.take().unwrap_or(0.0),
                sequence,
                range: range.take(),
            });
            sequence = sequence.saturating_add(1);
        }
    }
    media
}

/// `length[@offset]`, as (offset, length).
fn byte_range(text: &str, default_offset: u64) -> Option<(u64, u64)> {
    let (len, offset) = match text.trim().split_once('@') {
        Some((l, o)) => (l, o.parse().ok()?),
        None => (text.trim(), default_offset),
    };
    let len: u64 = len.parse().ok()?;
    // A range past the end of any file is a broken playlist.
    offset.checked_add(len).map(|_| (offset, len))
}

/// The quality to play: the sharpest up to `max_height`, or the smallest if all are
/// taller.
pub fn pick(variants: &[Variant], max_height: u32) -> Option<&Variant> {
    variants
        .iter()
        .filter(|v| v.height <= max_height)
        .max_by_key(|v| (v.height, v.bandwidth))
        .or_else(|| variants.iter().min_by_key(|v| (v.height, v.bandwidth)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Url {
        Url::parse("https://video.bsky.app/watch/did%3Aplc%3Aa/bafy/playlist.m3u8").unwrap()
    }

    #[test]
    fn reads_master_playlists() {
        let text = "#EXTM3U\n#EXT-X-VERSION:3\n\
            #EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"aud\",NAME=\"en\",URI=\"audio/a.m3u8\"\n\
            #EXT-X-STREAM-INF:BANDWIDTH=1200000,RESOLUTION=640x360,CODECS=\"avc1.64001e,mp4a.40.2\",AUDIO=\"aud\"\n\
            360p/video.m3u8\n\
            #EXT-X-STREAM-INF:BANDWIDTH=2800000,RESOLUTION=1280x720\n\
            720p/video.m3u8\n";
        assert!(is_master(text));
        let m = parse_master(text, &base());
        assert_eq!(m.variants.len(), 2);
        assert_eq!(
            m.variants[0].uri,
            "https://video.bsky.app/watch/did%3Aplc%3Aa/bafy/360p/video.m3u8"
        );
        assert_eq!(m.variants[0].height, 360);
        assert_eq!(m.variants[0].audio_group.as_deref(), Some("aud"));
        assert_eq!(m.audio[0].0, "aud");
        assert_eq!(pick(&m.variants, 720).unwrap().height, 720);
        assert_eq!(pick(&m.variants, 480).unwrap().height, 360);
        assert_eq!(pick(&m.variants, 100).unwrap().height, 360);
    }

    #[test]
    fn reads_media_playlists() {
        let text = "#EXTM3U\n#EXT-X-TARGETDURATION:6\n#EXTINF:6.000,\nvideo0.ts\n\
            #EXTINF:2.5,\nvideo1.ts?session=x\n#EXT-X-ENDLIST\n";
        let m = parse_media(text, &base());
        assert_eq!(m.map, None);
        assert!(m.ended);
        assert!((m.target - 6.0).abs() < f64::EPSILON);
        assert_eq!(m.segments.len(), 2);
        assert!((m.segments[1].duration - 2.5).abs() < f64::EPSILON);
        assert!(m.segments[1].uri.ends_with("/bafy/video1.ts?session=x"));
    }

    #[test]
    fn reads_live_fragmented_playlists() {
        let text = "#EXTM3U\n#EXT-X-VERSION:7\n#EXT-X-TARGETDURATION:2\n\
            #EXT-X-MEDIA-SEQUENCE:41\n#EXT-X-MAP:URI=\"init.mp4?sid=x\"\n\
            #EXTINF:1.0,\n41.m4s\n#EXTINF:0.98,\n42.m4s\n";
        let m = parse_media(text, &base());
        assert!(!m.ended);
        assert!(
            m.map
                .as_ref()
                .unwrap()
                .uri
                .ends_with("/bafy/init.mp4?sid=x")
        );
        let seq: Vec<u64> = m.segments.iter().map(|s| s.sequence).collect();
        assert_eq!(seq, [41, 42]);
    }

    #[test]
    fn ranges_past_any_file_are_refused() {
        assert_eq!(byte_range("10@20", 0), Some((20, 10)));
        assert_eq!(byte_range(&format!("2@{}", u64::MAX), 0), None);
        assert_eq!(byte_range("5", u64::MAX), None);
    }

    #[test]
    fn reads_byte_ranges() {
        let text = "#EXTM3U\n#EXT-X-MAP:URI=\"v.mp4\",BYTERANGE=\"800@0\"\n\
            #EXTINF:2,\n#EXT-X-BYTERANGE:1000@800\nv.mp4\n\
            #EXTINF:2,\n#EXT-X-BYTERANGE:500\nv.mp4\n#EXT-X-ENDLIST\n";
        let m = parse_media(text, &base());
        assert_eq!(m.map.unwrap().range, Some((0, 800)));
        assert_eq!(m.segments[0].range, Some((800, 1000)));
        assert_eq!(m.segments[1].range, Some((1800, 500)));
    }

    #[test]
    fn the_default_sound_is_picked_first() {
        let text = "#EXTM3U\n\
            #EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"audio\",NAME=\"opus\",DEFAULT=NO,URI=\"o.m3u8\"\n\
            #EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"audio\",NAME=\"mp4a.40.2\",DEFAULT=YES,URI=\"a.m3u8\"\n\
            #EXT-X-STREAM-INF:AUDIO=\"audio\",BANDWIDTH=1,RESOLUTION=1920x1080\nv.m3u8\n";
        let m = parse_master(text, &base());
        assert!(m.audio[0].1.ends_with("/a.m3u8"));
    }
}
