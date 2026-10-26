use treble_meta::{InstrumentDocument, ValidationErrors};

#[derive(Debug, thiserror::Error)]
pub enum DocumentError {
    #[error("invalid .trbl JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid .trbl document: {0}")]
    Validation(#[from] ValidationErrors),
}

/// Parse a format-1 JSON `.trbl` instrument, including unavailable or disconnected
/// modules. Runtime registry/capability validation happens separately at prepare.
pub fn parse_document(json: &str) -> Result<InstrumentDocument, DocumentError> {
    let document: InstrumentDocument = serde_json::from_str(json)?;
    document.validate()?;
    Ok(document)
}

/// Save the editable definition and opaque editor data, never runtime state.
/// Hosts own paths, dialogs and atomic file writes.
pub fn serialize_document(document: &InstrumentDocument) -> Result<String, DocumentError> {
    document.validate()?;
    Ok(serde_json::to_string_pretty(document)? + "\n")
}
