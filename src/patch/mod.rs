//! Portable rack instruments. Compilation policy belongs to the host.
//!
//! All modules render through [`crate::core::graph::System`]. Metadata is an
//! executable contract: unsupported capabilities are rejected during preparation.
mod compile;
mod document;
pub use document::{DocumentError, parse_document, serialize_document};
mod modules;
mod session;
pub use compile::PreparedPatch;
pub use modules::{ModuleRegistry, RuntimeModule};
pub use session::{
    ParameterEdit, PatchApply, PatchCommand, PatchRuntime, PatchSession, PatchSnapshot,
    PreparedEdit,
};
pub use treble_meta as metadata;

use treble_meta::{ValidationError, ValidationErrors};

pub(crate) fn error(path: impl Into<String>, message: impl Into<String>) -> ValidationErrors {
    ValidationErrors(vec![ValidationError {
        path: path.into(),
        message: message.into(),
    }])
}
