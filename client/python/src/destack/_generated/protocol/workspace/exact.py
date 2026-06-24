# generated client target, do not edit

from __future__ import annotations

import destack
import destack._generated.protocol

from collections.abc import Sequence
from typing import TypeVar

from destack.protocol.connection import Connection
import destack._generated.artifact.reference
import destack._generated.repository.revision
import destack._generated.source.file.model.file
from ..query import *
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
from destack._generated.artifact.reference import ArtifactReference
from destack._generated.source.edit.update import Edit as SourceUpdate
from destack._generated.source.file.model.file import Content, ContentId

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

    def close_root(self) -> destack._generated.protocol.root.RootClosedResponse:
        """Close a root handle."""

        payload = WorkspaceRequestCloseRoot(
            close_root=CloseRootRequest(handle=self._handle)
        )
        response = self._connection.request(payload)

        return expect_response(response, WorkspaceResponseRootClosed).root_closed

    def reload_root(
        self, reason: destack._generated.protocol.workspace.root.ReloadReason
    ) -> destack._generated.protocol.root.RootReloadResponse:
        """Reload a root."""

        payload = WorkspaceRequestReloadRoot(
            reload_root=ReloadRootRequest(handle=self._handle, reason=reason)
        )
        response = self._connection.request(payload)

        return expect_response(response, WorkspaceResponseRootReloaded).root_reloaded

    def apply_file_operation(
        self, operation: destack._generated.protocol.workspace.file.image.FileOperation
    ) -> destack._generated.protocol.root.FileOperationResponse:
        """Apply a file operation to a root."""

        payload = WorkspaceRequestApplyFileOperation(
            apply_file_operation=FileOperationRequest(
                handle=self._handle, operation=operation
            )
        )
        response = self._connection.request(payload)

        return expect_response(
            response, WorkspaceResponseFileOperationApplied
        ).file_operation_applied

    def apply_source_update(
        self, update: destack._generated.protocol.workspace.file.update.SourceUpdate
    ) -> destack._generated.protocol.root.SourceUpdateResponse:
        """Apply a source update to a root."""

        payload = WorkspaceRequestApplySourceUpdate(
            apply_source_update=SourceUpdateRequest(handle=self._handle, update=update)
        )
        response = self._connection.request(payload)

        return expect_response(response, WorkspaceResponseSourceUpdated).source_updated

    def start_watch(
        self, roots: Sequence[str], options: WatchStartOptions
    ) -> destack._generated.protocol.watch.WatchStartedResponse:
        """Start watching a root."""

        payload = WorkspaceRequestStartWatch(
            start_watch=WatchStartRequest(
                handle=self._handle, roots=roots, options=options
            )
        )
        response = self._connection.request(payload)

        return expect_response(response, WorkspaceResponseWatchStarted).watch_started

    def next_watch_batch(self) -> destack._generated.protocol.watch.WatchBatchResponse:
        """Receive and apply the next watch batch."""

        payload = WorkspaceRequestNextWatchBatch(
            next_watch_batch=WatchNextRequest(handle=self._handle)
        )
        response = self._connection.request(payload)

        return expect_response(
            response, WorkspaceResponseWatchBatchReady
        ).watch_batch_ready

    def stop_watch(self) -> destack._generated.protocol.watch.WatchStoppedResponse:
        """Stop watching a root."""

        payload = WorkspaceRequestStopWatch(
            stop_watch=WatchStopRequest(handle=self._handle)
        )
        response = self._connection.request(payload)

        return expect_response(response, WorkspaceResponseWatchStopped).watch_stopped

    def check(
        self, input: destack._generated.protocol.workspace.command.check.CheckInput
    ) -> destack._generated.protocol.workspace.command.output.CheckOutput:
        """Check source state."""

        payload = WorkspaceRequestCheck(handle=self._handle, input=input)
        response = self._connection.request(payload)

        return expect_response(response, WorkspaceResponseCheck).check

    def lint(
        self, input: destack._generated.protocol.workspace.command.check.LintInput
    ) -> destack._generated.protocol.workspace.command.output.LintOutput:
        """Lint source state."""

        payload = WorkspaceRequestLint(handle=self._handle, input=input)
        response = self._connection.request(payload)

        return expect_response(response, WorkspaceResponseLint).lint

    def format(
        self, input: destack._generated.protocol.workspace.command.format.FormatInput
    ) -> destack._generated.protocol.workspace.command.output.FormatOutput:
        """Format source files or content."""

        payload = WorkspaceRequestFormat(handle=self._handle, input=input)
        response = self._connection.request(payload)

        return expect_response(response, WorkspaceResponseFormat).format

    def build(
        self, input: destack._generated.protocol.workspace.command.build.BuildInput
    ) -> destack._generated.protocol.workspace.command.output.BuildOutput:
        """Build target artifacts."""

        payload = WorkspaceRequestBuild(handle=self._handle, input=input)
        response = self._connection.request(payload)

        return expect_response(response, WorkspaceResponseBuild).build

    def run(
        self, input: destack._generated.protocol.workspace.command.run.RunInput
    ) -> destack._generated.protocol.workspace.command.output.RunOutput:
        """Run a workspace target."""

        payload = WorkspaceRequestRun(handle=self._handle, input=input)
        response = self._connection.request(payload)

        return expect_response(response, WorkspaceResponseRun).run

    def test(
        self, input: destack._generated.protocol.workspace.command.test.TestInput
    ) -> destack._generated.protocol.workspace.command.output.TestOutput:
        """Run workspace tests."""

        payload = WorkspaceRequestTest(handle=self._handle, input=input)
        response = self._connection.request(payload)

        return expect_response(response, WorkspaceResponseTest).test

    def doc(
        self, input: destack._generated.protocol.workspace.command.doc.DocInput
    ) -> destack._generated.protocol.workspace.command.output.DocOutput:
        """Generate documentation."""

        payload = WorkspaceRequestDoc(handle=self._handle, input=input)
        response = self._connection.request(payload)

        return expect_response(response, WorkspaceResponseDoc).doc

    def bench(
        self, input: destack._generated.protocol.workspace.command.bench.BenchInput
    ) -> destack._generated.protocol.workspace.command.output.BenchOutput:
        """Run benchmarks."""

        payload = WorkspaceRequestBench(handle=self._handle, input=input)
        response = self._connection.request(payload)

        return expect_response(response, WorkspaceResponseBench).bench

    def info(
        self, input: destack._generated.protocol.workspace.command.info.InfoInput
    ) -> destack._generated.protocol.workspace.command.output.InfoOutput:
        """Return workspace information."""

        payload = WorkspaceRequestInfo(handle=self._handle, input=input)
        response = self._connection.request(payload)

        return expect_response(response, WorkspaceResponseInfo).info

    def targets(
        self, input: destack._generated.protocol.workspace.command.targets.TargetsInput
    ) -> destack._generated.protocol.workspace.command.output.TargetsOutput:
        """Return configured targets."""

        payload = WorkspaceRequestTargets(handle=self._handle, input=input)
        response = self._connection.request(payload)

        return expect_response(response, WorkspaceResponseTargets).targets

    def cache(
        self, input: destack._generated.protocol.workspace.command.cache.CacheInput
    ) -> destack._generated.protocol.workspace.command.output.CacheOutput:
        """Return cache locations."""

        payload = WorkspaceRequestCache(handle=self._handle, input=input)
        response = self._connection.request(payload)

        return expect_response(response, WorkspaceResponseCache).cache

    def settings(
        self,
        input: destack._generated.protocol.workspace.command.settings.SettingsInput,
    ) -> destack._generated.protocol.workspace.command.output.SettingsOutput:
        """Return resolved settings."""

        payload = WorkspaceRequestSettings(handle=self._handle, input=input)
        response = self._connection.request(payload)

        return expect_response(response, WorkspaceResponseSettings).settings

    def doctor(
        self, input: destack._generated.protocol.workspace.command.doctor.DoctorInput
    ) -> destack._generated.protocol.workspace.command.output.DoctorOutput:
        """Return workspace health information."""

        payload = WorkspaceRequestDoctor(handle=self._handle, input=input)
        response = self._connection.request(payload)

        return expect_response(response, WorkspaceResponseDoctor).doctor

    def task(
        self, input: destack._generated.protocol.workspace.command.task.TaskInput
    ) -> destack._generated.protocol.workspace.command.output.TaskOutput:
        """Run workspace tasks."""

        payload = WorkspaceRequestTask(handle=self._handle, input=input)
        response = self._connection.request(payload)

        return expect_response(response, WorkspaceResponseTask).task

    def clean(
        self, input: destack._generated.protocol.workspace.command.clean.CleanInput
    ) -> destack._generated.protocol.workspace.command.output.CleanOutput:
        """Clean generated state."""

        payload = WorkspaceRequestClean(handle=self._handle, input=input)
        response = self._connection.request(payload)

        return expect_response(response, WorkspaceResponseClean).clean

    def artifact(
        self, artifact: destack._generated.artifact.reference.ArtifactReference
    ) -> ArtifactBlob:
        """Return one artifact payload."""

        payload = WorkspaceRequestArtifact(handle=self._handle, artifact=artifact)
        response = self._connection.request(payload)

        return expect_response(
            response, WorkspaceResponseArtifactResult
        ).artifact_result

    def store(
        self, content: destack._generated.source.file.model.file.Content
    ) -> destack._generated.source.file.model.file.ContentId:
        """Store one content payload."""

        payload = WorkspaceRequestStore(handle=self._handle, content=content)
        response = self._connection.request(payload)

        return expect_response(response, WorkspaceResponseStoreResult).store_result

    def load(
        self, content: destack._generated.source.file.model.file.ContentId
    ) -> destack._generated.source.file.model.file.Content:
        """Load one content payload."""

        payload = WorkspaceRequestLoad(handle=self._handle, content=content)
        response = self._connection.request(payload)

        return expect_response(response, WorkspaceResponseLoadResult).load_result

    def export(
        self,
        request: destack._generated.protocol.workspace.artifact.export.ExportRequest,
    ) -> destack._generated.protocol.workspace.artifact.export.ExportResult:
        """Materialize derived outputs."""

        payload = WorkspaceRequestExport(handle=self._handle, request=request)
        response = self._connection.request(payload)

        return expect_response(response, WorkspaceResponseExportResult).export_result

    def diagnostics(
        self,
    ) -> Sequence[destack._generated.protocol.notification.DiagnosticBatch]:
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

    def current_revision(self) -> destack._generated.repository.revision.Revision:
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

    def file_images(
        self, request: FileImagesRequest
    ) -> Sequence[destack._generated.protocol.workspace.file.image.FileImage]:
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
