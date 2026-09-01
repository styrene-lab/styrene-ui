use std::cell::RefCell;
use std::collections::VecDeque;

use crate::PlatformFuture;

/// Maximum text accepted from a clipboard or QR platform boundary.
pub const MAX_CANDIDATE_PAYLOAD_BYTES: usize = 4096;

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
}

/// Reads text from the system clipboard without interpreting it as a destination.
pub trait ClipboardTextReader {
    fn read_clipboard_text(
        &self,
        generation: TextAcquisitionGeneration,
    ) -> PlatformFuture<'_, TextAcquisitionCompletion>;
}

/// Scans one possible LXMF destination without validating backend syntax.
pub trait QrDestinationScanner {
    fn scan_qr_destination(
        &self,
        generation: TextAcquisitionGeneration,
    ) -> PlatformFuture<'_, TextAcquisitionCompletion>;
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
    ) -> PlatformFuture<'_, TextAcquisitionCompletion> {
        let completion = self.responses.complete(generation);
        Box::pin(async move { completion })
    }
}
