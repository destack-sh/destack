use std::collections::HashMap;

use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use destack_source::{Diagnostic, FileId};

use super::{PayloadChunkNotification, RootId};
use crate::{Message, ProgressEvent};

/// Diagnostic batch for notifications.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct DiagnosticBatch {
    /// The file id for these diagnostics.
    pub file_id: FileId,
    /// Diagnostics for the file.
    pub diagnostics: Vec<Diagnostic>,
}

impl DiagnosticBatch {
    /// Group diagnostics by primary file id.
    pub fn group(diagnostics: &[Diagnostic]) -> Vec<Self> {
        let mut by_file = HashMap::new();
        for diagnostic in diagnostics {
            let file_id = diagnostic.primary_label().target.file();
            by_file
                .entry(file_id)
                .or_insert_with(Vec::new)
                .push(diagnostic.clone());
        }

        let mut batches: Vec<Self> = by_file
            .into_iter()
            .map(|(file_id, diagnostics)| Self {
                file_id,
                diagnostics,
            })
            .collect();
        batches.sort_by_key(|batch| batch.file_id.0);

        batches
    }
}

/// Notification for diagnostics updates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct DiagnosticsNotification {
    /// Root handle.
    pub handle: RootId,
    /// Diagnostics grouped by file.
    pub diagnostics: Vec<DiagnosticBatch>,
}

/// Notification for workspace messages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct MessageNotification {
    /// Root handle.
    pub handle: RootId,
    /// Messages emitted by the workspace.
    pub messages: Vec<Message>,
}

/// Progress notification payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ProgressNotification {
    /// Root handle.
    pub handle: RootId,
    /// Progress event payload.
    pub event: ProgressEvent,
}

/// Notifications emitted by the workspace protocol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum WorkspaceNotification {
    /// Publish diagnostics for a root.
    Diagnostics(DiagnosticsNotification),
    /// Publish workspace messages.
    Messages(MessageNotification),
    /// Publish progress updates.
    Progress(ProgressNotification),
    /// Publish chunked payload data.
    PayloadChunk(PayloadChunkNotification),
}
