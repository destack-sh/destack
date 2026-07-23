use std::path::{Path, PathBuf};

use destack_artifact::{ArtifactPayload, ArtifactReference};
use destack_repository::Revision;
use destack_source::{Content, ContentId};

use super::{
    DiagnosticsRequest, QueryResult, ReloadRequest, ViewRequest, ViewResult, WorkspaceQueryRequest,
};
use crate::diagnostic::{DiagnosticView, Error};
use crate::file::{Commit, FileOperation, SourceUpdate};
use crate::protocol::WatchPolicy;
use crate::watch::WatchUpdate;
use crate::{
    BenchInput, BenchOutput, BuildInput, BuildOutput, CacheInput, CacheOutput, CheckInput,
    CheckOutput, CleanInput, CleanOutput, CommandError, CommandProgress, DocInput, DocOutput,
    DoctorInput, DoctorOutput, ExportRequest, ExportResult, FormatInput, FormatOutput, InfoInput,
    InfoOutput, RunInput, RunOutput, SettingsInput, SettingsOutput, TargetsInput, TargetsOutput,
    TaskInput, TaskOutput, TestInput, TestOutput, UpdateBatch,
};

/// Workspace operations shared by local and remote workspace implementations.
pub trait Workspace: std::fmt::Debug + Send + Sync {
    // ================================================================================
    // Roots
    // ================================================================================

    /// Return the workspace home path.
    fn home(&self) -> &Path;

    /// Return one canonical path according to the workspace host.
    fn canonicalize(&self, path: &Path) -> Result<PathBuf, Error>;

    /// Return opened workspace roots.
    fn roots(&self) -> Vec<PathBuf>;

    /// Open one root.
    fn open(&self, root: PathBuf) -> Result<(), Error>;

    /// Close one root.
    fn close(&self, root: &Path) -> Result<(), Error>;

    /// Return the open root that owns one path.
    fn root(&self, path: &Path) -> Result<PathBuf, Error>;

    /// Return the current revision for a root.
    fn revision(&self, root: &Path) -> Result<Revision, Error>;

    // ================================================================================
    // Source
    // ================================================================================

    /// Reload source state from the workspace host.
    fn reload(&self, request: ReloadRequest) -> Result<UpdateBatch, Error>;

    /// Apply one file operation through the workspace.
    fn file(&self, operation: FileOperation) -> Result<UpdateBatch, Error>;

    /// Return whether one file is currently open through the workspace.
    fn is_file_open(&self, path: &Path) -> Result<bool, Error>;

    /// Apply one atomic source edit through the workspace.
    fn edit(&self, root: &Path, update: SourceUpdate) -> Result<Commit, Error>;

    // ================================================================================
    // Command
    // ================================================================================

    /// Check source state for a root.
    fn check(
        &self,
        root: &Path,
        input: CheckInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<CheckOutput, CommandError>;

    /// Format source files or content.
    fn format(
        &self,
        root: &Path,
        input: FormatInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<FormatOutput, CommandError>;

    /// Build target artifacts for a root.
    fn build(
        &self,
        root: &Path,
        input: BuildInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<BuildOutput, CommandError>;

    /// Run a workspace target.
    fn run(
        &self,
        root: &Path,
        input: RunInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<RunOutput, CommandError>;

    /// Run workspace tests.
    fn test(
        &self,
        root: &Path,
        input: TestInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<TestOutput, CommandError>;

    /// Generate documentation.
    fn doc(
        &self,
        root: &Path,
        input: DocInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<DocOutput, CommandError>;

    /// Run benchmarks.
    fn bench(
        &self,
        root: &Path,
        input: BenchInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<BenchOutput, CommandError>;

    /// Return workspace information.
    fn info(
        &self,
        root: &Path,
        input: InfoInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<InfoOutput, CommandError>;

    /// Return configured targets.
    fn targets(
        &self,
        root: &Path,
        input: TargetsInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<TargetsOutput, CommandError>;

    /// Return cache locations.
    fn cache(
        &self,
        root: &Path,
        input: CacheInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<CacheOutput, CommandError>;

    /// Return resolved settings.
    fn settings(
        &self,
        root: &Path,
        input: SettingsInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<SettingsOutput, CommandError>;

    /// Return workspace health information.
    fn doctor(
        &self,
        root: &Path,
        input: DoctorInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<DoctorOutput, CommandError>;

    /// Run workspace tasks.
    fn task(
        &self,
        root: &Path,
        input: TaskInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<TaskOutput, CommandError>;

    /// Clean generated state.
    fn clean(
        &self,
        root: &Path,
        input: CleanInput,
        progress: Option<CommandProgress<'_>>,
    ) -> Result<CleanOutput, CommandError>;

    // ================================================================================
    // Query
    // ================================================================================

    /// Run one semantic query for a root.
    fn query(&self, root: &Path, request: WorkspaceQueryRequest) -> Result<QueryResult, Error>;

    /// Return one workspace view.
    fn view(&self, root: &Path, request: ViewRequest) -> Result<ViewResult, Error>;

    /// Return current diagnostic views.
    fn diagnostics(&self, request: DiagnosticsRequest) -> Result<Vec<DiagnosticView>, Error>;

    // ================================================================================
    // Artifact
    // ================================================================================

    /// Return one artifact payload.
    fn artifact(&self, root: &Path, artifact: ArtifactReference) -> Result<ArtifactPayload, Error>;

    /// Store one content payload in the workspace content store.
    fn store(&self, content: Content) -> Result<ContentId, Error>;

    /// Return one content payload from the workspace content store.
    fn load(&self, content: ContentId) -> Result<Content, Error>;

    /// Materialize derived outputs on the workspace host.
    fn export(&self, root: &Path, request: ExportRequest) -> Result<ExportResult, Error>;

    // ================================================================================
    // Watch
    // ================================================================================

    /// Watch workspace files.
    fn watch(&self, roots: Vec<PathBuf>, policy: WatchPolicy) -> Result<(), Error>;

    /// Return the next watch update.
    fn next_watch(&self, root: &Path) -> Result<Option<WatchUpdate>, Error>;

    /// Stop watching workspace files.
    fn unwatch(&self, root: &Path) -> Result<(), Error>;
}
