use image::{ExtendedColorType, GrayImage, ImageEncoder, Luma};
use qrcode::{QrCode, types::Color};
use styrene_ui_platform::{
    MAX_QR_ENCODED_IMAGE_BYTES, TextAcquisitionFailure, decode_qr_destination_image,
};

const CANDIDATE: &str = "e01b09b22ccc4e2755d29eead962677b";
const SCALE: u32 = 8;
const QUIET_MODULES: u32 = 4;

fn qr_image(payloads: &[&[u8]]) -> GrayImage {
    let codes = payloads
        .iter()
        .map(|payload| QrCode::new(payload).expect("test payload must fit in a QR symbol"))
        .collect::<Vec<_>>();
    let symbol_sizes = codes
        .iter()
        .map(|code| (u32::try_from(code.width()).unwrap() + QUIET_MODULES * 2) * SCALE)
        .collect::<Vec<_>>();
    let gap = QUIET_MODULES * SCALE * 2;
    let gaps = u32::try_from(symbol_sizes.len().saturating_sub(1)).unwrap();
    let width = symbol_sizes.iter().sum::<u32>() + gap * gaps;
    let height = *symbol_sizes.iter().max().unwrap();
    let mut image = GrayImage::from_pixel(width, height, Luma([255]));
    let mut left = 0;

    for (code, symbol_size) in codes.iter().zip(symbol_sizes) {
        for y in 0..code.width() {
            for x in 0..code.width() {
                let value = if code[(x, y)] == Color::Dark { 0 } else { 255 };
                for dy in 0..SCALE {
                    for dx in 0..SCALE {
                        image.put_pixel(
                            left + (u32::try_from(x).unwrap() + QUIET_MODULES) * SCALE + dx,
                            (u32::try_from(y).unwrap() + QUIET_MODULES) * SCALE + dy,
                            Luma([value]),
                        );
                    }
                }
            }
        }
        left += symbol_size + gap;
    }
    image
}

fn png(image: &GrayImage) -> Vec<u8> {
    let mut bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes)
        .write_image(image.as_raw(), image.width(), image.height(), ExtendedColorType::L8)
        .unwrap();
    bytes
}

fn jpeg(image: &GrayImage) -> Vec<u8> {
    let mut bytes = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 95)
        .write_image(image.as_raw(), image.width(), image.height(), ExtendedColorType::L8)
        .unwrap();
    bytes
}

#[test]
fn jpeg_single_symbol_returns_bounded_candidate() {
    let result = decode_qr_destination_image(&jpeg(&qr_image(&[CANDIDATE.as_bytes()]))).unwrap();
    assert_eq!(result.as_str(), CANDIDATE);
}

#[test]
fn png_single_symbol_returns_bounded_candidate() {
    let result = decode_qr_destination_image(&png(&qr_image(&[CANDIDATE.as_bytes()]))).unwrap();
    assert_eq!(result.as_str(), CANDIDATE);
}

#[test]
fn blank_image_returns_no_code() {
    let image = GrayImage::from_pixel(128, 128, Luma([255]));
    assert_eq!(decode_qr_destination_image(&png(&image)), Err(TextAcquisitionFailure::NoCode));
}

#[test]
fn multiple_symbols_return_ambiguous() {
    let image = qr_image(&[CANDIDATE.as_bytes(), b"not-validated-by-platform"]);
    assert_eq!(decode_qr_destination_image(&png(&image)), Err(TextAcquisitionFailure::Ambiguous));
}

#[test]
fn non_image_bytes_return_malformed() {
    assert_eq!(
        decode_qr_destination_image(b"private-marker-not-an-image"),
        Err(TextAcquisitionFailure::Malformed)
    );
}

#[test]
fn disabled_image_format_returns_unsupported() {
    let gif = b"GIF89a\x01\0\x01\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x3b";
    assert_eq!(decode_qr_destination_image(gif), Err(TextAcquisitionFailure::Unsupported));
}

#[test]
fn encoded_limit_is_checked_before_image_decode() {
    let bytes = vec![0; MAX_QR_ENCODED_IMAGE_BYTES + 1];
    assert_eq!(decode_qr_destination_image(&bytes), Err(TextAcquisitionFailure::Oversized));
}

#[test]
fn pixel_limit_is_checked_before_frame_allocation() {
    let image = GrayImage::from_pixel(4097, 1, Luma([255]));
    assert_eq!(decode_qr_destination_image(&png(&image)), Err(TextAcquisitionFailure::Oversized));
}

#[test]
fn decoded_payload_must_be_utf8() {
    let image = qr_image(&[&[0xff, 0xfe, 0xfd]]);
    assert_eq!(decode_qr_destination_image(&png(&image)), Err(TextAcquisitionFailure::Malformed));
}

#[test]
fn debug_errors_and_diagnostics_exclude_frames_and_payloads() {
    let failure = decode_qr_destination_image(b"sensitive-marker").unwrap_err();
    let diagnostic = format!("{failure:?}");
    assert_eq!(diagnostic, "Malformed");
    assert!(!diagnostic.contains("sensitive-marker"));
}
