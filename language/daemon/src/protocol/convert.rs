use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use destack_repository::{Repository, Revision};
use destack_source::{
    Diagnostic, FileContent, FileId, FileWatchEvent, FileWatchEventKind, FileWatchRescanReason,
    FileWatchStatus,
};
use destack_workspace::{FileImage, FileUpdate, Message, MessageKind, UpdateKind};

use super::{
    DaemonMessageRecord, DaemonUpdateRecord, DiagnosticBatch, FileUpdateImage, ReloadReason,
    UpdateChange, UpdateChangeKind, WatchEvent, WatchEventKind, WatchStartOptions, WatchStatus,
};

impl From<UpdateKind> for UpdateChangeKind {
    fn from(kind: UpdateKind) -> Self {
        match kind {
            UpdateKind::Config => Self::Destack,
            UpdateKind::Source => Self::Unknown,
        }
    }
}

impl From<&FileUpdate> for UpdateChange {
    fn from(update: &FileUpdate) -> Self {
        Self {
            file_id: update.file_id,
            kind: UpdateChangeKind::from(update.kind),
        }
    }
}

impl From<&FileUpdate> for DaemonUpdateRecord {
    fn from(update: &FileUpdate) -> Self {
        Self {
            module_id: update.module_id,
            file_id: update.file_id,
            file: update.file.as_ref().map(protocol_image_from_root),
            change: UpdateChange::from(update),
            diagnostics: update.diagnostics.clone(),
        }
    }
}

/// Convert a root image into protocol shape.
fn protocol_image_from_root(image: &FileImage) -> FileUpdateImage {
    FileUpdateImage {
        id: image.id,
        name: image.name.clone(),
        uri: image.uri.clone(),
        path: image.path.clone(),
        file_type: image.file_type,
        content: image.content.clone(),
    }
}

impl From<&Message> for DaemonMessageRecord {
    fn from(message: &Message) -> Self {
        let kind = match message.kind {
            MessageKind::Info => super::DaemonMessageKind::Info,
            MessageKind::Warning => super::DaemonMessageKind::Warning,
            MessageKind::Error => super::DaemonMessageKind::Error,
        };

        Self {
            kind,
            code: message.code.clone(),
            message: message.message.clone(),
            path: None,
        }
    }
}

impl From<&FileWatchEventKind> for WatchEventKind {
    fn from(kind: &FileWatchEventKind) -> Self {
        match kind {
            FileWatchEventKind::Created => Self::Created,
            FileWatchEventKind::Modified => Self::Modified,
            FileWatchEventKind::Deleted => Self::Deleted,
            FileWatchEventKind::Renamed => Self::Renamed,
            FileWatchEventKind::Overflow => Self::Overflow,
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

impl From<&FileWatchEvent> for WatchEvent {
    fn from(event: &FileWatchEvent) -> Self {
        Self {
            path: event.path.clone(),
            previous_path: event.previous_path.clone(),
            kind: WatchEventKind::from(&event.kind),
        }
    }
}

impl From<&FileWatchStatus> for WatchStatus {
    fn from(status: &FileWatchStatus) -> Self {
        match status {
            FileWatchStatus::Ready { roots } => Self::Ready {
                roots: roots.clone(),
            },
            FileWatchStatus::RescanRequested { roots, reason } => Self::ReloadRequested {
                roots: roots.clone(),
                reason: ReloadReason::from(reason),
            },
            FileWatchStatus::Error { message } => Self::Error {
                message: message.clone(),
            },
            FileWatchStatus::Stopped => Self::Stopped,
        }
    }
}

impl From<&super::WatchBatch> for crate::WatchBatch {
    fn from(batch: &super::WatchBatch) -> Self {
        // derive the relative duration from the protocol payload
        let started_at = Instant::now();
        let duration = batch.ended_at_ns.saturating_sub(batch.started_at_ns);
        let ended_at = started_at + Duration::from_nanos(duration);

        Self {
            events: batch.events.iter().map(WatchEvent::to_daemon).collect(),
            status: batch.status.iter().map(WatchStatus::to_daemon).collect(),
            started_at,
            ended_at,
            overflowed: batch.overflowed,
        }
    }
}

impl From<&crate::WatchBatch> for super::WatchBatch {
    fn from(batch: &crate::WatchBatch) -> Self {
        let duration = batch.duration();
        Self {
            events: batch.events.iter().map(WatchEvent::from).collect(),
            status: batch.status.iter().map(WatchStatus::from).collect(),
            overflowed: batch.overflowed,
            started_at_ns: 0,
            ended_at_ns: duration.as_nanos() as u64,
        }
    }
}

impl From<&WatchStartOptions> for crate::WatchPolicy {
    fn from(options: &WatchStartOptions) -> Self {
        Self {
            coalesce_window: Duration::from_millis(options.coalesce_window_ms),
            max_batch_size: options.max_batch_size,
        }
    }
}

impl From<&crate::WatchPolicy> for WatchStartOptions {
    fn from(policy: &crate::WatchPolicy) -> Self {
        Self {
            coalesce_window_ms: policy.coalesce_window.as_millis() as u64,
            max_batch_size: policy.max_batch_size,
        }
    }
}

impl WatchEventKind {
    /// Convert to a daemon watch event kind.
    pub fn to_daemon(self) -> FileWatchEventKind {
        match self {
            Self::Created => FileWatchEventKind::Created,
            Self::Modified => FileWatchEventKind::Modified,
            Self::Deleted => FileWatchEventKind::Deleted,
            Self::Renamed => FileWatchEventKind::Renamed,
            Self::Overflow => FileWatchEventKind::Overflow,
        }
    }
}

impl WatchEvent {
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

impl WatchStatus {
    /// Convert to a daemon watch status.
    pub fn to_daemon(&self) -> FileWatchStatus {
        match self {
            Self::Ready { roots } => FileWatchStatus::Ready {
                roots: roots.clone(),
            },
            Self::ReloadRequested { roots, reason } => FileWatchStatus::RescanRequested {
                roots: roots.clone(),
                reason: reason.to_daemon(),
            },
            Self::Error { message } => FileWatchStatus::Error {
                message: message.clone(),
            },
            Self::Stopped => FileWatchStatus::Stopped,
        }
    }
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
            && let Some(image) = file_image(repository, revision, file_id)
        {
            images.push(image);
        }
    }

    // keep images stable by file id
    images.sort_by_key(|image| image.id.0);
    images
}

/// Build an image for a file id.
fn file_image(
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
