//! Where the player's segments come from: reading the playlists, then a download
//! thread that fetches segments ahead of the decoder. A video on demand is fetched
//! from wherever a seek points; a live stream follows the playlist as it grows.

use super::net::{fetch_bytes, fetch_playlist, http};
use super::{MAX_HEIGHT, PlayError, Shared, hls, lock};
use crate::net::Transport;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::sync::mpsc::SyncSender;
use std::time::{Duration, Instant};

/// Segments a live stream starts this far behind its newest one, to have some in hand.
const LIVE_LEAD: u64 = 2;
/// A live stream with no new segment for this long has ended.
const LIVE_QUIET: Duration = Duration::from_secs(30);

/// What the download thread hands the decoder.
pub(super) enum Fetched {
    Segment {
        generation: u64,
        /// When the segment starts, in seconds from the start of playback.
        start: f64,
        video: Vec<u8>,
        audio: Option<Vec<u8>>,
    },
    End {
        generation: u64,
    },
    Failed(PlayError),
}

/// One media playlist, and the address it last came from: a live server may move it
/// (to add a session), and it is fetched again from there.
struct Track {
    url: String,
    media: hls::Media,
}

pub(super) struct Plan {
    video: Track,
    /// A separate sound playlist, when the sound isn't in the video's segments.
    audio: Option<Track>,
    /// The init segments of fragmented MP4, for the video and the sound.
    pub(super) inits: (Option<Vec<u8>>, Option<Vec<u8>>),
}

impl Plan {
    /// A live stream, whose playlist has no end yet.
    pub(super) fn live(&self) -> bool {
        !self.video.media.ended
    }

    /// When each segment starts, and how long the whole is, in seconds.
    pub(super) fn starts(&self) -> (Vec<f64>, f64) {
        let mut t = 0.0;
        let starts = self
            .video
            .media
            .segments
            .iter()
            .map(|s| {
                let start = t;
                t += s.duration;
                start
            })
            .collect();
        (starts, t)
    }
}

/// Reads the playlists at `url`: the quality to play, its sound, and their init
/// segments.
pub(super) async fn load(http: &dyn Transport, url: &str) -> Result<Plan, PlayError> {
    let (text, at) = fetch_playlist(http, url).await?;
    let (video, audio_url) = if hls::is_master(&text) {
        let master = hls::parse_master(&text, &at);
        let variant = hls::pick(&master.variants, MAX_HEIGHT).ok_or(PlayError::NoStreams)?;
        let audio = variant.audio_group.as_ref().and_then(|g| {
            master
                .audio
                .iter()
                .find(|(group, _)| group == g)
                .map(|(_, uri)| uri.clone())
        });
        (track(http, &variant.uri).await?, audio)
    } else {
        let media = hls::parse_media(&text, &at);
        (
            Track {
                url: at.to_string(),
                media,
            },
            None,
        )
    };
    if video.media.segments.is_empty() {
        return Err(if video.media.ended {
            PlayError::Empty
        } else {
            PlayError::NotStarted
        });
    }
    let audio = match audio_url {
        Some(a) => Some(track(http, &a).await?),
        None => None,
    };
    let video_init = init(http, &video).await?;
    let audio_init = match &audio {
        Some(a) => init(http, a).await?,
        None => None,
    };
    Ok(Plan {
        video,
        audio,
        inits: (video_init, audio_init),
    })
}

async fn track(http: &dyn Transport, url: &str) -> Result<Track, PlayError> {
    let (text, at) = fetch_playlist(http, url).await?;
    Ok(Track {
        media: hls::parse_media(&text, &at),
        url: at.to_string(),
    })
}

async fn init(http: &dyn Transport, track: &Track) -> Result<Option<Vec<u8>>, PlayError> {
    match &track.media.map {
        Some(map) => Ok(Some(fetch_bytes(http, &map.uri, map.range).await?)),
        None => Ok(None),
    }
}

/// One runtime for every video's downloads, so connections to the video server are
/// reused from one video to the next. The player's transport stays on this runtime:
/// a pooled connection belongs to the runtime that opened it.
pub(super) fn runtime() -> Option<&'static tokio::runtime::Runtime> {
    static RT: std::sync::OnceLock<Option<tokio::runtime::Runtime>> = std::sync::OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("video-net")
            .enable_all()
            .build()
            .ok()
    })
    .as_ref()
}

/// Downloads segments for the decoder until playback stops.
pub(super) fn download(shared: &Arc<Shared>, plan: Plan, tx: &SyncSender<Fetched>) {
    let Some(rt) = runtime() else {
        // A closed channel means playback already stopped.
        let _ = tx.send(Fetched::Failed(PlayError::NoThread("download")));
        return;
    };
    if plan.live() {
        live(shared, plan, tx, rt);
    } else {
        on_demand(shared, &plan, tx, rt);
    }
}

/// The segment and its sound, fetched together.
fn fetch_pair(
    rt: &tokio::runtime::Runtime,
    video: &hls::Segment,
    audio: Option<&hls::Segment>,
) -> Result<(Vec<u8>, Option<Vec<u8>>), PlayError> {
    let net = http();
    let v = rt.block_on(fetch_bytes(net, &video.uri, video.range));
    let a = audio
        .map(|seg| rt.block_on(fetch_bytes(net, &seg.uri, seg.range)))
        .transpose();
    Ok((v?, a?))
}

/// A video on demand: segments in order, starting again wherever a seek points.
fn on_demand(
    shared: &Arc<Shared>,
    plan: &Plan,
    tx: &SyncSender<Fetched>,
    rt: &tokio::runtime::Runtime,
) {
    let (starts, _) = plan.starts();
    let segments = &plan.video.media.segments;
    let mut generation = u64::MAX;
    let mut index = 0;
    while !shared.stop.load(Ordering::SeqCst) {
        let g = shared.generation.load(Ordering::SeqCst);
        if g != generation {
            generation = g;
            index = lock(&shared.state).from.0;
        }
        let Some(seg) = segments.get(index) else {
            if tx.send(Fetched::End { generation }).is_err() {
                return;
            }
            // Nothing more to fetch until a seek.
            while !shared.stale(generation) {
                std::thread::sleep(Duration::from_millis(50));
            }
            continue;
        };
        let audio = plan
            .audio
            .as_ref()
            .and_then(|a| a.media.segments.get(index));
        let fetched = fetch_pair(rt, seg, audio);
        if shared.stale(generation) {
            continue;
        }
        let item = match fetched {
            Ok((video, audio)) => Fetched::Segment {
                generation,
                start: starts.get(index).copied().unwrap_or(0.0),
                video,
                audio,
            },
            Err(e) => Fetched::Failed(e),
        };
        let failed = matches!(item, Fetched::Failed(_));
        if tx.send(item).is_err() || failed {
            return;
        }
        index += 1;
    }
}

/// A live stream: starts a little behind the newest segment and keeps fetching the
/// playlist for new ones, until it ends or goes quiet.
fn live(
    shared: &Arc<Shared>,
    mut plan: Plan,
    tx: &SyncSender<Fetched>,
    rt: &tokio::runtime::Runtime,
) {
    let generation = shared.generation.load(Ordering::SeqCst);
    let mut next = edge(&plan.video.media);
    let mut start = 0.0;
    let mut heard = Instant::now();
    while !shared.stop.load(Ordering::SeqCst) {
        // Fell out of the playlist's window, as after a long pause: catch up.
        if plan
            .video
            .media
            .segments
            .first()
            .is_some_and(|s| s.sequence > next)
        {
            next = edge(&plan.video.media);
        }
        let due: Vec<hls::Segment> = plan
            .video
            .media
            .segments
            .iter()
            .filter(|s| s.sequence >= next)
            .cloned()
            .collect();
        for seg in due {
            let audio = plan
                .audio
                .as_ref()
                .and_then(|a| a.media.segments.iter().find(|s| s.sequence == seg.sequence));
            // One gone from the server already is skipped rather than stopping.
            let Ok((video, audio)) = fetch_pair(rt, &seg, audio) else {
                next = seg.sequence + 1;
                continue;
            };
            let item = Fetched::Segment {
                generation,
                start,
                video,
                audio,
            };
            if shared.stop.load(Ordering::SeqCst) || tx.send(item).is_err() {
                return;
            }
            start += seg.duration;
            next = seg.sequence + 1;
            heard = Instant::now();
        }
        if plan.video.media.ended || heard.elapsed() > LIVE_QUIET {
            // A closed channel means playback already stopped.
            let _ = tx.send(Fetched::End { generation });
            return;
        }
        let wait = (plan.video.media.target / 2.0).clamp(0.5, 3.0);
        std::thread::sleep(Duration::from_secs_f64(wait));
        // A playlist that can't be fetched leaves the old one, until the stream is
        // quiet for long enough to count as ended.
        if let Ok(t) = rt.block_on(refetch(&plan.video)) {
            plan.video = t;
        }
        if let Some(a) = &plan.audio
            && let Ok(t) = rt.block_on(refetch(a))
        {
            plan.audio = Some(t);
        }
    }
}

async fn refetch(t: &Track) -> Result<Track, PlayError> {
    track(http(), &t.url).await
}

/// The segment to start a live stream from: a couple behind the newest.
fn edge(media: &hls::Media) -> u64 {
    let first = media.segments.first().map_or(0, |s| s.sequence);
    let last = media.segments.last().map_or(0, |s| s.sequence);
    last.saturating_sub(LIVE_LEAD).max(first)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::fake::Fake;

    fn media(text: &str) -> hls::Media {
        hls::parse_media(
            text,
            &url::Url::parse("https://live.example/v.m3u8").unwrap(),
        )
    }

    #[test]
    fn live_streams_start_near_the_newest_segment() {
        let m = media(
            "#EXTM3U\n#EXT-X-MEDIA-SEQUENCE:10\n#EXTINF:1,\na\n#EXTINF:1,\nb\n#EXTINF:1,\nc\n#EXTINF:1,\nd\n",
        );
        assert_eq!(edge(&m), 11);
        let short = media("#EXTM3U\n#EXT-X-MEDIA-SEQUENCE:5\n#EXTINF:1,\na\n");
        assert_eq!(edge(&short), 5);
    }

    #[tokio::test]
    async fn plans_follow_the_master_to_the_default_sound_and_fetch_the_inits() {
        let master = "#EXTM3U\n\
            #EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"a\",NAME=\"opus\",URI=\"opus.m3u8\"\n\
            #EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"a\",NAME=\"aac\",DEFAULT=YES,URI=\"aac.m3u8\"\n\
            #EXT-X-STREAM-INF:AUDIO=\"a\",BANDWIDTH=1,RESOLUTION=1280x720\nvideo.m3u8\n";
        let video = "#EXTM3U\n#EXT-X-MAP:URI=\"vinit.mp4\"\n#EXTINF:1,\nv1.m4s\n";
        let audio = "#EXTM3U\n#EXT-X-MAP:URI=\"ainit.mp4\"\n#EXTINF:1,\na1.m4s\n";
        let net = Fake::default()
            .reply(200, master)
            .reply(200, video)
            .reply(200, audio)
            .reply(200, "VINIT")
            .reply(200, "AINIT");
        let plan = load(&net, "https://live.example/master.m3u8")
            .await
            .unwrap();
        assert!(plan.live());
        assert_eq!(
            plan.inits,
            (Some(b"VINIT".to_vec()), Some(b"AINIT".to_vec()))
        );
        let asked: Vec<String> = net
            .seen()
            .iter()
            .map(|r| r.url.path().to_string())
            .collect();
        assert_eq!(
            asked,
            [
                "/master.m3u8",
                "/video.m3u8",
                "/aac.m3u8",
                "/vinit.mp4",
                "/ainit.mp4"
            ]
        );
    }

    #[tokio::test]
    async fn a_moved_playlist_is_fetched_again_from_where_it_moved() {
        let net = Fake::default().reply_moved(
            "https://live.example/v.m3u8?sid=abc",
            200,
            "#EXTM3U\n#EXTINF:1,\nseg.m4s\n#EXT-X-ENDLIST\n",
        );
        let t = track(&net, "https://live.example/v.m3u8").await.unwrap();
        assert_eq!(t.url, "https://live.example/v.m3u8?sid=abc");
        assert_eq!(t.media.segments[0].uri, "https://live.example/seg.m4s");
    }

    #[tokio::test]
    async fn an_empty_live_playlist_has_not_started() {
        let net = Fake::default().reply(200, "#EXTM3U\n#EXT-X-TARGETDURATION:1\n");
        let Err(e) = load(&net, "https://live.example/v.m3u8").await else {
            panic!("loaded");
        };
        assert_eq!(e, PlayError::NotStarted);
    }
}
