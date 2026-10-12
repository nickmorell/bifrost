//! The player without a network: seeking, the clock, moving through frames, and the
//! decoder turning the fMP4 test clips into pictures and sound.

use super::*;

const VIDEO_INIT: &[u8] = include_bytes!("mp4/testdata/v_init.mp4");
const VIDEO: &[u8] = include_bytes!("mp4/testdata/v_0.m4s");
const AUDIO_INIT: &[u8] = include_bytes!("mp4/testdata/a_init.mp4");
const AUDIO: &[u8] = include_bytes!("mp4/testdata/a_0.m4s");

/// A player with no download or decode thread, and no sound device.
fn player() -> Player {
    Player {
        shared: Arc::new(Shared::new(Arc::new(audio::Buffer::silent()))),
    }
}

/// A 12 s video in three segments, playing.
fn loaded(p: &Player) {
    let mut st = lock(&p.shared.state);
    st.starts = vec![0.0, 4.0, 8.0];
    st.duration = 12.0;
    st.status = Status::Playing;
}

fn frame(time: f64) -> Frame {
    Frame {
        time,
        handle: image::Handle::from_rgba(1, 1, vec![0; 4]),
    }
}

#[test]
fn ticks_become_seconds_across_the_wrap() {
    assert!((time_of((10.0, 90_000), 180_000) - 11.0).abs() < 1e-9);
    // Ticks wrap at 2^33: just after the wrap is just after the first tick.
    let first = (1 << 33) - 45_000;
    assert!((time_of((0.0, first), 45_000) - 1.0).abs() < 1e-9);
    // A picture shown before the segment's first tick (B-frames).
    assert!((time_of((2.0, 9000), 0) - 1.9).abs() < 1e-9);
}

#[test]
fn seeking_picks_the_segment_and_starts_over() {
    let p = player();
    p.seek(5.0);
    assert_eq!(p.status(), Status::Loading, "nothing loaded: no-op");
    assert_eq!(p.shared.generation.load(Ordering::SeqCst), 0);
    loaded(&p);
    lock(&p.shared.state).frames.push_back(frame(1.0));
    p.seek(5.0);
    {
        let st = lock(&p.shared.state);
        assert_eq!(st.from, (1, 5.0));
        assert!(st.frames.is_empty());
        assert_eq!(st.status, Status::Loading);
    }
    assert_eq!(p.shared.generation.load(Ordering::SeqCst), 1);
    assert!((p.progress().0 - 5.0).abs() < 1e-9);
    // Past the end plays the last moment; before the start plays from 0.
    p.seek(100.0);
    assert_eq!(lock(&p.shared.state).from, (2, 11.9));
    p.seek(-3.0);
    assert_eq!(lock(&p.shared.state).from, (0, 0.0));
    // Live streams and failed videos can't be sought.
    lock(&p.shared.state).live = true;
    p.seek(6.0);
    assert_eq!(lock(&p.shared.state).from, (0, 0.0));
}

#[test]
fn a_paused_clock_stands_still_and_progress_stays_in_bounds() {
    let p = player();
    loaded(&p);
    lock(&p.shared.state).clock.set(3.0, true);
    p.set_paused(true);
    assert!(p.paused() && !p.active());
    let (at, length) = p.progress();
    std::thread::sleep(Duration::from_millis(20));
    assert!((p.progress().0 - at).abs() < 1e-9, "paused");
    assert!((3.0..3.1).contains(&at));
    assert!((length - 12.0).abs() < 1e-9);
    lock(&p.shared.state).clock.set(50.0, false);
    assert!((p.progress().0 - 12.0).abs() < 1e-9, "never past the end");
    p.set_paused(false);
    assert!(p.active());
}

#[test]
fn frames_are_shown_when_due_and_the_video_ends_after_the_last() {
    let p = player();
    loaded(&p);
    let pictures = p.pictures();
    let frames = [frame(0.0), frame(0.1), frame(0.5)];
    let ids: Vec<_> = frames.iter().map(|f| f.handle.id()).collect();
    {
        let mut st = lock(&p.shared.state);
        st.frames.extend(frames);
        st.clock.set(0.2, false);
    }
    assert!(pictures.due().is_none());
    assert!(pictures.advance().is_some(), "playing: look again soon");
    assert_eq!(
        lock(&p.shared.state).frames.len(),
        1,
        "0.0 and 0.1 were due"
    );
    let due = pictures.due().map(|f| f.id());
    assert_eq!(due, Some(ids[1]), "the latest due frame");

    lock(&p.shared.state).eof = true;
    lock(&p.shared.state).clock.set(0.6, false);
    pictures.advance();
    assert_eq!(pictures.due().map(|f| f.id()), Some(ids[2]));
    assert_eq!(p.status(), Status::Ended);
    assert!(pictures.advance().is_none(), "nothing changes once ended");
}

#[test]
fn dropping_the_player_stops_everything() {
    let p = player();
    let shared = p.shared.clone();
    drop(p);
    assert!(shared.stop.load(Ordering::SeqCst));
    assert!(shared.audio.stop.load(Ordering::SeqCst));
    assert!(shared.stale(0));
}

fn decoder(shared: &Arc<Shared>) -> Decoder {
    let inits = (Some(mp4::init(VIDEO_INIT)), Some(mp4::init(AUDIO_INIT)));
    let mut d = Decoder::new(shared.clone(), inits).unwrap();
    d.restart(shared.generation.load(Ordering::SeqCst)).unwrap();
    d
}

#[test]
fn the_decoder_turns_segments_into_pictures_and_sound() {
    let p = player();
    let mut d = decoder(&p.shared);
    d.segment(0.0, VIDEO, Some(AUDIO)).unwrap();
    d.flush();
    let st = lock(&p.shared.state);
    assert_eq!(st.status, Status::Playing);
    assert_eq!(st.size, Some((64, 48)));
    let times: Vec<f64> = st.frames.iter().map(|f| f.time).collect();
    assert_eq!(times.len(), 4, "{times:?}");
    assert!(times.windows(2).all(|w| (w[1] - w[0] - 0.1).abs() < 1e-6));
    drop(st);
    assert_eq!(p.shared.audio.time(), Some(0.0), "sound sets the clock");
}

#[test]
fn after_a_seek_the_decoder_skips_what_comes_before() {
    let p = player();
    lock(&p.shared.state).from = (0, 0.15);
    let mut d = decoder(&p.shared);
    d.segment(0.0, VIDEO, Some(AUDIO)).unwrap();
    d.flush();
    let times: Vec<f64> = lock(&p.shared.state)
        .frames
        .iter()
        .map(|f| f.time)
        .collect();
    assert_eq!(times.len(), 2, "{times:?}");
    assert!(times[0] >= 0.15);
}

#[test]
fn a_seek_while_decoding_throws_the_old_work_away() {
    let p = player();
    let mut d = decoder(&p.shared);
    p.shared.generation.fetch_add(1, Ordering::SeqCst);
    d.segment(0.0, VIDEO, None).unwrap();
    d.flush();
    assert!(lock(&p.shared.state).frames.is_empty());
}

#[test]
fn segments_the_player_cannot_decode_fail() {
    let p = player();
    let mut d = decoder(&p.shared);
    assert!(matches!(
        d.segment(0.0, b"not video", None),
        Err(PlayError::Unsupported)
    ));
}
