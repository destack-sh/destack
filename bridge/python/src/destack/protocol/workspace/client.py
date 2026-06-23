from __future__ import annotations

from collections.abc import Mapping, Sequence
from typing import Protocol, TypeVar

from destack._generated.protocol.artifact.reference import ArtifactReference
from ..connection import Connection, connect_endpoint
from destack._generated.protocol.query.model import (
    DiagnosticSnapshot,
    FileImagesRequest,
    FileSnapshot,
    FileSnapshotRequest,
    RootSnapshot,
    WorkspaceQuery,
    WorkspaceQueryResponse,
)
from destack._generated.protocol.notification import DiagnosticBatch
from destack._generated.protocol.repository.revision import Revision
from destack._generated.protocol.request import (
    WorkspaceRequestOpenRoot,
)
from destack._generated.protocol.response import (
    ArtifactBlob,
    WorkspaceResponse,
    WorkspaceResponseRootOpened,
)
from destack._generated.protocol.root import (
    OpenRootRequest,
    RootId,
    RootOpenOptions,
    SourceUpdateResponse,
)
from destack._generated.protocol.workspace.file.update import SourceUpdate
from destack._generated.protocol.source.file.model.file import (
    Content,
    ContentBinary,
    ContentId,
    ContentText,
)
from destack._generated.protocol.workspace.command.bench import BenchInput
from destack._generated.protocol.workspace.command.build import BuildInput, BuildOutputs
from destack._generated.protocol.workspace.command.cache import CacheInput
from destack._generated.protocol.workspace.command.check import CheckInput, LintInput
from destack._generated.protocol.workspace.command.common import (
    CommandRevisionCurrent,
)
from destack._generated.protocol.workspace.command.doc import DocInput
from destack._generated.protocol.workspace.command.doctor import DoctorInput
from destack._generated.protocol.workspace.command.format import (
    FormatInput,
    FormatSource,
    FormatSourceContent,
    FormatSourceFiles,
    FormatSourceOpenFile,
)
from destack._generated.protocol.workspace.command.info import InfoInput
from destack._generated.protocol.workspace.command.output import (
    BenchOutput,
    BuildOutput,
    CacheOutput,
    CheckOutput,
    CleanOutput,
    DocOutput,
    DoctorOutput,
    FormatOutput,
    InfoOutput,
    LintOutput,
    RunOutput,
    SettingsOutput,
    TargetsOutput,
    TaskOutput,
    TestOutput,
)
from destack._generated.protocol.workspace.command.run import RunInput, RunModeProgram
from destack._generated.protocol.workspace.command.settings import SettingsInput
from destack._generated.protocol.workspace.command.targets import TargetsInput
from destack._generated.protocol.workspace.command.task import TaskActionList, TaskInput
from destack._generated.protocol.workspace.command.test import TestInput
from destack._generated.protocol.workspace.artifact.export import (
    ExportRequest,
    ExportResult,
)
from destack._generated.protocol.workspace.exact import WorkspaceClient
from destack._generated.protocol.workspace.file.image import FileImage, FileOperation
from destack._generated.protocol.workspace.message import UpdateBatch
from destack._generated.protocol.workspace.root import ReloadReason
from destack._generated.protocol.watch import (
    WatchBatchResponse,
    WatchStartOptions,
    WatchStartedResponse,
    WatchStoppedResponse,
)

DEFAULT_WATCH_COALESCE_WINDOW_MS = 50
DEFAULT_WATCH_BATCH_SIZE = 1024
CommandInit = Mapping[str, object]
T = TypeVar("T")


class Workspace(Protocol):
    """Workspace command and query interface."""

    def connection(self) -> Connection:
        """Return the protocol connection backing this workspace."""

    def workspace(self) -> str:
        """Return the workspace path registered with the server."""

    def root(self) -> str:
        """Return the opened root path."""

    def handle(self) -> RootId:
        """Return the opened protocol root handle."""

    def revision(self) -> Revision:
        """Return the current root revision."""

    def diagnostics(self) -> Sequence[DiagnosticBatch]:
        """Return plain diagnostic batches for this root."""

    def diagnostic_snapshots(self) -> Sequence[DiagnosticSnapshot]:
        """Return diagnostic snapshots with source file images for this root."""

    def snapshot(self, target: str | None = None) -> RootSnapshot:
        """Return query context for this root."""

    def file_snapshot(self, request: FileSnapshotRequest) -> FileSnapshot | None:
        """Return one source file snapshot."""

    def file_images(self, request: FileImagesRequest) -> Sequence[FileImage]:
        """Return source file images for one revision."""

    def apply_source_update(self, update: SourceUpdate) -> SourceUpdateResponse:
        """Apply a source update through the workspace protocol."""

    def apply_file_operation(self, operation: FileOperation) -> UpdateBatch:
        """Apply one file operation through the workspace protocol."""

    def reload(self, reason: ReloadReason = "manual") -> UpdateBatch:
        """Reload this root from the server filesystem."""

    def check(self, request: CheckInput | CommandInit | None = None) -> CheckOutput:
        """Check source state."""

    def lint(self, request: LintInput | CommandInit | None = None) -> LintOutput:
        """Lint source state."""

    def format(self, request: FormatInput | CommandInit | None = None) -> FormatOutput:
        """Format source files or content."""

    def build(self, request: BuildInput | CommandInit | None = None) -> BuildOutput:
        """Build target artifacts."""

    def run(self, request: RunInput | CommandInit | None = None) -> RunOutput:
        """Run one workspace target."""

    def test(self, request: TestInput | CommandInit | None = None) -> TestOutput:
        """Run workspace tests."""

    def doc(self, request: DocInput | CommandInit | None = None) -> DocOutput:
        """Generate documentation."""

    def bench(self, request: BenchInput | CommandInit | None = None) -> BenchOutput:
        """Run benchmarks."""

    def info(self, request: InfoInput | CommandInit | None = None) -> InfoOutput:
        """Return workspace information."""

    def targets(
        self, request: TargetsInput | CommandInit | None = None
    ) -> TargetsOutput:
        """Return configured targets."""

    def cache(self, request: CacheInput | CommandInit | None = None) -> CacheOutput:
        """Return cache locations."""

    def settings(
        self, request: SettingsInput | CommandInit | None = None
    ) -> SettingsOutput:
        """Return resolved settings."""

    def doctor(self, request: DoctorInput | CommandInit | None = None) -> DoctorOutput:
        """Return workspace health information."""

    def task(self, request: TaskInput | CommandInit | None = None) -> TaskOutput:
        """Run workspace tasks."""

    def clean(self, request: CleanInput | CommandInit | None = None) -> CleanOutput:
        """Clean generated state."""

    def artifact(self, artifact: ArtifactReference) -> ArtifactBlob:
        """Return one artifact payload."""

    def store(self, content: Content | str | bytes | bytearray) -> ContentId:
        """Store one content payload."""

    def load(self, content: ContentId) -> Content:
        """Load one content payload."""

    def export(self, request: ExportRequest) -> ExportResult:
        """Materialize derived outputs on the workspace host."""

    def query(self, query: WorkspaceQuery) -> WorkspaceQueryResponse:
        """Run one workspace query against this root."""

    def watch(
        self,
        roots: Sequence[str] | None = None,
        options: WatchStartOptions | CommandInit | None = None,
    ) -> WatchStartedResponse:
        """Watch roots through this workspace handle."""

    def next_watch(self) -> WatchBatchResponse:
        """Receive and apply the next watch batch."""

    def unwatch(self) -> WatchStoppedResponse:
        """Stop watching roots through this workspace handle."""

    def close(self) -> None:
        """Close this root handle on the workspace server."""


class RemoteWorkspace:
    """Workspace backed by the workspace protocol."""

    def __init__(
        self,
        connection: Connection,
        workspace: str,
        root: str,
        handle: RootId,
    ) -> None:
        self._client = WorkspaceClient(connection, handle)
        self._workspace = workspace
        self._root = root

    @classmethod
    def open(
        cls,
        *,
        workspace: str,
        root: str | None = None,
        url: str | None = None,
        connection: Connection | None = None,
        load_index: bool = False,
    ) -> RemoteWorkspace:
        """Open one remote workspace root."""

        connection = remote_connection(url=url, connection=connection)
        connection.handshake()

        request = WorkspaceRequestOpenRoot(
            open_root=OpenRootRequest(
                root=root or workspace,
                options=RootOpenOptions(load_index=load_index),
            ),
        )
        response = expect_response(
            connection.request(request), WorkspaceResponseRootOpened
        )
        opened = response.root_opened

        return cls(
            connection=connection,
            workspace=workspace,
            root=opened.root,
            handle=opened.handle,
        )

    def connection(self) -> Connection:
        """Return the protocol connection backing this workspace."""

        return self._client.connection()

    def workspace(self) -> str:
        """Return the workspace path registered with the server."""

        return self._workspace

    def root(self) -> str:
        """Return the opened root path."""

        return self._root

    def handle(self) -> RootId:
        """Return the opened protocol root handle."""

        return self._client.handle()

    def revision(self) -> Revision:
        """Return the current root revision."""

        return self._client.current_revision()

    def diagnostics(self) -> Sequence[DiagnosticBatch]:
        """Return plain diagnostic batches for this root."""

        return self._client.diagnostics()

    def diagnostic_snapshots(self) -> Sequence[DiagnosticSnapshot]:
        """Return diagnostic snapshots with source file images for this root."""

        return self._client.diagnostic_snapshots()

    def snapshot(self, target: str | None = None) -> RootSnapshot:
        """Return query context for this root."""

        return self._client.root_snapshot(target)

    def file_snapshot(self, request: FileSnapshotRequest) -> FileSnapshot | None:
        """Return one source file snapshot."""

        return self._client.file_snapshot(request)

    def file_images(self, request: FileImagesRequest) -> Sequence[FileImage]:
        """Return source file images for one revision."""

        return self._client.file_images(request)

    def apply_file_operation(self, operation: FileOperation) -> UpdateBatch:
        """Apply one file operation through the workspace protocol."""

        response = self._client.apply_file_operation(operation)

        return response.updates

    def apply_source_update(self, update: SourceUpdate) -> SourceUpdateResponse:
        """Apply a source update through the workspace protocol."""

        return self._client.apply_source_update(update)

    def reload(self, reason: ReloadReason = "manual") -> UpdateBatch:
        """Reload this root from the server filesystem."""

        response = self._client.reload_root(reason)

        return response.updates

    def check(self, request: CheckInput | CommandInit | None = None) -> CheckOutput:
        """Check source state."""

        return self._client.check(check_input(request))

    def lint(self, request: LintInput | CommandInit | None = None) -> LintOutput:
        """Lint source state."""

        return self._client.lint(lint_input(request))

    def format(self, request: FormatInput | CommandInit | None = None) -> FormatOutput:
        """Format source files or content."""

        return self._client.format(format_input(request))

    def build(self, request: BuildInput | CommandInit | None = None) -> BuildOutput:
        """Build target artifacts."""

        return self._client.build(build_input(request))

    def run(self, request: RunInput | CommandInit | None = None) -> RunOutput:
        """Run one workspace target."""

        return self._client.run(run_input(request))

    def test(self, request: TestInput | CommandInit | None = None) -> TestOutput:
        """Run workspace tests."""

        return self._client.test(command_test_input(request))

    def doc(self, request: DocInput | CommandInit | None = None) -> DocOutput:
        """Generate documentation."""

        return self._client.doc(doc_input(request))

    def bench(self, request: BenchInput | CommandInit | None = None) -> BenchOutput:
        """Run benchmarks."""

        return self._client.bench(bench_input(request))

    def info(self, request: InfoInput | CommandInit | None = None) -> InfoOutput:
        """Return workspace information."""

        return self._client.info(info_input(request))

    def targets(
        self, request: TargetsInput | CommandInit | None = None
    ) -> TargetsOutput:
        """Return configured targets."""

        return self._client.targets(targets_input(request))

    def cache(self, request: CacheInput | CommandInit | None = None) -> CacheOutput:
        """Return cache locations."""

        return self._client.cache(cache_input(request))

    def settings(
        self, request: SettingsInput | CommandInit | None = None
    ) -> SettingsOutput:
        """Return resolved settings."""

        return self._client.settings(settings_input(request))

    def doctor(self, request: DoctorInput | CommandInit | None = None) -> DoctorOutput:
        """Return workspace health information."""

        return self._client.doctor(doctor_input(request))

    def task(self, request: TaskInput | CommandInit | None = None) -> TaskOutput:
        """Run workspace tasks."""

        return self._client.task(task_input(request))

    def clean(self, request: CleanInput | CommandInit | None = None) -> CleanOutput:
        """Clean generated state."""

        return self._client.clean(clean_input(request))

    def artifact(self, artifact: ArtifactReference) -> ArtifactBlob:
        """Return one artifact payload."""

        return self._client.artifact(artifact)

    def store(self, content: Content | str | bytes | bytearray) -> ContentId:
        """Store one content payload."""

        return self._client.store(content_payload(content))

    def load(self, content: ContentId) -> Content:
        """Load one content payload."""

        return self._client.load(content)

    def export(self, request: ExportRequest) -> ExportResult:
        """Materialize derived outputs on the workspace host."""

        return self._client.export(request)

    def query(self, query: WorkspaceQuery) -> WorkspaceQueryResponse:
        """Run one workspace query against this root."""

        return self._client.query(query)

    def watch(
        self,
        roots: Sequence[str] | None = None,
        options: WatchStartOptions | CommandInit | None = None,
    ) -> WatchStartedResponse:
        """Watch roots through this workspace handle."""

        roots = roots if roots is not None else [self._root]

        return self._client.start_watch(roots, watch_options(options))

    def next_watch(self) -> WatchBatchResponse:
        """Receive and apply the next watch batch."""

        return self._client.next_watch_batch()

    def unwatch(self) -> WatchStoppedResponse:
        """Stop watching roots through this workspace handle."""

        return self._client.stop_watch()

    def close(self) -> None:
        """Close this root handle on the workspace server."""

        self._client.close_root()


def open_remote_workspace(
    *,
    workspace: str,
    root: str | None = None,
    url: str | None = None,
    connection: Connection | None = None,
    load_index: bool = False,
) -> RemoteWorkspace:
    """Open one remote workspace through a workspace protocol endpoint."""

    return RemoteWorkspace.open(
        workspace=workspace,
        root=root,
        url=url,
        connection=connection,
        load_index=load_index,
    )


def remote_connection(
    *,
    url: str | None,
    connection: Connection | None,
) -> Connection:
    """Return the provided endpoint connection or connect to one URL."""

    if connection is not None:
        return connection

    if url is not None:
        return connect_endpoint(url)

    raise ValueError("remote workspace requires either connection or url")


def command_fields(input: CommandInit | None) -> dict[str, object]:
    """Return exact shared command fields from sparse command input."""

    input = input or {}

    return {
        "revision": input.get("revision", CommandRevisionCurrent()),
        "inputs": input.get("inputs", []),
        "config_inputs": input.get("config_inputs", True),
        "cwd": input.get("cwd"),
        "manifest": input.get("manifest"),
        "target": input.get("target"),
        "target_overrides": input.get("target_overrides"),
        "profile": input.get("profile"),
        "env": input.get("env", []),
        "overrides": input.get("overrides", []),
        "watch": input.get("watch", False),
        "dry_run": input.get("dry_run", False),
    }


def check_input(input: CheckInput | CommandInit | None) -> CheckInput:
    """Return exact check input from sparse check input."""

    if isinstance(input, CheckInput):
        return input

    input = input or {}
    fields = command_fields(input)

    return CheckInput(
        **fields,
        lint=input.get("lint", True),
        fix=input.get("fix", False),
        unsafe_fixes=input.get("unsafe_fixes", False),
        diff=input.get("diff", False),
        trace=input.get("trace", "summary"),
    )


def lint_input(input: LintInput | CommandInit | None) -> LintInput:
    """Return exact lint input from sparse lint input."""

    if isinstance(input, LintInput):
        return input

    input = input or {}
    fields = command_fields(input)

    return LintInput(
        **fields,
        fix=input.get("fix", False),
        unsafe_fixes=input.get("unsafe_fixes", False),
        diff=input.get("diff", False),
    )


def format_input(input: FormatInput | CommandInit | None) -> FormatInput:
    """Return exact format input from sparse format input."""

    if isinstance(input, FormatInput):
        return input

    input = input or {}
    fields = command_fields(input)

    return FormatInput(
        **fields,
        source=format_source(input.get("source")),
        mode=input.get("mode", "preview"),
    )


def format_source(source: object) -> FormatSource:
    """Return exact format source from sparse format source input."""

    if source is None:
        return FormatSourceFiles(files=[])

    if isinstance(source, str):
        return FormatSourceFiles(files=[source])

    if isinstance(source, Sequence) and not isinstance(source, (bytes, bytearray)):
        return FormatSourceFiles(files=list(source))

    if isinstance(
        source, (FormatSourceFiles, FormatSourceOpenFile, FormatSourceContent)
    ):
        return source

    if isinstance(source, Mapping) and "files" in source:
        return FormatSourceFiles(files=source["files"])

    if isinstance(source, Mapping) and "open_file" in source:
        return FormatSourceOpenFile(open_file=source["open_file"])

    if isinstance(source, Mapping) and "content" in source:
        return FormatSourceContent(
            name=source.get("name", "<content>"),
            file_type=source.get("file_type", "destack"),
            content=source["content"],
        )

    raise TypeError("unsupported format source")


def build_input(input: BuildInput | CommandInit | None) -> BuildInput:
    """Return exact build input from sparse build input."""

    if isinstance(input, BuildInput):
        return input

    input = input or {}
    fields = command_fields(input)

    return BuildInput(
        **fields,
        trace=input.get("trace", "summary"),
        product=input.get("product"),
        outputs=input.get(
            "outputs",
            BuildOutputs(products=True, bundles=True, programs=True, assets=False),
        ),
    )


def run_input(input: RunInput | CommandInit | None) -> RunInput:
    """Return exact run input from sparse run input."""

    if isinstance(input, RunInput):
        return input

    input = input or {}
    fields = command_fields(input)

    return RunInput(
        **fields,
        entry=input.get("entry"),
        args=input.get("args", []),
        run_mode=input.get("run_mode", RunModeProgram()),
    )


def command_test_input(input: TestInput | CommandInit | None) -> TestInput:
    """Return exact test input from sparse test input."""

    if isinstance(input, TestInput):
        return input

    return TestInput(**command_fields(input))


def doc_input(input: DocInput | CommandInit | None) -> DocInput:
    """Return exact doc input from sparse doc input."""

    if isinstance(input, DocInput):
        return input

    return DocInput(**command_fields(input))


def bench_input(input: BenchInput | CommandInit | None) -> BenchInput:
    """Return exact bench input from sparse bench input."""

    if isinstance(input, BenchInput):
        return input

    return BenchInput(**command_fields(input))


def info_input(input: InfoInput | CommandInit | None) -> InfoInput:
    """Return exact info input from sparse info input."""

    if isinstance(input, InfoInput):
        return input

    input = input or {}

    return InfoInput(**command_fields(input), all=input.get("all", False))


def targets_input(input: TargetsInput | CommandInit | None) -> TargetsInput:
    """Return exact targets input from sparse targets input."""

    if isinstance(input, TargetsInput):
        return input

    input = input or {}

    return TargetsInput(**command_fields(input), all=input.get("all", False))


def cache_input(input: CacheInput | CommandInit | None) -> CacheInput:
    """Return exact cache input from sparse cache input."""

    if isinstance(input, CacheInput):
        return input

    return CacheInput(**command_fields(input))


def settings_input(input: SettingsInput | CommandInit | None) -> SettingsInput:
    """Return exact settings input from sparse settings input."""

    if isinstance(input, SettingsInput):
        return input

    return SettingsInput(**command_fields(input))


def doctor_input(input: DoctorInput | CommandInit | None) -> DoctorInput:
    """Return exact doctor input from sparse doctor input."""

    if isinstance(input, DoctorInput):
        return input

    input = input or {}

    return DoctorInput(**command_fields(input), full=input.get("full", False))


def task_input(input: TaskInput | CommandInit | None) -> TaskInput:
    """Return exact task input from sparse task input."""

    if isinstance(input, TaskInput):
        return input

    input = input or {}

    return TaskInput(
        **command_fields(input),
        action=input.get("action", TaskActionList()),
        projects=input.get("projects", []),
        groups=input.get("groups", []),
    )


def clean_input(input: CleanInput | CommandInit | None) -> CleanInput:
    """Return exact clean input from sparse clean input."""

    if isinstance(input, CleanInput):
        return input

    input = input or {}

    return CleanInput(
        **command_fields(input),
        dir=input.get("dir"),
        dist=input.get("dist", False),
        cache=input.get("cache", False),
        all=input.get("all", False),
        all_packages=input.get("all_packages", False),
    )


def content_payload(content: Content | str | bytes | bytearray) -> Content:
    """Return exact content payload."""

    if isinstance(content, (ContentText, ContentBinary)):
        return content

    if isinstance(content, str):
        return ContentText(content=content)

    if isinstance(content, (bytes, bytearray)):
        return ContentBinary(content=bytes(content))

    raise TypeError("unsupported content payload")


def watch_options(options: WatchStartOptions | CommandInit | None) -> WatchStartOptions:
    """Return exact watch options from sparse watch options."""

    if isinstance(options, WatchStartOptions):
        return options

    options = options or {}

    return WatchStartOptions(
        coalesce_window_ms=options.get(
            "coalesce_window_ms", DEFAULT_WATCH_COALESCE_WINDOW_MS
        ),
        max_batch_size=options.get("max_batch_size", DEFAULT_WATCH_BATCH_SIZE),
    )


def expect_response(response: WorkspaceResponse, ty: type[T]) -> T:
    """Return one response variant or throw an error."""

    if isinstance(response, ty):
        return response

    raise TypeError(f"expected {ty.__name__}, got {type(response).__name__}")
