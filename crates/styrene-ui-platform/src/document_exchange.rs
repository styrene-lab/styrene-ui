use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt;

use crate::PlatformFuture;

/// Maximum opaque document size accepted at the platform boundary.
pub const MAX_OPAQUE_DOCUMENT_BYTES: usize = 16 * 1024 * 1024;

/// Bounded bytes transported without inspecting or interpreting their contents.
#[derive(Clone, Eq, PartialEq)]
pub struct OpaqueDocument(Vec<u8>);

impl OpaqueDocument {
    /// Create a bounded opaque document.
    ///
    /// # Errors
    ///
    /// Returns [`OpaqueDocumentError::Oversized`] when `bytes` exceeds
    /// [`MAX_OPAQUE_DOCUMENT_BYTES`].
    pub fn new(bytes: Vec<u8>) -> Result<Self, OpaqueDocumentError> {
        if bytes.len() > MAX_OPAQUE_DOCUMENT_BYTES {
            return Err(OpaqueDocumentError::Oversized);
        }
        Ok(Self(bytes))
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }
}

impl fmt::Debug for OpaqueDocument {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpaqueDocument")
            .field("bytes", &"[REDACTED]")
            .field("len", &self.0.len())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OpaqueDocumentError {
    Oversized,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DocumentRequestGeneration(u64);

impl DocumentRequestGeneration {
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
pub enum DocumentPickerFailure {
    Cancelled,
    Oversized,
    Unavailable,
    ReadFailed,
}

impl From<OpaqueDocumentError> for DocumentPickerFailure {
    fn from(value: OpaqueDocumentError) -> Self {
        match value {
            OpaqueDocumentError::Oversized => Self::Oversized,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentPickerCompletion {
    pub generation: DocumentRequestGeneration,
    pub result: Result<OpaqueDocument, DocumentPickerFailure>,
}

impl DocumentPickerCompletion {
    /// Consume a completion only when it belongs to the currently active request.
    #[must_use]
    pub fn into_result_for(
        self,
        current: DocumentRequestGeneration,
    ) -> Option<Result<OpaqueDocument, DocumentPickerFailure>> {
        (self.generation == current).then_some(self.result)
    }
}

/// Selects and reads one document without interpreting its contents.
pub trait OpaqueDocumentPicker {
    fn pick_document(
        &self,
        generation: DocumentRequestGeneration,
    ) -> PlatformFuture<'_, DocumentPickerCompletion>;
}

/// A share request has been presented to the user.
///
/// This does not assert that the user selected a target or completed sharing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentShareOutcome {
    Presented,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentShareFailure {
    Unavailable,
    PresentationFailed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentShareCompletion {
    pub generation: DocumentRequestGeneration,
    pub result: Result<DocumentShareOutcome, DocumentShareFailure>,
}

impl DocumentShareCompletion {
    /// Consume a completion only when it belongs to the currently active request.
    #[must_use]
    pub fn into_result_for(
        self,
        current: DocumentRequestGeneration,
    ) -> Option<Result<DocumentShareOutcome, DocumentShareFailure>> {
        (self.generation == current).then_some(self.result)
    }
}

/// Presents bounded opaque bytes using the platform share interface.
pub trait OpaqueDocumentSharer {
    fn present_document_share(
        &self,
        generation: DocumentRequestGeneration,
        document: OpaqueDocument,
    ) -> PlatformFuture<'_, DocumentShareCompletion>;
}

/// One deterministic response produced by a document-picker mock.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MockDocumentPickerResponse {
    DocumentBytes(Vec<u8>),
    Cancelled,
    Unavailable,
    ReadFailed,
}

impl MockDocumentPickerResponse {
    fn into_result(self) -> Result<OpaqueDocument, DocumentPickerFailure> {
        match self {
            Self::DocumentBytes(bytes) => OpaqueDocument::new(bytes).map_err(Into::into),
            Self::Cancelled => Err(DocumentPickerFailure::Cancelled),
            Self::Unavailable => Err(DocumentPickerFailure::Unavailable),
            Self::ReadFailed => Err(DocumentPickerFailure::ReadFailed),
        }
    }
}

/// Scripted document picker. Responses are consumed in insertion order.
#[derive(Debug)]
pub struct MockOpaqueDocumentPicker {
    responses: RefCell<VecDeque<MockDocumentPickerResponse>>,
}

impl MockOpaqueDocumentPicker {
    #[must_use]
    pub fn new(responses: impl IntoIterator<Item = MockDocumentPickerResponse>) -> Self {
        Self { responses: RefCell::new(responses.into_iter().collect()) }
    }
}

impl OpaqueDocumentPicker for MockOpaqueDocumentPicker {
    fn pick_document(
        &self,
        generation: DocumentRequestGeneration,
    ) -> PlatformFuture<'_, DocumentPickerCompletion> {
        let response = self
            .responses
            .borrow_mut()
            .pop_front()
            .unwrap_or(MockDocumentPickerResponse::Unavailable);
        let completion = DocumentPickerCompletion { generation, result: response.into_result() };
        Box::pin(async move { completion })
    }
}

/// One deterministic response produced by a document-share mock.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MockDocumentShareResponse {
    Presented,
    Unavailable,
    PresentationFailed,
}

impl MockDocumentShareResponse {
    const fn into_result(self) -> Result<DocumentShareOutcome, DocumentShareFailure> {
        match self {
            Self::Presented => Ok(DocumentShareOutcome::Presented),
            Self::Unavailable => Err(DocumentShareFailure::Unavailable),
            Self::PresentationFailed => Err(DocumentShareFailure::PresentationFailed),
        }
    }
}

/// Scripted document sharer that records every requested opaque document.
#[derive(Debug)]
pub struct MockOpaqueDocumentSharer {
    responses: RefCell<VecDeque<MockDocumentShareResponse>>,
    presentations: RefCell<Vec<OpaqueDocument>>,
}

impl MockOpaqueDocumentSharer {
    #[must_use]
    pub fn new(responses: impl IntoIterator<Item = MockDocumentShareResponse>) -> Self {
        Self {
            responses: RefCell::new(responses.into_iter().collect()),
            presentations: RefCell::new(Vec::new()),
        }
    }

    #[must_use]
    pub fn presentations(&self) -> Vec<OpaqueDocument> {
        self.presentations.borrow().clone()
    }
}

impl OpaqueDocumentSharer for MockOpaqueDocumentSharer {
    fn present_document_share(
        &self,
        generation: DocumentRequestGeneration,
        document: OpaqueDocument,
    ) -> PlatformFuture<'_, DocumentShareCompletion> {
        self.presentations.borrow_mut().push(document);
        let response = self
            .responses
            .borrow_mut()
            .pop_front()
            .unwrap_or(MockDocumentShareResponse::Unavailable);
        let completion = DocumentShareCompletion { generation, result: response.into_result() };
        Box::pin(async move { completion })
    }
}
