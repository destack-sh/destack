use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use destack_compiler::Compiler;
use destack_source::{Diagnostic, File, FileContent, FileId, ModuleId};
use destack_workspace::{InvalidationPlan, Program};
use parking_lot::Mutex;

use super::{
    FileSnapshot, LanguageServiceError, WorkspaceHandleId, WorkspaceMessage, WorkspaceMessageKind,
    WorkspaceUpdateRecord,
};

/// Per workspace root handle state.
#[derive(Debug)]
pub(super) struct WorkspaceHandle {
    /// Stable workspace handle id.
    pub(super) id: WorkspaceHandleId,
    /// Root path for this workspace handle.
    pub(super) root: PathBuf,
    /// Semantic revision for this workspace handle.
    revision: AtomicU64,
    /// Program for this root.
    pub(super) program: Arc<Program>,
    /// Compiler for this root.
    pub(super) compiler: Arc<Compiler>,
    /// Serialize compilation per root.
    pub(super) compile_lock: Mutex<()>,
}

impl WorkspaceHandle {
    /// Create a new workspace handle for a root.
    pub(super) fn new(
        id: WorkspaceHandleId,
        root: PathBuf,
        program: Arc<Program>,
        compiler: Arc<Compiler>,
    ) -> Self {
        Self {
            id,
            root,
            revision: AtomicU64::new(1),
            program,
            compiler,
            compile_lock: Mutex::new(()),
        }
    }

    /// Return the current semantic revision.
    pub(super) fn revision(&self) -> u64 {
        self.revision.load(Ordering::Relaxed)
    }

    /// Increment and return the semantic revision.
    pub(super) fn bump_revision(&self) -> u64 {
        self.revision.fetch_add(1, Ordering::Relaxed) + 1
    }
}

/// Internal update with invalidation metadata.
#[derive(Debug, Clone)]
pub(super) struct ServiceUpdate {
    /// Updated module id when known.
    pub(super) module_id: Option<ModuleId>,
    /// Updated file id.
    pub(super) file_id: FileId,
    /// Updated file snapshot.
    pub(super) file: FileSnapshot,
    /// Invalidation summary.
    pub(super) invalidation: InvalidationPlan,
    /// Diagnostics for the updated file.
    pub(super) diagnostics: Vec<Diagnostic>,
}

/// Build an internal update from program state.
pub(super) fn build_update(
    program: &Program,
    module_id: Option<ModuleId>,
    file_id: FileId,
    invalidation: InvalidationPlan,
) -> Result<ServiceUpdate, LanguageServiceError> {
    let file = file_snapshot_for_id(program, file_id)?;
    Ok(ServiceUpdate {
        module_id,
        file_id,
        file,
        invalidation,
        diagnostics: Vec::new(),
    })
}

/// Convert an internal update to a public update record.
pub(super) fn workspace_update_record(update: ServiceUpdate) -> WorkspaceUpdateRecord {
    WorkspaceUpdateRecord {
        module_id: update.module_id,
        file_id: update.file_id,
        file: update.file,
        invalidation: update.invalidation,
        diagnostics: update.diagnostics,
    }
}

/// Build a file snapshot for a program file id.
pub(super) fn file_snapshot_for_id(
    program: &Program,
    file_id: FileId,
) -> Result<FileSnapshot, LanguageServiceError> {
    let file = program
        .files
        .get_maybe(file_id)
        .ok_or(LanguageServiceError::FileIdNotTracked { file_id })?;

    Ok(file_snapshot_from_file(&file))
}

/// Build a file snapshot payload.
pub(super) fn file_snapshot_from_file(file: &File) -> FileSnapshot {
    let content = match &file.content {
        FileContent::Text { content } => Some(content.clone()),
        FileContent::Json { content, .. } => Some(content.clone()),
        FileContent::Binary { .. } => None,
        FileContent::Missing => None,
        FileContent::Unloaded => None,
    };

    FileSnapshot {
        id: file.id,
        name: file.name.clone(),
        uri: file.uri.clone(),
        path: file.path.clone(),
        file_type: file.ty,
        content,
    }
}

/// Build a workspace message.
pub(super) fn workspace_message(
    kind: WorkspaceMessageKind,
    code: &str,
    message: &str,
) -> WorkspaceMessage {
    WorkspaceMessage {
        kind,
        code: code.to_string(),
        message: message.to_string(),
    }
}

/// Build a warning message.
pub(super) fn warning_message(code: &str, message: &str) -> WorkspaceMessage {
    workspace_message(WorkspaceMessageKind::Warning, code, message)
}
