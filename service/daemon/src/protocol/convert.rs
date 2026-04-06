use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use destack_service::{FileSnapshot as WorkspaceFileSnapshot, UpdateImpact, UpdateImpactKind};
use destack_source::{
    Diagnostic, FileContent, FileId, FileWatchEvent, FileWatchEventKind, FileWatchRescanReason,
    FileWatchStatus,
};
use destack_workspace::{Repository, Revision};

use crate::{DaemonMessage, DaemonMessageKind, DaemonUpdate, WatchBatch as DaemonWatchBatch};

use super::{
    DaemonMessageKind as ProtocolMessageKind, DaemonMessageRecord, DaemonUpdateRecord,
    DiagnosticBatch, FileSnapshot, RescanReason, UpdateImpactKind as ProtocolUpdateImpactKind,
    UpdateImpactSummary, WatchBatch as ProtocolWatchBatch, WatchEvent as ProtocolWatchEvent,
    WatchEventKind as ProtocolWatchEventKind, WatchStatus as ProtocolWatchStatus,
};

impl From<UpdateImpactKind> for ProtocolUpdateImpactKind {
    fn from(kind: UpdateImpactKind) -> Self {
        match kind {
            UpdateImpactKind::Destack => ProtocolUpdateImpactKind::Destack,
            UpdateImpactKind::TsConfig => ProtocolUpdateImpactKind::TsConfig,
            UpdateImpactKind::Unknown => ProtocolUpdateImpactKind::Unknown,
        }
    }
}

impl From<&UpdateImpact> for UpdateImpactSummary {
    fn from(plan: &UpdateImpact) -> Self {
        Self {
            file_id: plan.file_id,
            kinds: plan
                .kinds
                .iter()
                .copied()
                .map(ProtocolUpdateImpactKind::from)
                .collect(),
            modules: plan.modules.clone(),
            packages: plan.packages.clone(),
            profiles: plan.profiles.clone(),
            graphs_dropped: plan.graphs_dropped.clone(),
        }
    }
}

impl From<&DaemonUpdate> for DaemonUpdateRecord {
    fn from(update: &DaemonUpdate) -> Self {
        Self {
            module_id: update.module_id,
            file_id: update.file_id,
            file: protocol_snapshot_from_workspace(&update.file),
            impact: UpdateImpactSummary::from(&update.impact),
            diagnostics: update.diagnostics.clone(),
        }
    }
}

/// Convert a workspace snapshot into protocol shape.
fn protocol_snapshot_from_workspace(snapshot: &WorkspaceFileSnapshot) -> FileSnapshot {
    FileSnapshot {
        id: snapshot.id,
        name: snapshot.name.clone(),
        uri: snapshot.uri.clone(),
        path: snapshot.path.clone(),
        file_type: snapshot.file_type,
        content: snapshot.content.clone(),
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

impl From<&FileWatchRescanReason> for RescanReason {
    fn from(reason: &FileWatchRescanReason) -> Self {
        match reason {
            FileWatchRescanReason::Startup => RescanReason::Startup,
            FileWatchRescanReason::Overflow => RescanReason::Overflow,
            FileWatchRescanReason::Manual => RescanReason::Manual,
            FileWatchRescanReason::Update => RescanReason::Update,
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
                ProtocolWatchStatus::RescanRequested {
                    roots: roots.clone(),
                    reason: RescanReason::from(reason),
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

impl RescanReason {
    /// Convert to a daemon rescan reason.
    pub fn to_daemon(self) -> FileWatchRescanReason {
        match self {
            RescanReason::Startup => FileWatchRescanReason::Startup,
            RescanReason::Overflow => FileWatchRescanReason::Overflow,
            RescanReason::Manual => FileWatchRescanReason::Manual,
            RescanReason::Update => FileWatchRescanReason::Update,
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
            ProtocolWatchStatus::RescanRequested { roots, reason } => {
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
        by_file
            .entry(diagnostic.file_id)
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

/// Convert diagnostics into file snapshots for rendering.
pub fn diagnostic_file_snapshots(
    repository: &Repository,
    revision: Revision,
    diagnostics: &[Diagnostic],
) -> Vec<FileSnapshot> {
    // collect unique file ids in order
    let mut seen = HashSet::new();
    let mut snapshots = Vec::new();
    for diagnostic in diagnostics {
        if seen.insert(diagnostic.file_id)
            && let Some(snapshot) = snapshot_for_file(repository, revision, diagnostic.file_id)
        {
            snapshots.push(snapshot);
        }
    }

    // keep snapshots stable by file id
    snapshots.sort_by_key(|snapshot| snapshot.id.0);
    snapshots
}

/// Build a snapshot for a file id.
fn snapshot_for_file(
    repository: &Repository,
    revision: Revision,
    file_id: FileId,
) -> Option<FileSnapshot> {
    // load the file snapshot
    let file = repository.file(revision, file_id).ok()??;

    // resolve text content when available
    let content = match &file.content {
        FileContent::Text { .. } | FileContent::Json { .. } => Some(file.text().to_string()),
        FileContent::Missing | FileContent::Unloaded | FileContent::Binary { .. } => None,
    };

    Some(FileSnapshot {
        id: file.id,
        name: file.name.clone(),
        uri: file.uri.clone(),
        path: file.path.clone(),
        file_type: file.ty,
        content,
    })
}
