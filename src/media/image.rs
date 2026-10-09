//! Images prepared for upload: upright, without metadata and under Bluesky's size
//! limit.

use super::MediaError;

/// Blob size limit for images.
pub const MAX_IMAGE_BYTES: usize = 1_000_000;

/// An image ready to upload.
#[derive(Debug, Clone)]
pub struct Prepared {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Whether a file looks like an image BiFrost can attach.
pub fn is_image_path(path: &std::path::Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        ["jpg", "jpeg", "png", "webp", "gif", "bmp"].contains(&e.to_ascii_lowercase().as_str())
    })
}

/// Decodes an image, turns it upright according to its EXIF orientation, and
/// re-encodes it so that no metadata (camera details, GPS location) is uploaded.
/// Images with transparency stay PNG; everything else becomes JPEG. Images over the
/// blob size limit are shrunk until they fit.
pub fn prepare_image(bytes: Vec<u8>) -> Result<Prepared, MediaError> {
    encode_upload(decode_upright(bytes)?)
}

/// Like [`prepare_image`], but cut to `aspect` (width / height) around the middle and
/// shrunk to at most `max_width` wide, as profile pictures and banners are.
pub fn prepare_cropped(
    bytes: Vec<u8>,
    aspect: f32,
    max_width: u32,
) -> Result<Prepared, MediaError> {
    let img = decode_upright(bytes)?;
    let (w, h) = (img.width(), img.height());
    let (cw, ch) = if w as f32 / h as f32 > aspect {
        (((h as f32 * aspect).round() as u32).clamp(1, w), h)
    } else {
        (w, ((w as f32 / aspect).round() as u32).clamp(1, h))
    };
    let mut img = img.crop_imm((w - cw) / 2, (h - ch) / 2, cw, ch);
    if cw > max_width {
        let nh = ((max_width as f32 / aspect).round() as u32).max(1);
        img = img.resize_exact(max_width, nh, ::image::imageops::FilterType::Lanczos3);
    }
    encode_upload(img)
}

/// Decodes an image and turns it upright according to its EXIF orientation.
fn decode_upright(bytes: Vec<u8>) -> Result<::image::DynamicImage, MediaError> {
    use ::image::{DynamicImage, ImageDecoder, ImageReader};

    let unsupported = |e: &dyn std::fmt::Display| MediaError::Unsupported(e.to_string());
    let mut decoder = ImageReader::new(std::io::Cursor::new(&bytes))
        .with_guessed_format()
        .map_err(|e| unsupported(&e))?
        .into_decoder()
        .map_err(|e| unsupported(&e))?;
    let orientation = decoder.orientation().ok();
    let mut img = DynamicImage::from_decoder(decoder).map_err(|e| unsupported(&e))?;
    if let Some(o) = orientation {
        img.apply_orientation(o);
    }
    Ok(img)
}

/// Encodes a decoded image for upload, without metadata and under the size limit.
fn encode_upload(img: ::image::DynamicImage) -> Result<Prepared, MediaError> {
    use ::image::DynamicImage;

    let (width, height) = (img.width(), img.height());

    let encode = |img: &DynamicImage| -> Result<Vec<u8>, MediaError> {
        let mut out = Vec::new();
        if img.color().has_alpha() {
            img.write_to(
                &mut std::io::Cursor::new(&mut out),
                ::image::ImageFormat::Png,
            )
            .map_err(|e| MediaError::Encode(e.to_string()))?;
        } else {
            let rgb = DynamicImage::ImageRgb8(img.to_rgb8());
            ::image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 90)
                .encode_image(&rgb)
                .map_err(|e| MediaError::Encode(e.to_string()))?;
        }
        Ok(out)
    };

    let mut out = encode(&img)?;
    if out.len() > MAX_IMAGE_BYTES {
        // Too big: drop transparency and shrink as JPEG until it fits.
        let mut max_side = width.max(height).min(2000);
        loop {
            let scaled = if width.max(height) > max_side {
                img.resize(max_side, max_side, ::image::imageops::FilterType::Lanczos3)
            } else {
                img.clone()
            };
            let rgb = DynamicImage::ImageRgb8(scaled.to_rgb8());
            out.clear();
            ::image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 85)
                .encode_image(&rgb)
                .map_err(|e| MediaError::Encode(e.to_string()))?;
            if out.len() <= MAX_IMAGE_BYTES {
                break;
            }
            if max_side < 400 {
                return Err(MediaError::TooLargeToShrink);
            }
            max_side = max_side * 3 / 4;
        }
    }
    Ok(Prepared {
        bytes: out,
        width,
        height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cropping_keeps_the_middle_at_the_aspect() {
        let img = ::image::RgbImage::from_pixel(4000, 1000, ::image::Rgb([10, 200, 30]));
        let mut png = Vec::new();
        ::image::DynamicImage::ImageRgb8(img)
            .write_to(
                &mut std::io::Cursor::new(&mut png),
                ::image::ImageFormat::Png,
            )
            .unwrap();
        let square = prepare_cropped(png.clone(), 1.0, 1000).unwrap();
        assert_eq!((square.width, square.height), (1000, 1000));
        let banner = prepare_cropped(png, 3.0, 1500).unwrap();
        assert_eq!((banner.width, banner.height), (1500, 500));
    }

    #[test]
    fn large_images_are_shrunk_under_the_limit() {
        // Noise compresses badly, so a 2400px PNG is well over 1 MB.
        let mut img = ::image::RgbImage::new(2400, 1600);
        for (i, p) in img.pixels_mut().enumerate() {
            let v = (i.wrapping_mul(2654435761) >> 7) as u8;
            *p = ::image::Rgb([v, v.wrapping_mul(3), v.wrapping_mul(7)]);
        }
        let mut png = Vec::new();
        ::image::DynamicImage::ImageRgb8(img)
            .write_to(
                &mut std::io::Cursor::new(&mut png),
                ::image::ImageFormat::Png,
            )
            .unwrap();
        assert!(png.len() > MAX_IMAGE_BYTES);
        let prepared = prepare_image(png).unwrap();
        assert!(prepared.bytes.len() <= MAX_IMAGE_BYTES);
        assert_eq!((prepared.width, prepared.height), (2400, 1600));
    }

    /// A small JPEG carrying an EXIF block with GPS-looking text and orientation 6
    /// (rotate 90° clockwise).
    fn jpeg_with_exif(w: u32, h: u32) -> Vec<u8> {
        let mut jpeg = Vec::new();
        let img = ::image::RgbImage::from_pixel(w, h, ::image::Rgb([200, 100, 50]));
        ::image::DynamicImage::ImageRgb8(img)
            .write_to(
                &mut std::io::Cursor::new(&mut jpeg),
                ::image::ImageFormat::Jpeg,
            )
            .unwrap();
        // TIFF header, one IFD entry: Orientation (0x0112), SHORT, 1, value 6.
        let mut tiff = b"II*\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0".to_vec();
        tiff.extend_from_slice(b"GPSLatitude 51.5007");
        let mut app1 = b"Exif\0\0".to_vec();
        app1.extend_from_slice(&tiff);
        let len = (app1.len() + 2) as u16;
        let mut out = jpeg[..2].to_vec();
        out.extend_from_slice(&[0xFF, 0xE1]);
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(&app1);
        out.extend_from_slice(&jpeg[2..]);
        out
    }

    #[test]
    fn metadata_is_stripped_and_orientation_applied() {
        let src = jpeg_with_exif(40, 20);
        let contains = |hay: &[u8], needle: &[u8]| hay.windows(needle.len()).any(|w| w == needle);
        assert!(contains(&src, b"Exif") && contains(&src, b"GPSLatitude"));
        let prepared = prepare_image(src).unwrap();
        assert!(!contains(&prepared.bytes, b"Exif"));
        assert!(!contains(&prepared.bytes, b"GPSLatitude"));
        // Rotated upright, so the stored size is portrait.
        assert_eq!((prepared.width, prepared.height), (20, 40));
        let out = ::image::load_from_memory(&prepared.bytes).unwrap();
        assert_eq!((out.width(), out.height()), (20, 40));
    }
}
