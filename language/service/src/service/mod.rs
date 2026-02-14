mod analysis;
mod error;
mod file;
mod lifecycle;
#[cfg(feature = "query")]
mod query;
mod service;
mod types;
mod update;
mod workspace;

pub use error::LanguageServiceError;
pub use service::LanguageService;
pub use types::{
    AnalyzeOutcome, FileSnapshot, LanguageServiceResult, RescanReason, WorkspaceHandleId,
    WorkspaceMessage, WorkspaceMessageKind, WorkspaceUpdateRecord,
};
