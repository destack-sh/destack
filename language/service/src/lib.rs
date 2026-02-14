#[cfg(feature = "query")]
pub mod query;
pub mod service;

pub use service::{
    AnalyzeOutcome, FileSnapshot, LanguageService, LanguageServiceError, LanguageServiceResult,
    RescanReason, WorkspaceHandleId, WorkspaceMessage, WorkspaceMessageKind, WorkspaceUpdateRecord,
};

#[cfg(test)]
mod tests;
