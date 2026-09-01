use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::Cursor;

use crate::PlatformFuture;
use image::{ImageFormat, ImageReader};

/// Maximum text accepted from a clipboard or QR platform boundary.
pub const MAX_CANDIDATE_PAYLOAD_BYTES: usize = 4096;
/// Maximum compressed JPEG or PNG supplied by system image capture.
pub const MAX_QR_ENCODED_IMAGE_BYTES: usize = 8 * 1024 * 1024;
/// Maximum width or height accepted before image frame allocation.
pub const MAX_QR_IMAGE_DIMENSION: u32 = 4096;
/// Maximum decoded pixels accepted before image frame allocation.
pub const MAX_QR_DECODED_PIXELS: u64 = 4096 * 4096;

/// Bounded UTF-8 text supplied as a possible LXMF destination.
///
/// This type only enforces platform-boundary safety. The backend remains
/// authoritative for interpreting and validating the destination.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidatePayload(String);

impl CandidatePayload {
    /// Create a bounded candidate from text already validated as UTF-8.
    ///
    /// # Errors
    ///
    /// Returns [`CandidatePayloadError::Oversized`] when the UTF-8 encoding is
    /// larger than [`MAX_CANDIDATE_PAYLOAD_BYTES`].
    pub fn new(value: impl Into<String>) -> Result<Self, CandidatePayloadError> {
        let value = value.into();
        if value.len() > MAX_CANDIDATE_PAYLOAD_BYTES {
            return Err(CandidatePayloadError::Oversized);
        }
        Ok(Self(value))
    }

    /// Decode and bound raw bytes received from a platform service.
    ///
    /// # Errors
    ///
    /// Returns a typed error for an oversized or non-UTF-8 service payload.
    pub fn from_service_bytes(value: Vec<u8>) -> Result<Self, CandidatePayloadError> {
        if value.len() > MAX_CANDIDATE_PAYLOAD_BYTES {
            return Err(CandidatePayloadError::Oversized);
        }
        String::from_utf8(value).map(Self).map_err(|_| CandidatePayloadError::Malformed)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidatePayloadError {
    Oversized,
    Malformed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextAcquisitionGeneration(u64);

impl TextAcquisitionGeneration {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextAcquisitionFailure {
    Denied,
    Restricted,
    Unavailable,
    Oversized,
    Malformed,
    Cancelled,
    NoCode,
    Ambiguous,
    Unsupported,
    Stale,
}

impl TextAcquisitionFailure {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Denied => "denied",
            Self::Restricted => "restricted",
            Self::Unavailable => "unavailable",
            Self::Oversized => "oversized",
            Self::Malformed => "malformed",
            Self::Cancelled => "cancelled",
            Self::NoCode => "no_code",
            Self::Ambiguous => "ambiguous",
            Self::Unsupported => "unsupported",
            Self::Stale => "stale",
        }
    }
}

impl From<CandidatePayloadError> for TextAcquisitionFailure {
    fn from(value: CandidatePayloadError) -> Self {
        match value {
            CandidatePayloadError::Oversized => Self::Oversized,
            CandidatePayloadError::Malformed => Self::Malformed,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextAcquisitionCompletion {
    pub generation: TextAcquisitionGeneration,
    pub result: Result<CandidatePayload, TextAcquisitionFailure>,
}

impl TextAcquisitionCompletion {
    /// Consume a completion only when it belongs to the currently active request.
    ///
    /// Returning `None` prevents a late platform callback from replacing a newer
    /// request's candidate or failure state.
    #[must_use]
    pub fn into_result_for(
        self,
        current: TextAcquisitionGeneration,
    ) -> Option<Result<CandidatePayload, TextAcquisitionFailure>> {
        (self.generation == current).then_some(self.result)
    }

    /// Consume a completion and retain staleness as a typed outcome.
    ///
    /// # Errors
    ///
    /// Returns [`TextAcquisitionFailure::Stale`] when the completion does not
    /// belong to `current`, or the typed acquisition failure otherwise.
    pub fn into_typed_result_for(
        self,
        current: TextAcquisitionGeneration,
    ) -> Result<CandidatePayload, TextAcquisitionFailure> {
        if self.generation == current { self.result } else { Err(TextAcquisitionFailure::Stale) }
    }
}

/// Reads text from the system clipboard without interpreting it as a destination.
pub trait ClipboardTextReader {
    fn read_clipboard_text(
        &self,
        generation: TextAcquisitionGeneration,
    ) -> PlatformFuture<'_, TextAcquisitionCompletion>;
}

/// Writes an already-selected public value to the system clipboard.
pub trait ClipboardTextWriter {
    fn write_clipboard_text(
        &self,
        value: String,
    ) -> PlatformFuture<'_, Result<(), crate::PlatformFailure>>;
}

/// Scans one possible LXMF destination without validating backend syntax.
pub trait QrDestinationScanner {
    fn scan_qr_destination(
        &self,
        generation: TextAcquisitionGeneration,
        encoded_image: Vec<u8>,
    ) -> PlatformFuture<'_, TextAcquisitionCompletion>;
}

/// Decode one bounded JPEG or PNG containing exactly one QR symbol.
///
/// The returned text is only a candidate. Destination interpretation remains a
/// backend responsibility.
///
/// # Errors
///
/// Returns a payload-free typed failure when an image violates a resource
/// bound, has an unsupported format, cannot be decoded, contains anything other
/// than one QR symbol, or contains a non-UTF-8 or oversized QR payload.
pub fn decode_qr_destination_image(
    encoded_image: &[u8],
) -> Result<CandidatePayload, TextAcquisitionFailure> {
    if encoded_image.len() > MAX_QR_ENCODED_IMAGE_BYTES {
        return Err(TextAcquisitionFailure::Oversized);
    }

    let reader = ImageReader::new(Cursor::new(encoded_image))
        .with_guessed_format()
        .map_err(|_| TextAcquisitionFailure::Malformed)?;
    match reader.format() {
        Some(ImageFormat::Jpeg | ImageFormat::Png) => {}
        Some(_) => return Err(TextAcquisitionFailure::Unsupported),
        None => return Err(TextAcquisitionFailure::Malformed),
    }
    let (width, height) =
        reader.into_dimensions().map_err(|_| TextAcquisitionFailure::Malformed)?;
    if width > MAX_QR_IMAGE_DIMENSION
        || height > MAX_QR_IMAGE_DIMENSION
        || u64::from(width) * u64::from(height) > MAX_QR_DECODED_PIXELS
    {
        return Err(TextAcquisitionFailure::Oversized);
    }

    let grayscale = ImageReader::new(Cursor::new(encoded_image))
        .with_guessed_format()
        .map_err(|_| TextAcquisitionFailure::Malformed)?
        .decode()
        .map_err(|_| TextAcquisitionFailure::Malformed)?
        .into_luma8();
    let mut decoder = quircs::Quirc::default();
    let mut codes = decoder.identify(width as usize, height as usize, grayscale.as_raw());
    let Some(code) = codes.next() else {
        return Err(TextAcquisitionFailure::NoCode);
    };
    if codes.next().is_some() {
        return Err(TextAcquisitionFailure::Ambiguous);
    }
    let data = code
        .map_err(|_| TextAcquisitionFailure::Malformed)?
        .decode()
        .map_err(|_| TextAcquisitionFailure::Malformed)?;
    CandidatePayload::from_service_bytes(data.payload).map_err(Into::into)
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ImageQrDestinationScanner;

impl QrDestinationScanner for ImageQrDestinationScanner {
    fn scan_qr_destination(
        &self,
        generation: TextAcquisitionGeneration,
        encoded_image: Vec<u8>,
    ) -> PlatformFuture<'_, TextAcquisitionCompletion> {
        Box::pin(async move {
            TextAcquisitionCompletion {
                generation,
                result: decode_qr_destination_image(&encoded_image),
            }
        })
    }
}

/// One deterministic response produced by a platform-service mock.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MockTextAcquisitionResponse {
    ServiceBytes(Vec<u8>),
    Denied,
    Restricted,
    Unavailable,
    Cancelled,
}

impl MockTextAcquisitionResponse {
    fn into_result(self) -> Result<CandidatePayload, TextAcquisitionFailure> {
        match self {
            Self::ServiceBytes(value) => {
                CandidatePayload::from_service_bytes(value).map_err(Into::into)
            }
            Self::Denied => Err(TextAcquisitionFailure::Denied),
            Self::Restricted => Err(TextAcquisitionFailure::Restricted),
            Self::Unavailable => Err(TextAcquisitionFailure::Unavailable),
            Self::Cancelled => Err(TextAcquisitionFailure::Cancelled),
        }
    }
}

#[derive(Debug)]
struct MockResponses {
    scripted: RefCell<VecDeque<MockTextAcquisitionResponse>>,
}

impl MockResponses {
    fn new(responses: impl IntoIterator<Item = MockTextAcquisitionResponse>) -> Self {
        Self { scripted: RefCell::new(responses.into_iter().collect()) }
    }

    fn complete(&self, generation: TextAcquisitionGeneration) -> TextAcquisitionCompletion {
        let response = self
            .scripted
            .borrow_mut()
            .pop_front()
            .unwrap_or(MockTextAcquisitionResponse::Unavailable);
        TextAcquisitionCompletion { generation, result: response.into_result() }
    }
}

/// Scripted clipboard reader. Responses are consumed in insertion order.
#[derive(Debug)]
pub struct MockClipboardTextReader {
    responses: MockResponses,
}

impl MockClipboardTextReader {
    #[must_use]
    pub fn new(responses: impl IntoIterator<Item = MockTextAcquisitionResponse>) -> Self {
        Self { responses: MockResponses::new(responses) }
    }
}

impl ClipboardTextReader for MockClipboardTextReader {
    fn read_clipboard_text(
        &self,
        generation: TextAcquisitionGeneration,
    ) -> PlatformFuture<'_, TextAcquisitionCompletion> {
        let completion = self.responses.complete(generation);
        Box::pin(async move { completion })
    }
}

/// Scripted clipboard writer that records every requested value.
#[derive(Debug)]
pub struct MockClipboardTextWriter {
    responses: RefCell<VecDeque<Result<(), crate::PlatformFailure>>>,
    writes: RefCell<Vec<String>>,
}

impl MockClipboardTextWriter {
    #[must_use]
    pub fn new(responses: impl IntoIterator<Item = Result<(), crate::PlatformFailure>>) -> Self {
        Self {
            responses: RefCell::new(responses.into_iter().collect()),
            writes: RefCell::new(Vec::new()),
        }
    }

    #[must_use]
    pub fn writes(&self) -> Vec<String> {
        self.writes.borrow().clone()
    }
}

impl ClipboardTextWriter for MockClipboardTextWriter {
    fn write_clipboard_text(
        &self,
        value: String,
    ) -> PlatformFuture<'_, Result<(), crate::PlatformFailure>> {
        self.writes.borrow_mut().push(value);
        let response = self.responses.borrow_mut().pop_front().unwrap_or_else(|| {
            Err(crate::PlatformFailure {
                code: "clipboard_write_unavailable".into(),
                retryable: false,
            })
        });
        Box::pin(async move { response })
    }
}

/// Scripted QR scanner. Responses are consumed in insertion order.
#[derive(Debug)]
pub struct MockQrDestinationScanner {
    responses: MockResponses,
}

impl MockQrDestinationScanner {
    #[must_use]
    pub fn new(responses: impl IntoIterator<Item = MockTextAcquisitionResponse>) -> Self {
        Self { responses: MockResponses::new(responses) }
    }
}

impl QrDestinationScanner for MockQrDestinationScanner {
    fn scan_qr_destination(
        &self,
        generation: TextAcquisitionGeneration,
        _: Vec<u8>,
    ) -> PlatformFuture<'_, TextAcquisitionCompletion> {
        let completion = self.responses.complete(generation);
        Box::pin(async move { completion })
    }
}
