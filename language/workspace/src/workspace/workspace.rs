use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact::{ArtifactPayload, ArtifactReference};
use destack_repository::Revision;
use destack_source::{Content, ContentId, File, FileId, TextRange};
use futures::future::BoxFuture;

use super::{RunQueryInput, RunQueryResponse};
use crate::diagnostic::{DiagnosticsRequest, Error, FileDiagnostics};
use crate::file::{Commit, FileOperation, SourceUpdate};
use crate::watch::Watch;
use crate::{
    BenchInput, BenchOutput, BuildInput, BuildOutput, CacheInput, CacheOutput, CheckInput,
    CheckOutput, CleanInput, CleanOutput, CommandError, CommandProgress, DocInput, DocOutput,
    DoctorInput, DoctorOutput, ExportInput, ExportResult, FileEdit, FormatInput, FormatOutput,
    InfoInput, InfoOutput, QueryFile, QueryInput, QueryOutput, RewriteInput, RewriteOutput,
    RunInput, RunOutput, SettingsInput, SettingsOutput, TargetsInput, TargetsOutput, TaskInput,
    TaskOutput, TestInput, TestOutput,
};

/// Operations over one live Destack workspace.
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
    fn reload(&self, root: &Path) -> Result<Option<Commit>, Error>;

    /// Apply one file operation through the workspace.
    fn file(&self, root: &Path, operation: FileOperation) -> Result<Option<Commit>, Error>;

    /// Return whether one file is currently open through the workspace.
    fn is_file_open(&self, root: &Path, path: &Path) -> Result<bool, Error>;

    /// Apply one atomic source edit through the workspace.
    fn edit(&self, root: &Path, update: SourceUpdate) -> Result<Commit, Error>;

    /// Format one source file or selected text range.
    fn format_file(
        &self,
        root: &Path,
        path: PathBuf,
        range: Option<TextRange>,
    ) -> Result<Option<FileEdit>, Error>;

    /// Read source files from one exact revision.
    fn read_files(
        &self,
        root: &Path,
        revision: Revision,
        file_ids: Vec<FileId>,
    ) -> Result<Vec<Arc<File>>, Error>;

    // ================================================================================
    // Command
    // ================================================================================

    /// Check source state for a root.
    fn check<'a>(
        &'a self,
        root: &'a Path,
        input: CheckInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<CheckOutput, CommandError>>;

    /// Format source files or content.
    fn format<'a>(
        &'a self,
        root: &'a Path,
        input: FormatInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<FormatOutput, CommandError>>;

    /// Query source files with one structural pattern.
    fn query<'a>(
        &'a self,
        root: &'a Path,
        input: QueryInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<QueryOutput, CommandError>>;

    /// Rewrite source files with one structural pattern.
    fn rewrite<'a>(
        &'a self,
        root: &'a Path,
        input: RewriteInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<RewriteOutput, CommandError>>;

    /// Build target artifacts for a root.
    fn build<'a>(
        &'a self,
        root: &'a Path,
        input: BuildInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<BuildOutput, CommandError>>;

    /// Run a workspace target.
    fn run<'a>(
        &'a self,
        root: &'a Path,
        input: RunInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<RunOutput, CommandError>>;

    /// Run workspace tests.
    fn test<'a>(
        &'a self,
        root: &'a Path,
        input: TestInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<TestOutput, CommandError>>;

    /// Generate documentation.
    fn doc<'a>(
        &'a self,
        root: &'a Path,
        input: DocInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<DocOutput, CommandError>>;

    /// Run benchmarks.
    fn bench<'a>(
        &'a self,
        root: &'a Path,
        input: BenchInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<BenchOutput, CommandError>>;

    /// Return workspace information.
    fn info<'a>(
        &'a self,
        root: &'a Path,
        input: InfoInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<InfoOutput, CommandError>>;

    /// Return configured targets.
    fn targets<'a>(
        &'a self,
        root: &'a Path,
        input: TargetsInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<TargetsOutput, CommandError>>;

    /// Return cache locations.
    fn cache<'a>(
        &'a self,
        root: &'a Path,
        input: CacheInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<CacheOutput, CommandError>>;

    /// Return resolved settings.
    fn settings<'a>(
        &'a self,
        root: &'a Path,
        input: SettingsInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<SettingsOutput, CommandError>>;

    /// Return workspace health information.
    fn doctor<'a>(
        &'a self,
        root: &'a Path,
        input: DoctorInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<DoctorOutput, CommandError>>;

    /// Run workspace tasks.
    fn task<'a>(
        &'a self,
        root: &'a Path,
        input: TaskInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<TaskOutput, CommandError>>;

    /// Clean generated state.
    fn clean<'a>(
        &'a self,
        root: &'a Path,
        input: CleanInput,
        progress: Option<CommandProgress>,
    ) -> BoxFuture<'a, Result<CleanOutput, CommandError>>;

    // ================================================================================
    // Query
    // ================================================================================

    /// Run one semantic query for a root.
    fn run_query<'a>(
        &'a self,
        root: &'a Path,
        request: RunQueryInput,
    ) -> BoxFuture<'a, Result<RunQueryResponse, Error>>;

    /// Resolve one source file for semantic queries.
    fn resolve_query_file(&self, root: &Path, path: PathBuf) -> Result<Option<QueryFile>, Error>;

    // ================================================================================
    // Diagnostics
    // ================================================================================

    /// Return exact file diagnostics.
    fn diagnose(
        &self,
        request: DiagnosticsRequest,
    ) -> BoxFuture<'_, Result<Vec<FileDiagnostics>, Error>>;

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
    fn export(&self, root: &Path, request: ExportInput) -> Result<ExportResult, Error>;

    // ================================================================================
    // Watch
    // ================================================================================

    /// Watch semantic changes for one workspace root.
    fn watch(&self, root: &Path) -> Result<Watch, Error>;
}
