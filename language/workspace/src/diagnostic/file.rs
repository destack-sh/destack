use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact::ArtifactKey;
use destack_query::Module;
use destack_repository::Revision;
use destack_serde::Reflect;
use destack_session::{ArtifactPriority, ArtifactRun};
use destack_source::{Diagnostic, File, FileId};
use serde::{Deserialize, Serialize};

use crate::Error;
use crate::workspace::{Workspace, WorkspacePin};

/// Selection for one diagnostic read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum DiagnosticsRequest {
    /// Return diagnostics for this workspace.
    All,
    /// Return diagnostics for one file.
    File(PathBuf),
}

/// Diagnostics for one file at one exact semantic revision.
#[derive(Debug, Clone)]
pub struct FileDiagnostics {
    /// The semantic revision containing these diagnostics.
    pub revision: Revision,
    /// The file used for range conversion.
    pub file: Arc<File>,
    /// The diagnostics for this file.
    pub diagnostics: Vec<Diagnostic>,
}

/// Diagnostics and failures from one completed diagnostic run.
#[derive(Debug, Default)]
pub struct DiagnosticOutcome {
    /// The completed diagnostics.
    pub diagnostics: Vec<FileDiagnostics>,
    /// The run failures.
    pub failures: Vec<Error>,
}

impl DiagnosticOutcome {
    /// Return completed diagnostics or every failure from this outcome.
    pub fn into_result(self) -> Result<Vec<FileDiagnostics>, Error> {
        let mut failures = self.failures.into_iter();
        let Some(first) = failures.next() else {
            return Ok(self.diagnostics);
        };
        let Some(second) = failures.next() else {
            return Err(first);
        };

        // retain multiple diagnostic failures in the returned error
        let mut messages = vec![first.to_string(), second.to_string()];
        messages.extend(failures.map(|failure| failure.to_string()));
        let detail = messages.join("; ");

        Err(Error::Internal { detail })
    }
}

impl FileDiagnostics {
    /// Read diagnostics for one exact file.
    fn read(
        session: &WorkspacePin,
        file_id: FileId,
        diagnostics: Vec<Diagnostic>,
    ) -> Result<Self, Error> {
        let file = session.file(file_id)?;

        Ok(Self {
            revision: session.revision(),
            file,
            diagnostics,
        })
    }
}

/// One scheduled diagnostic read at an exact workspace revision.
pub struct DiagnosticRun {
    /// The exact workspace revision selected by this request.
    revision: Revision,
    /// The scheduled read, absent when a requested file is not tracked.
    read: Option<DiagnosticRead>,
}

impl std::fmt::Debug for DiagnosticRun {
    /// Format the visible diagnostic run state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DiagnosticRun")
            .field("revision", &self.revision)
            .field("is_scheduled", &self.read.is_some())
            .finish()
    }
}

impl DiagnosticRun {
    /// Return the exact workspace revision pinned by this diagnostic run.
    pub fn revision(&self) -> Revision {
        self.revision
    }

    /// Complete the scheduled read and return its diagnostics and failures.
    pub async fn wait(self) -> DiagnosticOutcome {
        match self.read {
            Some(read) => read.wait().await,
            None => DiagnosticOutcome::default(),
        }
    }
}

/// One exact workspace diagnostic read.
struct DiagnosticRead {
    /// Pinned source and diagnostic state.
    session: WorkspacePin,
    /// Files selected from this workspace.
    selection: DiagnosticSelection,
    /// Exact diagnostic artifact roots for this workspace.
    artifact_keys: Vec<ArtifactKey>,
    /// Foreground provisioning for the diagnostic artifacts.
    artifact_run: ArtifactRun,
}

impl DiagnosticRead {
    /// Schedule one exact workspace diagnostic read.
    fn new(
        session: WorkspacePin,
        selection: DiagnosticSelection,
        modules: &[Module],
    ) -> Result<Self, Error> {
        let artifact_keys = session.diagnostic_artifacts(modules);
        let artifact_run = session.session().provide(
            session.revision(),
            &artifact_keys,
            ArtifactPriority::Foreground,
        );

        Ok(Self {
            session,
            selection,
            artifact_keys,
            artifact_run,
        })
    }

    /// Complete this workspace read and return its diagnostics and failures.
    async fn wait(self) -> DiagnosticOutcome {
        let Self {
            session,
            selection,
            artifact_keys,
            artifact_run,
        } = self;
        let revision = session.revision();
        let repository = session.repository();

        // finish every requested phase, then read all completed diagnostics
        let mut outcome = DiagnosticOutcome::default();
        if let Err(error) = artifact_run.complete().await {
            outcome.failures.push(error.into());
        }
        let diagnostics = match repository.diagnostics_for_keys(revision, &artifact_keys) {
            Ok(diagnostics) => diagnostics,
            Err(error) => {
                outcome.failures.push(error.into());

                return outcome;
            }
        };
        let mut diagnostics_by_file = diagnostics.group_by_file();

        // retain only the requested file when this is a file read
        if let DiagnosticSelection::File(file_id) = selection {
            let diagnostics = diagnostics_by_file.remove(&file_id).unwrap_or_default();
            match FileDiagnostics::read(&session, file_id, diagnostics) {
                Ok(file) => outcome.diagnostics.push(file),
                Err(error) => outcome.failures.push(error),
            }

            return outcome;
        }

        // build stable workspace diagnostics
        for (file_id, file_diagnostics) in diagnostics_by_file {
            let file = match FileDiagnostics::read(&session, file_id, file_diagnostics) {
                Ok(file) => file,
                Err(error) => {
                    outcome.failures.push(error);

                    continue;
                }
            };
            outcome.diagnostics.push(file);
        }
        outcome.diagnostics.sort_by(|left, right| {
            let paths = left.file.path.cmp(&right.file.path);
            if paths != Ordering::Equal {
                return paths;
            }

            left.file.uri.to_string().cmp(&right.file.uri.to_string())
        });

        outcome
    }
}

/// Files selected from one diagnostic request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiagnosticSelection {
    /// Every file carrying diagnostics.
    All,
    /// One exact source file.
    File(FileId),
}

impl Workspace {
    /// Schedule exact diagnostics selected by one request.
    pub fn start_diagnostics(
        &self,
        revision: Revision,
        request: DiagnosticsRequest,
    ) -> Result<DiagnosticRun, Error> {
        let session = self.pin(revision)?;
        let revision = session.revision();
        let read = match request {
            DiagnosticsRequest::All => Some(self.start_all_diagnostics(session)?),
            DiagnosticsRequest::File(path) => self.start_file_diagnostics(session, &path)?,
        };

        Ok(DiagnosticRun { revision, read })
    }

    /// Schedule exact diagnostics for one file path.
    fn start_file_diagnostics(
        &self,
        session: WorkspacePin,
        path: &Path,
    ) -> Result<Option<DiagnosticRead>, Error> {
        let revision = session.revision();
        let repository = session.repository();

        let Some(file_id) = session.file_id(path)? else {
            return Ok(None);
        };
        let module_id = repository.module_id_for_file(revision, file_id)?;
        let modules = module_id
            .map(|module_id| session.module(module_id))
            .transpose()?
            .into_iter()
            .collect::<Vec<_>>();
        self.schedule_program_indexes(&session)?;

        let read = DiagnosticRead::new(session, DiagnosticSelection::File(file_id), &modules)?;

        Ok(Some(read))
    }

    /// Schedule exact diagnostics for this workspace.
    fn start_all_diagnostics(&self, session: WorkspacePin) -> Result<DiagnosticRead, Error> {
        let revision = session.revision();
        let repository = session.repository();
        let module_ids = repository.module_ids(revision)?;
        let modules = session.selected_modules(&module_ids)?;
        self.schedule_program_indexes(&session)?;

        DiagnosticRead::new(session, DiagnosticSelection::All, &modules)
    }

    /// Schedule program indexes for one selected revision.
    fn schedule_program_indexes(&self, session: &WorkspacePin) -> Result<(), Error> {
        let artifacts = session.program_indexes()?;
        self.schedule_background(session.revision(), &artifacts)?;

        Ok(())
    }

    /// Return exact diagnostics selected by one request.
    pub async fn diagnose(
        &self,
        revision: Revision,
        request: DiagnosticsRequest,
    ) -> Result<Vec<FileDiagnostics>, Error> {
        self.start_diagnostics(revision, request)?
            .wait()
            .await
            .into_result()
    }
}
