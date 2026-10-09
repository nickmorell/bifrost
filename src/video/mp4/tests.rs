//! Tested on short clips made by ffmpeg as HLS fMP4: 0.4 s of 64x48 H.264 at 10 frames
//! a second, and 0.1 s of stereo AAC at 48 kHz, each an init and one media segment.

use super::*;
use crate::video::ts;

const VIDEO_INIT: &[u8] = include_bytes!("testdata/v_init.mp4");
const VIDEO: &[u8] = include_bytes!("testdata/v_0.m4s");
const AUDIO_INIT: &[u8] = include_bytes!("testdata/a_init.mp4");
const AUDIO: &[u8] = include_bytes!("testdata/a_0.m4s");

#[test]
fn tells_mp4_from_transport_streams() {
    assert!(looks_like(VIDEO_INIT));
    assert!(looks_like(VIDEO));
    assert!(!looks_like(&[ts::SYNC; 188]));
    assert!(!looks_like(b"abc"));
}

#[test]
fn reads_the_tracks_from_init_segments() {
    let v = init(VIDEO_INIT);
    assert_eq!(v.tracks.len(), 1);
    let Kind::Video {
        params,
        length_size,
    } = &v.tracks[0].kind
    else {
        panic!("not video: {:?}", v.tracks[0].kind);
    };
    assert_eq!(*length_size, 4);
    // The sequence parameter set (NAL type 7) comes first.
    assert_eq!(&params[..5], &[0, 0, 0, 1, 0x67]);

    let a = init(AUDIO_INIT);
    assert_eq!(
        a.tracks[0].kind,
        Kind::Audio {
            object_type: 2,
            rate_index: 3,
            channels: 2
        }
    );
    assert_eq!(a.tracks[0].timescale, 48_000);
    assert_eq!(init(b"not an mp4"), Init::default());
}

#[test]
fn video_comes_out_as_pictures_the_decoder_reads() {
    let d = demux(VIDEO, &init(VIDEO_INIT));
    assert!(!d.unsupported_video);
    assert_eq!(d.video.len(), 4);
    assert!(d.video.iter().all(|u| u.data.starts_with(&[0, 0, 0, 1])));
    let mut times: Vec<u64> = d.video.iter().filter_map(|u| u.pts).collect();
    times.sort_unstable();
    let steps: Vec<u64> = times.windows(2).map(|w| w[1] - w[0]).collect();
    assert_eq!(
        steps,
        [9000, 9000, 9000],
        "10 frames a second in 90 kHz ticks"
    );

    let mut decoder = openh264::decoder::Decoder::new().unwrap();
    let mut pictures = 0;
    for unit in &d.video {
        if let Ok(Some(yuv)) = decoder.decode(&unit.data) {
            use openh264::formats::YUVSource;
            assert_eq!(yuv.dimensions(), (64, 48));
            pictures += 1;
        }
    }
    assert!(pictures > 0);
}

#[test]
fn sound_comes_out_as_adts_frames() {
    let d = demux(AUDIO, &init(AUDIO_INIT));
    assert!(d.video.is_empty());
    assert!(d.audio.len() >= 4, "{} frames", d.audio.len());
    for unit in &d.audio {
        let frames = ts::adts_frames(&unit.data);
        assert_eq!(frames.len(), 1);
        let (h, raw) = frames[0];
        assert_eq!(h.sample_rate(), 48_000);
        assert_eq!(h.channels, 2);
        assert_eq!(h.object_type, 2);
        assert_eq!(raw.len() + 7, unit.data.len());
    }
}

#[test]
fn segments_without_their_init_give_nothing() {
    let d = demux(VIDEO, &Init::default());
    assert!(d.video.is_empty() && d.audio.is_empty());
    assert!(demux(b"\0\0\0\x08moof", &init(VIDEO_INIT)).video.is_empty());
}

#[test]
fn length_prefixes_become_start_codes() {
    let mut out = Vec::new();
    annex_b(&[0, 0, 0, 2, 9, 9, 0, 0, 0, 1, 7, 0, 0, 0, 9], 4, &mut out);
    assert_eq!(out, [0, 0, 0, 1, 9, 9, 0, 0, 0, 1, 7]);
}

/// A `trun` body: version and flags, the sample count, then `fields` bytes.
fn trun(flags: u32, count: u32, fields: &[u8]) -> Vec<u8> {
    let mut t = flags.to_be_bytes().to_vec();
    t.extend_from_slice(&count.to_be_bytes());
    t.extend_from_slice(fields);
    t
}

#[test]
fn runs_cant_claim_more_samples_than_they_hold() {
    // No per-sample fields: billions of samples would be billions of allocations.
    assert!(run(&trun(0, u32::MAX, &[]), 1, 0).is_none());
    assert_eq!(run(&trun(0, 3, &[]), 1, 0).map(|r| r.items.len()), Some(3));
    // Sizes for two samples, but a claim of a thousand.
    let sizes = [0, 0, 0, 5, 0, 0, 0, 6];
    assert!(run(&trun(0x200, 1000, &sizes), 1, 0).is_none());
    let two = run(&trun(0x200, 2, &sizes), 1, 0).map(|r| r.items);
    assert_eq!(
        two.map(|s| s.iter().map(|s| s.size).collect::<Vec<_>>()),
        Some(vec![5, 6])
    );
}

#[test]
fn cut_off_segments_never_panic() {
    let video = init(VIDEO_INIT);
    for n in 0..VIDEO_INIT.len() {
        let _ = init(&VIDEO_INIT[..n]);
    }
    for n in 0..VIDEO.len() {
        let d = demux(&VIDEO[..n], &video);
        assert!(d.video.len() <= 4);
    }
    let audio = init(AUDIO_INIT);
    for n in 0..AUDIO.len() {
        let _ = demux(&AUDIO[..n], &audio);
    }
}
