use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use destack_service::{FileImage as ServiceFileImage, FileUpdateKind};
use destack_source::{
    Diagnostic, FileContent, FileId, FileWatchEvent, FileWatchEventKind, FileWatchRescanReason,
    FileWatchStatus,
};
use destack_workspace::{Repository, Revision};

use crate::{DaemonMessage, DaemonMessageKind, DaemonUpdate, WatchBatch as DaemonWatchBatch};

use super::{
    DaemonMessageKind as ProtocolMessageKind, DaemonMessageRecord, DaemonUpdateRecord,
    DiagnosticBatch, FileUpdateImage, ReloadReason, UpdateChangeKind as ProtocolUpdateChangeKind,
    UpdateChangeSummary, WatchBatch as ProtocolWatchBatch, WatchEvent as ProtocolWatchEvent,
    WatchEventKind as ProtocolWatchEventKind, WatchStatus as ProtocolWatchStatus,
};

impl From<FileUpdateKind> for ProtocolUpdateChangeKind {
    fn from(kind: FileUpdateKind) -> Self {
        match kind {
            FileUpdateKind::Config => ProtocolUpdateChangeKind::Destack,
            FileUpdateKind::Source => ProtocolUpdateChangeKind::Unknown,
        }
    }
}

impl From<&crate::DaemonUpdate> for UpdateChangeSummary {
    fn from(update: &crate::DaemonUpdate) -> Self {
        Self {
            file_id: update.file_id,
            kind: ProtocolUpdateChangeKind::from(update.kind),
        }
    }
}

impl From<&DaemonUpdate> for DaemonUpdateRecord {
    fn from(update: &DaemonUpdate) -> Self {
        Self {
            module_id: update.module_id,
            file_id: update.file_id,
            file: update.file.as_ref().map(protocol_image_from_root),
            change: UpdateChangeSummary::from(update),
            diagnostics: update.diagnostics.clone(),
        }
    }
}

/// Convert a root image into protocol shape.
fn protocol_image_from_root(image: &ServiceFileImage) -> FileUpdateImage {
    FileUpdateImage {
        id: image.id,
        name: image.name.clone(),
        uri: image.uri.clone(),
        path: image.path.clone(),
        file_type: image.file_type,
        content: image.content.clone(),
    }
}

impl From<&DaemonMessage> for DaemonMessageRecord {
    fn from(message: &DaemonMessage) -> Self {
        let kind = match message.kind {
            DaemonMessageKind::Info => ProtocolMessageKind::Info,
            DaemonMessageKind::Warning => ProtocolMessageKind::Warning,
            DaemonMessageKind::Error => ProtocolMessageKind::Error,
        };

        Self {
            kind,
            code: message.code.clone(),
            message: message.message.clone(),
            path: message.path.clone(),
        }
    }
}

impl From<&FileWatchEventKind> for ProtocolWatchEventKind {
    fn from(kind: &FileWatchEventKind) -> Self {
        match kind {
            FileWatchEventKind::Created => ProtocolWatchEventKind::Created,
            FileWatchEventKind::Modified => ProtocolWatchEventKind::Modified,
            FileWatchEventKind::Deleted => ProtocolWatchEventKind::Deleted,
            FileWatchEventKind::Renamed => ProtocolWatchEventKind::Renamed,
            FileWatchEventKind::Overflow => ProtocolWatchEventKind::Overflow,
        }
    }
}

impl From<&FileWatchRescanReason> for ReloadReason {
    fn from(reason: &FileWatchRescanReason) -> Self {
        match reason {
            FileWatchRescanReason::Startup => ReloadReason::Startup,
            FileWatchRescanReason::Overflow => ReloadReason::Overflow,
            FileWatchRescanReason::Manual => ReloadReason::Manual,
            FileWatchRescanReason::Update => ReloadReason::Update,
        }
    }
}

impl From<&FileWatchEvent> for ProtocolWatchEvent {
    fn from(event: &FileWatchEvent) -> Self {
        Self {
            path: event.path.clone(),
            previous_path: event.previous_path.clone(),
            kind: ProtocolWatchEventKind::from(&event.kind),
        }
    }
}

impl From<&FileWatchStatus> for ProtocolWatchStatus {
    fn from(status: &FileWatchStatus) -> Self {
        match status {
            FileWatchStatus::Ready { roots } => ProtocolWatchStatus::Ready {
                roots: roots.clone(),
            },
            FileWatchStatus::RescanRequested { roots, reason } => {
                ProtocolWatchStatus::ReloadRequested {
                    roots: roots.clone(),
                    reason: ReloadReason::from(reason),
                }
            }
            FileWatchStatus::Error { message } => ProtocolWatchStatus::Error {
                message: message.clone(),
            },
            FileWatchStatus::Stopped => ProtocolWatchStatus::Stopped,
        }
    }
}

impl From<&ProtocolWatchBatch> for DaemonWatchBatch {
    fn from(batch: &ProtocolWatchBatch) -> Self {
        // derive the relative duration from the protocol payload
        let started_at = Instant::now();
        let duration = batch.ended_at_ns.saturating_sub(batch.started_at_ns);
        let ended_at = started_at + Duration::from_nanos(duration);

        Self {
            events: batch
                .events
                .iter()
                .map(ProtocolWatchEvent::to_daemon)
                .collect(),
            status: batch
                .status
                .iter()
                .map(ProtocolWatchStatus::to_daemon)
                .collect(),
            started_at,
            ended_at,
            overflowed: batch.overflowed,
        }
    }
}

impl ProtocolWatchEventKind {
    /// Convert to a daemon watch event kind.
    pub fn to_daemon(self) -> FileWatchEventKind {
        match self {
            ProtocolWatchEventKind::Created => FileWatchEventKind::Created,
            ProtocolWatchEventKind::Modified => FileWatchEventKind::Modified,
            ProtocolWatchEventKind::Deleted => FileWatchEventKind::Deleted,
            ProtocolWatchEventKind::Renamed => FileWatchEventKind::Renamed,
            ProtocolWatchEventKind::Overflow => FileWatchEventKind::Overflow,
        }
    }
}

impl ProtocolWatchEvent {
    /// Convert to a daemon watch event.
    pub fn to_daemon(&self) -> FileWatchEvent {
        FileWatchEvent {
            path: self.path.clone(),
            previous_path: self.previous_path.clone(),
            kind: self.kind.to_daemon(),
        }
    }
}

impl ReloadReason {
    /// Convert to a daemon rescan reason.
    pub fn to_daemon(self) -> FileWatchRescanReason {
        match self {
            ReloadReason::Startup => FileWatchRescanReason::Startup,
            ReloadReason::Overflow => FileWatchRescanReason::Overflow,
            ReloadReason::Manual => FileWatchRescanReason::Manual,
            ReloadReason::Update => FileWatchRescanReason::Update,
        }
    }
}

impl ProtocolWatchStatus {
    /// Convert to a daemon watch status.
    pub fn to_daemon(&self) -> FileWatchStatus {
        match self {
            ProtocolWatchStatus::Ready { roots } => FileWatchStatus::Ready {
                roots: roots.clone(),
            },
            ProtocolWatchStatus::ReloadRequested { roots, reason } => {
                FileWatchStatus::RescanRequested {
                    roots: roots.clone(),
                    reason: reason.to_daemon(),
                }
            }
            ProtocolWatchStatus::Error { message } => FileWatchStatus::Error {
                message: message.clone(),
            },
            ProtocolWatchStatus::Stopped => FileWatchStatus::Stopped,
        }
    }
}

/// Convert daemon updates into protocol records.
pub fn daemon_updates_to_records(updates: &[DaemonUpdate]) -> Vec<DaemonUpdateRecord> {
    updates.iter().map(DaemonUpdateRecord::from).collect()
}

/// Convert daemon messages into protocol records.
pub fn daemon_messages_to_records(messages: &[DaemonMessage]) -> Vec<DaemonMessageRecord> {
    messages.iter().map(DaemonMessageRecord::from).collect()
}

/// Convert diagnostics into protocol batches grouped by file id.
pub fn diagnostics_to_batches(diagnostics: &[Diagnostic]) -> Vec<DiagnosticBatch> {
    // group diagnostics by file
    let mut by_file = HashMap::new();
    for diagnostic in diagnostics {
        let file_id = diagnostic.primary_label().span.file;
        by_file
            .entry(file_id)
            .or_insert_with(Vec::new)
            .push(diagnostic.clone());
    }

    // build batches sorted by file id
    let mut batches: Vec<DiagnosticBatch> = by_file
        .into_iter()
        .map(|(file_id, diagnostics)| DiagnosticBatch {
            file_id,
            diagnostics,
        })
        .collect();
    batches.sort_by_key(|batch| batch.file_id.0);
    batches
}

/// Convert diagnostics into file images for rendering.
pub fn diagnostic_file_images(
    repository: &Repository,
    revision: Revision,
    diagnostics: &[Diagnostic],
) -> Vec<FileUpdateImage> {
    // collect unique file ids in order
    let mut seen = HashSet::new();
    let mut images = Vec::new();
    for diagnostic in diagnostics {
        let file_id = diagnostic.primary_label().span.file;
        if seen.insert(file_id)
            && let Some(image) = image_for_file(repository, revision, file_id)
        {
            images.push(image);
        }
    }

    // keep images stable by file id
    images.sort_by_key(|image| image.id.0);
    images
}

/// Build an image for a file id.
fn image_for_file(
    repository: &Repository,
    revision: Revision,
    file_id: FileId,
) -> Option<FileUpdateImage> {
    // load the file image
    let file = repository.file(revision, file_id).ok()??;

    // resolve text content when available
    let content = match file.content.payload() {
        FileContent::Text { .. } => Some(file.text().to_string()),
        FileContent::Binary { .. } => None,
    };

    Some(FileUpdateImage {
        id: file.id,
        name: file.name.clone(),
        uri: file.uri.clone(),
        path: file.path.clone(),
        file_type: file.ty,
        content,
    })
}
