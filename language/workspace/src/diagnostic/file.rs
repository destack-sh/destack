use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact::ArtifactKey;
use destack_repository::Revision;
use destack_session::{ArtifactCancellation, ArtifactPriority, ArtifactRun};
use destack_source::{Diagnostic, File, FileId, ModuleId, Uri};

use crate::diagnostic::Error;
use crate::workspace::{LocalWorkspace, SessionPin};

/// Selection for one diagnostic read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticsRequest {
    /// Return diagnostics for every open root.
    All,
    /// Return diagnostics for one root.
    Root(PathBuf),
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
    /// The URI published to the editor.
    pub uri: Uri,
    /// The editor document version when the file is open.
    pub version: Option<i32>,
    /// The diagnostics for this file.
    pub diagnostics: Vec<Diagnostic>,
}

impl FileDiagnostics {
    /// Read diagnostics for one exact file.
    fn read(
        session: &SessionPin,
        open_files: &HashMap<FileId, (Uri, Option<i32>)>,
        file_id: FileId,
        diagnostics: Vec<Diagnostic>,
    ) -> Result<Self, Error> {
        let file = session.file(file_id)?;
        let open_file = open_files.get(&file_id);
        let uri = open_file
            .map(|(uri, _)| uri.clone())
            .or_else(|| file.path.as_ref().map(Uri::from_file_path))
            .unwrap_or_else(|| file.uri.clone());
        let version = open_file.and_then(|(_, version)| *version);

        Ok(Self {
            revision: session.revision(),
            file,
            uri,
            version,
            diagnostics,
        })
    }
}

/// One scheduled diagnostic read across exact root revisions.
pub struct DiagnosticRun {
    /// Exact root reads scheduled by this request.
    reads: Vec<DiagnosticRead>,
}

impl std::fmt::Debug for DiagnosticRun {
    /// Format the visible diagnostic run state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DiagnosticRun")
            .field("roots", &self.reads.len())
            .finish()
    }
}

impl DiagnosticRun {
    /// Return every root revision pinned by this diagnostic run.
    pub fn revisions(&self) -> Vec<(PathBuf, Revision)> {
        self.reads
            .iter()
            .map(|read| (read.root.clone(), read.session.revision()))
            .collect()
    }

    /// Return cancellation access for every root artifact run.
    pub fn cancellations(&self) -> Vec<ArtifactCancellation> {
        self.reads
            .iter()
            .map(|read| read.artifact_run.cancellation())
            .collect()
    }

    /// Complete every root and read its exact diagnostics.
    pub fn wait(self) -> Result<Vec<FileDiagnostics>, Error> {
        let mut diagnostics = Vec::new();
        for read in self.reads {
            diagnostics.extend(read.wait()?);
        }

        Ok(diagnostics)
    }
}

/// One exact root diagnostic read.
struct DiagnosticRead {
    /// Opened root containing this exact read.
    root: PathBuf,
    /// Pinned source and diagnostic state.
    session: SessionPin,
    /// Files selected from this root.
    selection: DiagnosticSelection,
    /// Open file protocol identities at the pinned revision.
    open_files: HashMap<FileId, (Uri, Option<i32>)>,
    /// Exact diagnostic artifact roots for this root.
    artifact_keys: Vec<ArtifactKey>,
    /// Foreground provisioning for the diagnostic artifacts.
    artifact_run: ArtifactRun,
}

impl DiagnosticRead {
    /// Schedule one exact root diagnostic read.
    fn new(
        root: PathBuf,
        session: SessionPin,
        selection: DiagnosticSelection,
        open_files: HashMap<FileId, (Uri, Option<i32>)>,
        modules: &[ModuleId],
    ) -> Result<Self, Error> {
        let artifact_keys = session.diagnostic_artifacts(modules)?;
        let artifact_run = session.session().schedule_artifacts(
            session.revision(),
            &artifact_keys,
            ArtifactPriority::Foreground,
        );

        Ok(Self {
            root,
            session,
            selection,
            open_files,
            artifact_keys,
            artifact_run,
        })
    }

    /// Complete this root and read its selected diagnostics.
    fn wait(self) -> Result<Vec<FileDiagnostics>, Error> {
        let Self {
            root,
            session,
            selection,
            open_files,
            artifact_keys,
            artifact_run,
        } = self;
        let revision = session.revision();
        let repository = session.repository();

        // complete and read only the selected diagnostic roots
        artifact_run.complete()?;
        let diagnostics = repository.diagnostics_for_keys(revision, &artifact_keys)?;
        let mut diagnostics_by_file = diagnostics.group_by_file();

        // include selected open files even when they have no diagnostics
        for file_id in open_files.keys() {
            diagnostics_by_file.entry(*file_id).or_default();
        }

        // retain only the requested file when this is a file read
        if let DiagnosticSelection::File(file_id) = selection {
            let diagnostics = diagnostics_by_file.remove(&file_id).unwrap_or_default();
            let file = FileDiagnostics::read(&session, &open_files, file_id, diagnostics)?;

            return Ok(vec![file]);
        }

        // build stable root diagnostics
        let mut diagnostics = Vec::new();
        for (file_id, file_diagnostics) in diagnostics_by_file {
            let file = FileDiagnostics::read(&session, &open_files, file_id, file_diagnostics)?;
            let belongs_to_root = file
                .file
                .path
                .as_deref()
                .is_some_and(|path| path.starts_with(&root));
            if belongs_to_root || open_files.contains_key(&file_id) {
                diagnostics.push(file);
            }
        }
        diagnostics.sort_by(|left, right| {
            let paths = left.file.path.cmp(&right.file.path);
            if paths != Ordering::Equal {
                return paths;
            }

            left.uri.to_string().cmp(&right.uri.to_string())
        });

        Ok(diagnostics)
    }
}

/// Files selected from one diagnostic root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiagnosticSelection {
    /// Every file carrying diagnostics or open editor state.
    Root,
    /// One exact source file.
    File(FileId),
}

impl LocalWorkspace {
    /// Schedule exact diagnostics selected by one request.
    pub fn start_diagnostics(&self, request: DiagnosticsRequest) -> Result<DiagnosticRun, Error> {
        let reads = match request {
            DiagnosticsRequest::All => {
                let mut roots = self.root_paths();
                roots.sort();

                roots
                    .into_iter()
                    .map(|root| self.start_root_diagnostics(&root))
                    .collect::<Result<Vec<_>, _>>()?
            }
            DiagnosticsRequest::Root(root) => vec![self.start_root_diagnostics(&root)?],
            DiagnosticsRequest::File(path) => {
                self.start_file_diagnostics(&path)?.into_iter().collect()
            }
        };

        Ok(DiagnosticRun { reads })
    }

    /// Schedule exact diagnostics for one file path.
    fn start_file_diagnostics(&self, path: &Path) -> Result<Option<DiagnosticRead>, Error> {
        let root = self.root_at(path)?;
        let session = self.pin_session(&root)?;
        let revision = session.revision();
        let repository = session.repository();

        let Some(file_id) = session.file_id(path)? else {
            return Ok(None);
        };
        let file = session.file(file_id)?;
        let module = repository.module_id_for_file(revision, file_id)?;
        let modules = module.into_iter().collect::<Vec<_>>();
        self.schedule_program_indexes(&root, &session)?;

        // retain open protocol identity only when it matches this revision
        let mut open_files = HashMap::new();
        if let Some(path) = file.path.as_deref()
            && let Some(open_file) = self.open_state(path)
        {
            let version =
                self.open_file_version_in_revision(repository, revision, file_id, path)?;
            open_files.insert(file_id, (open_file.uri, version));
        }

        let read = DiagnosticRead::new(
            root,
            session,
            DiagnosticSelection::File(file_id),
            open_files,
            &modules,
        )?;

        Ok(Some(read))
    }

    /// Schedule exact diagnostics for one root.
    fn start_root_diagnostics(&self, root: &Path) -> Result<DiagnosticRead, Error> {
        let session = self.pin_session(root)?;
        let revision = session.revision();
        let repository = session.repository();
        let modules = repository.module_ids(revision)?;
        self.schedule_program_indexes(root, &session)?;

        // retain open protocol identities that match this revision
        let mut open_files = HashMap::new();
        for (path, file) in self.open_files_under(root) {
            let Some(file_id) = session.file_id(&path)? else {
                return Err(Error::FileMissing { path });
            };
            let version =
                self.open_file_version_in_revision(repository, revision, file_id, &path)?;
            open_files.insert(file_id, (file.uri, version));
        }

        DiagnosticRead::new(
            root.to_path_buf(),
            session,
            DiagnosticSelection::Root,
            open_files,
            &modules,
        )
    }

    /// Schedule program indexes for one selected revision.
    fn schedule_program_indexes(&self, root: &Path, session: &SessionPin) -> Result<(), Error> {
        let root = self.workspace_root(root)?;
        let artifacts = session.program_indexes()?;
        root.schedule_background(session.revision(), &artifacts);

        Ok(())
    }

    /// Return exact diagnostics selected by one request.
    pub fn diagnose(&self, request: DiagnosticsRequest) -> Result<Vec<FileDiagnostics>, Error> {
        self.start_diagnostics(request)?.wait()
    }
}
