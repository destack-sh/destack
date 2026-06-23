# generated bridge target, do not edit

from __future__ import annotations

from typing import TypeVar

from destack.protocol.connection import Connection
from ..query.model import *
from ..request import *
from ..response import *
from ..root import *
from ..watch import *
from .command.bench import BenchInput
from .command.build import BuildInput
from .command.cache import CacheInput
from .command.check import CheckInput, LintInput
from .command.clean import CleanInput
from .command.doc import DocInput
from .command.doctor import DoctorInput
from .command.format import FormatInput
from .command.info import InfoInput
from .command.output import *
from .command.run import RunInput
from .command.settings import SettingsInput
from .command.targets import TargetsInput
from .command.task import TaskInput
from .command.test import TestInput
from .artifact.export import ExportRequest
from .file.image import FileOperation
from ..artifact.reference import ArtifactReference
from .file.update import SourceUpdate
from ..source.file.model.file import Content, ContentId

T = TypeVar("T")


class WorkspaceClient:
    """Exact workspace protocol client for one opened root."""

    def __init__(self, connection: Connection, handle: RootId) -> None:
        self._connection = connection
        self._handle = handle

    def connection(self) -> Connection:
        """Return the backing protocol connection."""

        return self._connection

    def handle(self) -> RootId:
        """Return the opened root handle."""

        return self._handle

    def close_root(self) -> RootClosedResponse:
        """Close a root handle."""

        request = WorkspaceRequestCloseRoot(
            close_root=CloseRootRequest(handle=self._handle)
        )
        response = self._connection.request(request)

        return expect_response(response, WorkspaceResponseRootClosed).root_closed

    def reload_root(self, reason: ReloadReason) -> RootReloadResponse:
        """Reload a root."""

        request = WorkspaceRequestReloadRoot(
            reload_root=ReloadRootRequest(handle=self._handle, reason=reason)
        )
        response = self._connection.request(request)

        return expect_response(response, WorkspaceResponseRootReloaded).root_reloaded

    def apply_file_operation(self, operation: FileOperation) -> FileOperationResponse:
        """Apply a file operation to a root."""

        request = WorkspaceRequestApplyFileOperation(
            apply_file_operation=FileOperationRequest(
                handle=self._handle, operation=operation
            )
        )
        response = self._connection.request(request)

        return expect_response(
            response, WorkspaceResponseFileOperationApplied
        ).file_operation_applied

    def apply_source_update(self, update: SourceUpdate) -> SourceUpdateResponse:
        """Apply a source update to a root."""

        request = WorkspaceRequestApplySourceUpdate(
            apply_source_update=SourceUpdateRequest(handle=self._handle, update=update)
        )
        response = self._connection.request(request)

        return expect_response(response, WorkspaceResponseSourceUpdated).source_updated

    def start_watch(
        self, roots: Sequence[str], options: WatchStartOptions
    ) -> WatchStartedResponse:
        """Start watching a root."""

        request = WorkspaceRequestStartWatch(
            start_watch=WatchStartRequest(
                handle=self._handle, roots=roots, options=options
            )
        )
        response = self._connection.request(request)

        return expect_response(response, WorkspaceResponseWatchStarted).watch_started

    def next_watch_batch(self) -> WatchBatchResponse:
        """Receive and apply the next watch batch."""

        request = WorkspaceRequestNextWatchBatch(
            next_watch_batch=WatchNextRequest(handle=self._handle)
        )
        response = self._connection.request(request)

        return expect_response(
            response, WorkspaceResponseWatchBatchReady
        ).watch_batch_ready

    def stop_watch(self) -> WatchStoppedResponse:
        """Stop watching a root."""

        request = WorkspaceRequestStopWatch(
            stop_watch=WatchStopRequest(handle=self._handle)
        )
        response = self._connection.request(request)

        return expect_response(response, WorkspaceResponseWatchStopped).watch_stopped

    def check(self, input: CheckInput) -> CheckOutput:
        """Check source state."""

        request = WorkspaceRequestCheck(handle=self._handle, input=input)
        response = self._connection.request(request)

        return expect_response(response, WorkspaceResponseCheck).check

    def lint(self, input: LintInput) -> LintOutput:
        """Lint source state."""

        request = WorkspaceRequestLint(handle=self._handle, input=input)
        response = self._connection.request(request)

        return expect_response(response, WorkspaceResponseLint).lint

    def format(self, input: FormatInput) -> FormatOutput:
        """Format source files or content."""

        request = WorkspaceRequestFormat(handle=self._handle, input=input)
        response = self._connection.request(request)

        return expect_response(response, WorkspaceResponseFormat).format

    def build(self, input: BuildInput) -> BuildOutput:
        """Build target artifacts."""

        request = WorkspaceRequestBuild(handle=self._handle, input=input)
        response = self._connection.request(request)

        return expect_response(response, WorkspaceResponseBuild).build

    def run(self, input: RunInput) -> RunOutput:
        """Run a workspace target."""

        request = WorkspaceRequestRun(handle=self._handle, input=input)
        response = self._connection.request(request)

        return expect_response(response, WorkspaceResponseRun).run

    def test(self, input: TestInput) -> TestOutput:
        """Run workspace tests."""

        request = WorkspaceRequestTest(handle=self._handle, input=input)
        response = self._connection.request(request)

        return expect_response(response, WorkspaceResponseTest).test

    def doc(self, input: DocInput) -> DocOutput:
        """Generate documentation."""

        request = WorkspaceRequestDoc(handle=self._handle, input=input)
        response = self._connection.request(request)

        return expect_response(response, WorkspaceResponseDoc).doc

    def bench(self, input: BenchInput) -> BenchOutput:
        """Run benchmarks."""

        request = WorkspaceRequestBench(handle=self._handle, input=input)
        response = self._connection.request(request)

        return expect_response(response, WorkspaceResponseBench).bench

    def info(self, input: InfoInput) -> InfoOutput:
        """Return workspace information."""

        request = WorkspaceRequestInfo(handle=self._handle, input=input)
        response = self._connection.request(request)

        return expect_response(response, WorkspaceResponseInfo).info

    def targets(self, input: TargetsInput) -> TargetsOutput:
        """Return configured targets."""

        request = WorkspaceRequestTargets(handle=self._handle, input=input)
        response = self._connection.request(request)

        return expect_response(response, WorkspaceResponseTargets).targets

    def cache(self, input: CacheInput) -> CacheOutput:
        """Return cache locations."""

        request = WorkspaceRequestCache(handle=self._handle, input=input)
        response = self._connection.request(request)

        return expect_response(response, WorkspaceResponseCache).cache

    def settings(self, input: SettingsInput) -> SettingsOutput:
        """Return resolved settings."""

        request = WorkspaceRequestSettings(handle=self._handle, input=input)
        response = self._connection.request(request)

        return expect_response(response, WorkspaceResponseSettings).settings

    def doctor(self, input: DoctorInput) -> DoctorOutput:
        """Return workspace health information."""

        request = WorkspaceRequestDoctor(handle=self._handle, input=input)
        response = self._connection.request(request)

        return expect_response(response, WorkspaceResponseDoctor).doctor

    def task(self, input: TaskInput) -> TaskOutput:
        """Run workspace tasks."""

        request = WorkspaceRequestTask(handle=self._handle, input=input)
        response = self._connection.request(request)

        return expect_response(response, WorkspaceResponseTask).task

    def clean(self, input: CleanInput) -> CleanOutput:
        """Clean generated state."""

        request = WorkspaceRequestClean(handle=self._handle, input=input)
        response = self._connection.request(request)

        return expect_response(response, WorkspaceResponseClean).clean

    def artifact(self, artifact: ArtifactReference) -> ArtifactBlob:
        """Return one artifact payload."""

        request = WorkspaceRequestArtifact(handle=self._handle, artifact=artifact)
        response = self._connection.request(request)

        return expect_response(
            response, WorkspaceResponseArtifactResult
        ).artifact_result

    def store(self, content: Content) -> ContentId:
        """Store one content payload."""

        request = WorkspaceRequestStore(handle=self._handle, content=content)
        response = self._connection.request(request)

        return expect_response(response, WorkspaceResponseStoreResult).store_result

    def load(self, content: ContentId) -> Content:
        """Load one content payload."""

        request = WorkspaceRequestLoad(handle=self._handle, content=content)
        response = self._connection.request(request)

        return expect_response(response, WorkspaceResponseLoadResult).load_result

    def export(self, request: ExportRequest) -> ExportResult:
        """Materialize derived outputs."""

        request = WorkspaceRequestExport(handle=self._handle, request=request)
        response = self._connection.request(request)

        return expect_response(response, WorkspaceResponseExportResult).export_result

    def diagnostics(self) -> Sequence[DiagnosticBatch]:
        """Request diagnostics snapshot."""

        response = self.query(WorkspaceQueryDiagnostics(handle=self._handle))

        return expect_query(response, WorkspaceQueryResponseDiagnostics).diagnostics

    def diagnostic_snapshots(self) -> Sequence[DiagnosticSnapshot]:
        """Request rich diagnostics with file images."""

        response = self.query(WorkspaceQueryDiagnosticSnapshots(handle=self._handle))

        return expect_query(
            response, WorkspaceQueryResponseDiagnosticSnapshots
        ).diagnostic_snapshots

    def file_diagnostics(self, path: str) -> DiagnosticSnapshot | None:
        """Request diagnostics for one file."""

        response = self.query(
            WorkspaceQueryFileDiagnostics(handle=self._handle, path=path)
        )

        return expect_query(
            response, WorkspaceQueryResponseFileDiagnostics
        ).file_diagnostics

    def file_open(self, path: str) -> bool:
        """Request whether one source file is open."""

        response = self.query(WorkspaceQueryFileOpen(handle=self._handle, path=path))

        return expect_query(response, WorkspaceQueryResponseFileOpen).file_open

    def current_revision(self) -> Revision:
        """Request the current semantic revision."""

        response = self.query(WorkspaceQueryCurrentRevision(handle=self._handle))

        return expect_query(
            response, WorkspaceQueryResponseCurrentRevision
        ).current_revision

    def root_snapshot(self, target: str | None) -> RootSnapshot:
        """Request query context for one root."""

        response = self.query(
            WorkspaceQueryRootSnapshot(handle=self._handle, target=target)
        )

        return expect_query(response, WorkspaceQueryResponseRootSnapshot).root_snapshot

    def file_snapshot(self, request: FileSnapshotRequest) -> FileSnapshot | None:
        """Request a source file snapshot."""

        response = self.query(
            WorkspaceQueryFileSnapshot(handle=self._handle, request=request)
        )

        return expect_query(response, WorkspaceQueryResponseFileSnapshot).file_snapshot

    def file_images(self, request: FileImagesRequest) -> Sequence[FileImage]:
        """Request source file images for a revision."""

        response = self.query(
            WorkspaceQueryFileImages(handle=self._handle, request=request)
        )

        return expect_query(response, WorkspaceQueryResponseFileImages).file_images

    def execute(self, request: QueryRequestPayload) -> QueryResponsePayload:
        """Execute a query."""

        response = self.query(
            WorkspaceQueryExecute(handle=self._handle, request=request)
        )

        return expect_query(response, WorkspaceQueryResponseQuery).query

    def execute_batch(
        self, requests: Sequence[QueryRequestPayload]
    ) -> Sequence[QueryResponsePayload]:
        """Execute a batch of queries."""

        response = self.query(
            WorkspaceQueryExecuteBatch(handle=self._handle, requests=requests)
        )

        return expect_query(response, WorkspaceQueryResponseQueryBatch).query_batch

    def query(self, query: WorkspaceQuery) -> WorkspaceQueryResponse:
        """Run one exact workspace query."""

        response = self._connection.request(WorkspaceRequestQuery(query=query))

        return expect_response(response, WorkspaceResponseQueryResult).query_result


def expect_response(response: WorkspaceResponse, ty: type[T]) -> T:
    """Return one response variant or throw an error."""

    if isinstance(response, ty):
        return response

    raise TypeError(f"expected {ty.__name__}, got {type(response).__name__}")


def expect_query(response: WorkspaceQueryResponse, ty: type[T]) -> T:
    """Return one query response variant."""

    if isinstance(response, ty):
        return response

    raise TypeError(f"expected {ty.__name__}, got {type(response).__name__}")


__all__ = [
    "WorkspaceClient",
]
