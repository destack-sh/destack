mod analysis;
mod error;
mod file;
mod lifecycle;
mod query;
mod types;
mod update;
mod workspace;

pub use error::WorkspaceServiceError;
pub use types::{
    AnalyzeOutcome, FileSnapshot, RescanReason, WorkspaceHandleId, WorkspaceMessage,
    WorkspaceMessageKind, WorkspaceServiceResult, WorkspaceUpdateRecord,
};
pub use workspace::WorkspaceService;
