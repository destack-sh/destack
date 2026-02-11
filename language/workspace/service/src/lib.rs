pub mod service;

pub use service::{
    AnalyzeOutcome, FileSnapshot, RescanReason, WorkspaceHandleId, WorkspaceMessage,
    WorkspaceMessageKind, WorkspaceService, WorkspaceServiceError, WorkspaceServiceResult,
    WorkspaceUpdateRecord,
};

#[cfg(test)]
mod tests;
