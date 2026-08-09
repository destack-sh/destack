use destack_artifact::ArtifactPayload;
use destack_repository::{Commit, Revision};
use destack_source::{Content, ContentId};

use super::*;
use crate::{
    BenchOutput, BuildOutput, CacheOutput, CheckOutput, CleanOutput, DocOutput, DoctorOutput,
    ExportResult, FileImage, FormatOutput, InfoOutput, ProgressEvent, QueryOutput, RewriteOutput,
    RunQueryResponse, SettingsOutput, TargetsOutput, TaskOutput, TestOutput, WatchEvent,
};

/// RPC operations over one Destack workspace.
#[destack_rpc::service(name = "destack.workspace.Workspace")]
pub trait WorkspaceService {
    /// Reload one workspace root from its host.
    #[rpc(name = "Reload", idempotency = "idempotent")]
    fn reload(request: ReloadRequest) -> Option<Commit>;

    /// Read one workspace root revision.
    #[rpc(name = "ReadRevision", idempotency = "no_side_effects")]
    fn read_revision(request: ReadRevisionRequest) -> Revision;

    /// Apply one editor file operation.
    #[rpc(name = "ApplyFileOperation")]
    fn apply_file_operation(request: ApplyFileOperationRequest) -> Option<Commit>;

    /// Apply one atomic source update.
    #[rpc(name = "ApplySourceUpdate")]
    fn apply_source_update(request: ApplySourceUpdateRequest) -> Commit;

    /// Return whether one source file is open.
    #[rpc(name = "IsFileOpen", idempotency = "no_side_effects")]
    fn is_file_open(request: IsFileOpenRequest) -> bool;

    /// Format one source file or selected range.
    #[rpc(name = "FormatFile", idempotency = "no_side_effects")]
    fn format_file(request: FormatFileRequest) -> Option<FileEditResponse>;

    /// Read source files from one exact revision.
    #[rpc(name = "ReadFiles", idempotency = "no_side_effects")]
    fn read_files(request: ReadFilesRequest) -> Vec<FileImage>;

    /// Check source state.
    #[rpc(
        name = "Check",
        response_stream = ProgressEvent,
        idempotency = "no_side_effects"
    )]
    fn check(request: CheckRequest) -> CheckOutput;

    /// Format source files or content.
    #[rpc(name = "Format", response_stream = ProgressEvent)]
    fn format(request: FormatRequest) -> FormatOutput;

    /// Query source files with one structural pattern.
    #[rpc(
        name = "Query",
        response_stream = ProgressEvent,
        idempotency = "no_side_effects"
    )]
    fn query(request: QueryRequest) -> QueryOutput;

    /// Rewrite source files with one structural pattern.
    #[rpc(name = "Rewrite", response_stream = ProgressEvent)]
    fn rewrite(request: RewriteRequest) -> RewriteOutput;

    /// Build target artifacts.
    #[rpc(name = "Build", response_stream = ProgressEvent)]
    fn build(request: BuildRequest) -> BuildOutput;

    /// Run workspace tests.
    #[rpc(name = "Test", response_stream = ProgressEvent)]
    fn test(request: TestRequest) -> TestOutput;

    /// Generate workspace documentation.
    #[rpc(name = "Doc", response_stream = ProgressEvent)]
    fn doc(request: DocRequest) -> DocOutput;

    /// Run workspace benchmarks.
    #[rpc(name = "Bench", response_stream = ProgressEvent)]
    fn bench(request: BenchRequest) -> BenchOutput;

    /// Return workspace information.
    #[rpc(
        name = "Info",
        response_stream = ProgressEvent,
        idempotency = "no_side_effects"
    )]
    fn info(request: InfoRequest) -> InfoOutput;

    /// Return configured targets.
    #[rpc(
        name = "Targets",
        response_stream = ProgressEvent,
        idempotency = "no_side_effects"
    )]
    fn targets(request: TargetsRequest) -> TargetsOutput;

    /// Return cache locations.
    #[rpc(
        name = "Cache",
        response_stream = ProgressEvent,
        idempotency = "no_side_effects"
    )]
    fn cache(request: CacheRequest) -> CacheOutput;

    /// Return resolved settings.
    #[rpc(
        name = "Settings",
        response_stream = ProgressEvent,
        idempotency = "no_side_effects"
    )]
    fn settings(request: SettingsRequest) -> SettingsOutput;

    /// Diagnose workspace configuration and state.
    #[rpc(
        name = "Doctor",
        response_stream = ProgressEvent,
        idempotency = "no_side_effects"
    )]
    fn doctor(request: DoctorRequest) -> DoctorOutput;

    /// Execute configured workspace tasks.
    #[rpc(name = "Task", response_stream = ProgressEvent)]
    fn task(request: TaskRequest) -> TaskOutput;

    /// Clean generated workspace state.
    #[rpc(name = "Clean", response_stream = ProgressEvent)]
    fn clean(request: CleanRequest) -> CleanOutput;

    /// Read one exact artifact payload.
    #[rpc(name = "Artifact", idempotency = "no_side_effects")]
    fn artifact(request: ArtifactRequest) -> ArtifactPayload;

    /// Store one content value.
    #[rpc(name = "Store", idempotency = "idempotent")]
    fn store(request: StoreRequest) -> ContentId;

    /// Load one content value.
    #[rpc(name = "Load", idempotency = "no_side_effects")]
    fn load(request: LoadRequest) -> Content;

    /// Materialize one artifact on the workspace host.
    #[rpc(name = "Export")]
    fn export(request: ExportRequest) -> ExportResult;

    /// Read exact diagnostics.
    #[rpc(name = "Diagnose", idempotency = "no_side_effects")]
    fn diagnose(request: DiagnoseRequest) -> Vec<FileDiagnosticsResponse>;

    /// Resolve one source file for semantic queries.
    #[rpc(name = "ResolveQueryFile", idempotency = "no_side_effects")]
    fn resolve_query_file(request: ResolveQueryFileRequest) -> Option<QueryFileResponse>;

    /// Execute one semantic query.
    #[rpc(name = "RunQuery", idempotency = "no_side_effects")]
    fn run_query(request: RunQueryRequest) -> RunQueryResponse;

    /// Watch one workspace root until cancellation.
    #[rpc(name = "Watch", response_stream = WatchEvent)]
    fn watch(request: WatchRequest) -> ();
}
