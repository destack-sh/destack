use std::cmp::Ordering;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tspp_artifact::ArtifactKey;
use tspp_query::Module;
use tspp_repository::Revision;
use tspp_serde::Reflect;
use tspp_session::{ArtifactPriority, ArtifactRun, ArtifactRunId};
use tspp_source::{Diagnostic, File, FileId};

use crate::Error;
use crate::workspace::{Workspace, WorkspacePin};

/// Diagnostics selected from one workspace.
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
    /// Create diagnostics for one exact file.
    fn new(
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

/// One scheduled diagnostic run at an exact workspace revision.
pub struct DiagnosticRun {
    /// Pinned workspace revision.
    session: WorkspacePin,
    /// Files selected from this workspace.
    selection: DiagnosticSelection,
    /// Exact diagnostic artifact roots.
    artifact_keys: Vec<ArtifactKey>,
    /// Artifact run providing the roots.
    artifact_run: ArtifactRun,
}

impl std::fmt::Debug for DiagnosticRun {
    /// Format the visible diagnostic run state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DiagnosticRun")
            .field("revision", &self.session.revision())
            .field("artifact_run", &self.artifact_run.id())
            .finish()
    }
}

impl DiagnosticRun {
    /// Schedule diagnostics for selected modules.
    fn new(
        session: WorkspacePin,
        selection: DiagnosticSelection,
        modules: &[Module],
        priority: ArtifactPriority,
    ) -> Self {
        let artifact_keys = session.diagnostic_artifacts(modules);
        let artifact_run = session
            .session()
            .provide(session.revision(), &artifact_keys, priority);

        Self {
            session,
            selection,
            artifact_keys,
            artifact_run,
        }
    }

    /// Return the exact workspace revision pinned by this diagnostic run.
    pub fn revision(&self) -> Revision {
        self.session.revision()
    }

    /// Return the artifact run providing diagnostics.
    pub fn artifact_run_id(&self) -> ArtifactRunId {
        self.artifact_run.id()
    }

    /// Complete this run and return its diagnostics and failures.
    pub async fn wait(self) -> DiagnosticOutcome {
        let Self {
            session,
            selection,
            artifact_keys,
            artifact_run,
        } = self;
        let revision = session.revision();
        let repository = session.repository();

        // complete every requested artifact
        let mut outcome = DiagnosticOutcome::default();
        if let Err(error) = artifact_run.complete().await {
            outcome.failures.push(error.into());
        }
        session.persist_artifacts();

        // read every completed diagnostic
        let diagnostics = match repository.diagnostics_for_keys(revision, &artifact_keys) {
            Ok(diagnostics) => diagnostics,
            Err(error) => {
                outcome.failures.push(error.into());

                return outcome;
            }
        };
        let mut diagnostics_by_file = diagnostics.group_by_file();

        // retain only the requested file
        if let DiagnosticSelection::File(file_id) = selection {
            let diagnostics = diagnostics_by_file.remove(&file_id).unwrap_or_default();
            match FileDiagnostics::new(&session, file_id, diagnostics) {
                Ok(file) => outcome.diagnostics.push(file),
                Err(error) => outcome.failures.push(error),
            }

            return outcome;
        }

        // build stable workspace diagnostics
        for (file_id, file_diagnostics) in diagnostics_by_file {
            let file = match FileDiagnostics::new(&session, file_id, file_diagnostics) {
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
    /// Schedule exact diagnostics for this workspace.
    pub fn start_diagnostics(
        &self,
        revision: Revision,
        priority: ArtifactPriority,
    ) -> Result<DiagnosticRun, Error> {
        let session = self.pin(revision)?;
        let repository = session.repository();
        let module_ids = repository.module_ids(session.revision())?;
        let modules = session.selected_modules(&module_ids)?;

        Ok(DiagnosticRun::new(
            session,
            DiagnosticSelection::All,
            &modules,
            priority,
        ))
    }

    /// Schedule exact diagnostics for one source file.
    pub fn start_file_diagnostics(
        &self,
        revision: Revision,
        file_id: FileId,
        priority: ArtifactPriority,
    ) -> Result<DiagnosticRun, Error> {
        // open the exact workspace revision
        let session = self.pin(revision)?;
        let revision = session.revision();
        let repository = session.repository();

        // select the source module when the file belongs to one
        let module_id = repository.module_id_for_file(revision, file_id)?;
        let modules = module_id
            .map(|module_id| session.module(module_id))
            .transpose()?
            .into_iter()
            .collect::<Vec<_>>();

        // schedule diagnostics for the selected file and module
        let run = DiagnosticRun::new(
            session,
            DiagnosticSelection::File(file_id),
            &modules,
            priority,
        );

        Ok(run)
    }

    /// Return exact diagnostics selected by one request.
    pub async fn diagnose(
        &self,
        revision: Revision,
        request: DiagnosticsRequest,
    ) -> Result<Vec<FileDiagnostics>, Error> {
        let run = match request {
            DiagnosticsRequest::All => {
                self.start_diagnostics(revision, ArtifactPriority::Foreground)?
            }
            DiagnosticsRequest::File(path) => {
                let session = self.pin(revision)?;
                let Some(file_id) = session.file_id(&path)? else {
                    return Ok(Vec::new());
                };

                self.start_file_diagnostics(revision, file_id, ArtifactPriority::Foreground)?
            }
        };

        run.wait().await.into_result()
    }
}
