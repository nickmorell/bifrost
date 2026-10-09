//! Decoding pictures fetched from the network into pixels, within limits, so the
//! window only ever draws pixels that are ready: no decoding on the drawing thread,
//! and no file can make BiFrost allocate more than the limits allow.

use super::MediaError;
use ::image::ImageError;
use ::image::codecs::gif::GifDecoder;
use ::image::imageops::{FilterType, resize};
use ::image::{AnimationDecoder, DynamicImage, ImageDecoder, ImageReader, Limits, RgbaImage};
use std::io::Cursor;
use std::time::Duration;

/// The most pixels a picture may claim on a side, and the most memory decoding one
/// may take, both checked before anything is allocated: a tiny file can claim to be
/// 65535×65535, which would otherwise ask for 16 GB.
const MAX_DIMENSION: u32 = 8192;
const MAX_ALLOC: u64 = 256 * 1024 * 1024;
/// Largest side a still picture is kept at; bigger ones are shrunk to it.
const MAX_SIDE: u32 = 2048;
/// The same for each frame of a GIF, the most decoded pixel data kept per GIF, and
/// the most frames read.
const GIF_MAX_SIDE: u32 = 560;
const GIF_MAX_BYTES: usize = 48 * 1024 * 1024;
const GIF_MAX_FRAMES: usize = 1000;

/// A decoded picture's pixels, 8-bit RGBA.
#[derive(Debug, Clone)]
pub struct Pixels {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// A decoded picture.
#[derive(Debug, Clone)]
pub enum Decoded {
    Still(Pixels),
    /// An animated GIF: each frame and how long it shows.
    Animated(Vec<(Pixels, Duration)>),
}

/// Decodes a picture: a JPEG, PNG, WebP, BMP or GIF.
pub fn decode(bytes: &[u8]) -> Result<Decoded, MediaError> {
    if bytes.starts_with(b"GIF8") {
        let mut frames = gif_frames(bytes)?;
        return Ok(if frames.len() == 1 {
            Decoded::Still(frames.remove(0).0)
        } else {
            Decoded::Animated(frames)
        });
    }
    still(bytes).map(Decoded::Still)
}

/// Why a picture couldn't be decoded.
fn why(e: ImageError) -> MediaError {
    match e {
        ImageError::Limits(e) => MediaError::TooLarge(e.to_string()),
        other => MediaError::Unsupported(other.to_string()),
    }
}

fn limits() -> Limits {
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(MAX_ALLOC);
    limits
}

/// A still picture, upright and no bigger than [`MAX_SIDE`].
fn still(bytes: &[u8]) -> Result<Pixels, MediaError> {
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| MediaError::Read(e.to_string()))?;
    reader.limits(limits());
    let mut decoder = reader.into_decoder().map_err(why)?;
    let orientation = decoder.orientation().ok();
    let mut img = DynamicImage::from_decoder(decoder).map_err(why)?;
    if let Some(o) = orientation {
        img.apply_orientation(o);
    }
    Ok(pixels(shrink(img.into_rgba8(), MAX_SIDE)))
}

/// A GIF's frames, each no bigger than [`GIF_MAX_SIDE`].
fn gif_frames(bytes: &[u8]) -> Result<Vec<(Pixels, Duration)>, MediaError> {
    let mut decoder = GifDecoder::new(Cursor::new(bytes)).map_err(why)?;
    let mut limits = limits();
    limits.max_image_width = Some(MAX_DIMENSION / 2);
    limits.max_image_height = Some(MAX_DIMENSION / 2);
    decoder.set_limits(limits).map_err(why)?;
    let mut frames = Vec::new();
    let mut used = 0usize;
    for frame in decoder.into_frames().take(GIF_MAX_FRAMES) {
        let frame = frame.map_err(why)?;
        let (num, den) = frame.delay().numer_denom_ms();
        // Browsers treat very short delays as 100 ms; do the same.
        let ms = num.checked_div(den).unwrap_or(0);
        let delay = Duration::from_millis(if ms < 20 { 100 } else { u64::from(ms) });
        let buf = shrink(frame.into_buffer(), GIF_MAX_SIDE);
        used += buf.as_raw().len();
        if used > GIF_MAX_BYTES && !frames.is_empty() {
            break;
        }
        frames.push((pixels(buf), delay));
    }
    if frames.is_empty() {
        return Err(MediaError::Unsupported("empty GIF".into()));
    }
    Ok(frames)
}

/// `img` scaled down to fit `side` on its longest side, keeping its shape.
fn shrink(img: RgbaImage, side: u32) -> RgbaImage {
    let longest = img.width().max(img.height());
    if longest <= side {
        return img;
    }
    let scale = side as f32 / longest as f32;
    resize(
        &img,
        ((img.width() as f32 * scale) as u32).max(1),
        ((img.height() as f32 * scale) as u32).max(1),
        FilterType::Triangle,
    )
}

fn pixels(img: RgbaImage) -> Pixels {
    Pixels {
        width: img.width(),
        height: img.height(),
        rgba: img.into_raw(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encoded(img: RgbaImage, format: ::image::ImageFormat) -> Vec<u8> {
        let mut out = Vec::new();
        DynamicImage::ImageRgba8(img)
            .write_to(&mut Cursor::new(&mut out), format)
            .unwrap();
        out
    }

    #[test]
    fn stills_decode_to_pixels_and_large_ones_shrink() {
        let png = encoded(
            RgbaImage::from_pixel(30, 20, ::image::Rgba([1, 2, 3, 255])),
            ::image::ImageFormat::Png,
        );
        let Decoded::Still(p) = decode(&png).unwrap() else {
            panic!("a still");
        };
        assert_eq!((p.width, p.height, p.rgba.len()), (30, 20, 30 * 20 * 4));
        assert_eq!(&p.rgba[..4], &[1, 2, 3, 255]);

        let big = encoded(RgbaImage::new(4096, 1024), ::image::ImageFormat::Png);
        let Decoded::Still(p) = decode(&big).unwrap() else {
            panic!("a still");
        };
        assert_eq!((p.width, p.height), (2048, 512));
        assert!(decode(b"not a picture").is_err());
    }

    #[test]
    fn oversized_pictures_are_refused() {
        // A 1×1 image on a claimed 65535×65535 screen.
        let mut gif = b"GIF89a".to_vec();
        gif.extend([0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00]);
        gif.extend([0x2C, 0, 0, 0, 0, 1, 0, 1, 0, 0x80]);
        gif.extend([0, 0, 0, 255, 255, 255]);
        gif.extend([0x02, 0x02, 0x44, 0x01, 0x00, 0x3B]);
        assert!(matches!(decode(&gif), Err(MediaError::TooLarge(_))));
        // A PNG header claiming 60000×60000.
        let mut png = encoded(RgbaImage::new(1, 1), ::image::ImageFormat::Png);
        png[16..20].copy_from_slice(&60000u32.to_be_bytes());
        png[20..24].copy_from_slice(&60000u32.to_be_bytes());
        assert!(decode(&png).is_err());
    }

    #[test]
    fn animated_gifs_decode_into_frames() {
        use ::image::codecs::gif::{GifEncoder, Repeat};
        use ::image::{Delay, Frame};
        let mut bytes = Vec::new();
        {
            let mut enc = GifEncoder::new(&mut bytes);
            enc.set_repeat(Repeat::Infinite).unwrap();
            for c in [0u8, 255] {
                let img = RgbaImage::from_pixel(8, 4, ::image::Rgba([c, 0, 0, 255]));
                enc.encode_frame(Frame::from_parts(
                    img,
                    0,
                    0,
                    Delay::from_numer_denom_ms(50, 1),
                ))
                .unwrap();
            }
        }
        let Decoded::Animated(frames) = decode(&bytes).unwrap() else {
            panic!("animated");
        };
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].1, Duration::from_millis(50));
        assert_eq!((frames[1].0.width, frames[1].0.height), (8, 4));
    }
}
