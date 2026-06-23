# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.artifact.reference
import destack._generated.protocol.envelope
import destack._generated.protocol.handshake
import destack._generated.protocol.query.model
import destack._generated.protocol.root
import destack._generated.protocol.source.file.model.file
import destack._generated.protocol.watch
import destack._generated.protocol.workspace.artifact.export
import destack._generated.protocol.workspace.command.bench
import destack._generated.protocol.workspace.command.build
import destack._generated.protocol.workspace.command.cache
import destack._generated.protocol.workspace.command.check
import destack._generated.protocol.workspace.command.clean
import destack._generated.protocol.workspace.command.doc
import destack._generated.protocol.workspace.command.doctor
import destack._generated.protocol.workspace.command.format
import destack._generated.protocol.workspace.command.info
import destack._generated.protocol.workspace.command.run
import destack._generated.protocol.workspace.command.settings
import destack._generated.protocol.workspace.command.targets
import destack._generated.protocol.workspace.command.task
import destack._generated.protocol.workspace.command.test

if TYPE_CHECKING:
    from destack._generated.protocol.artifact.reference import (
        ArtifactReference,
    )

    from destack._generated.protocol.envelope import (
        RequestId,
    )

    from destack._generated.protocol.handshake import (
        HandshakeRequest,
    )

    from destack._generated.protocol.query.model import (
        WorkspaceQuery,
    )

    from destack._generated.protocol.root import (
        CloseRootRequest,
        FileOperationRequest,
        OpenRootRequest,
        ReloadRootRequest,
        RootId,
        SourceUpdateRequest,
    )

    from destack._generated.protocol.source.file.model.file import (
        Content,
        ContentId,
    )

    from destack._generated.protocol.watch import (
        WatchNextRequest,
        WatchStartRequest,
        WatchStopRequest,
    )

    from destack._generated.protocol.workspace.artifact.export import (
        ExportRequest,
    )

    from destack._generated.protocol.workspace.command.bench import (
        BenchInput,
    )

    from destack._generated.protocol.workspace.command.build import (
        BuildInput,
    )

    from destack._generated.protocol.workspace.command.cache import (
        CacheInput,
    )

    from destack._generated.protocol.workspace.command.check import (
        CheckInput,
        LintInput,
    )

    from destack._generated.protocol.workspace.command.clean import (
        CleanInput,
    )

    from destack._generated.protocol.workspace.command.doc import (
        DocInput,
    )

    from destack._generated.protocol.workspace.command.doctor import (
        DoctorInput,
    )

    from destack._generated.protocol.workspace.command.format import (
        FormatInput,
    )

    from destack._generated.protocol.workspace.command.info import (
        InfoInput,
    )

    from destack._generated.protocol.workspace.command.run import (
        RunInput,
    )

    from destack._generated.protocol.workspace.command.settings import (
        SettingsInput,
    )

    from destack._generated.protocol.workspace.command.targets import (
        TargetsInput,
    )

    from destack._generated.protocol.workspace.command.task import (
        TaskInput,
    )

    from destack._generated.protocol.workspace.command.test import (
        TestInput,
    )


@dataclass(frozen=True, slots=True)
class WorkspaceRequestHandshake:
    """Negotiate protocol version and capabilities."""

    handshake: HandshakeRequest
    kind: Literal["handshake"] = "handshake"


@dataclass(frozen=True, slots=True)
class WorkspaceRequestPing:
    """Check server liveness."""

    kind: Literal["ping"] = "ping"


@dataclass(frozen=True, slots=True)
class WorkspaceRequestCancel:
    """Cancel an in flight request."""

    id: RequestId
    kind: Literal["cancel"] = "cancel"


@dataclass(frozen=True, slots=True)
class WorkspaceRequestShutdown:
    """Request server shutdown."""

    kind: Literal["shutdown"] = "shutdown"


@dataclass(frozen=True, slots=True)
class WorkspaceRequestOpenRoot:
    """Open or register a root."""

    open_root: OpenRootRequest
    kind: Literal["openRoot"] = "openRoot"


@dataclass(frozen=True, slots=True)
class WorkspaceRequestCloseRoot:
    """Close a root handle."""

    close_root: CloseRootRequest
    kind: Literal["closeRoot"] = "closeRoot"


@dataclass(frozen=True, slots=True)
class WorkspaceRequestReloadRoot:
    """Reload a root."""

    reload_root: ReloadRootRequest
    kind: Literal["reloadRoot"] = "reloadRoot"


@dataclass(frozen=True, slots=True)
class WorkspaceRequestApplyFileOperation:
    """Apply a file operation to a root."""

    apply_file_operation: FileOperationRequest
    kind: Literal["applyFileOperation"] = "applyFileOperation"


@dataclass(frozen=True, slots=True)
class WorkspaceRequestApplySourceUpdate:
    """Apply a source update to a root."""

    apply_source_update: SourceUpdateRequest
    kind: Literal["applySourceUpdate"] = "applySourceUpdate"


@dataclass(frozen=True, slots=True)
class WorkspaceRequestStartWatch:
    """Start watching a root."""

    start_watch: WatchStartRequest
    kind: Literal["startWatch"] = "startWatch"


@dataclass(frozen=True, slots=True)
class WorkspaceRequestNextWatchBatch:
    """Receive and apply the next watch batch."""

    next_watch_batch: WatchNextRequest
    kind: Literal["nextWatchBatch"] = "nextWatchBatch"


@dataclass(frozen=True, slots=True)
class WorkspaceRequestStopWatch:
    """Stop watching a root."""

    stop_watch: WatchStopRequest
    kind: Literal["stopWatch"] = "stopWatch"


@dataclass(frozen=True, slots=True)
class WorkspaceRequestCheck:
    """Check source state."""

    """Root handle."""
    handle: RootId
    """Check input."""
    input: CheckInput
    kind: Literal["check"] = "check"


@dataclass(frozen=True, slots=True)
class WorkspaceRequestLint:
    """Lint source state."""

    """Root handle."""
    handle: RootId
    """Lint input."""
    input: LintInput
    kind: Literal["lint"] = "lint"


@dataclass(frozen=True, slots=True)
class WorkspaceRequestFormat:
    """Format source files or content."""

    """Root handle."""
    handle: RootId
    """Format input."""
    input: FormatInput
    kind: Literal["format"] = "format"


@dataclass(frozen=True, slots=True)
class WorkspaceRequestBuild:
    """Build target artifacts."""

    """Root handle."""
    handle: RootId
    """Build input."""
    input: BuildInput
    kind: Literal["build"] = "build"


@dataclass(frozen=True, slots=True)
class WorkspaceRequestRun:
    """Run a workspace target."""

    """Root handle."""
    handle: RootId
    """Run input."""
    input: RunInput
    kind: Literal["run"] = "run"


@dataclass(frozen=True, slots=True)
class WorkspaceRequestTest:
    """Run workspace tests."""

    """Root handle."""
    handle: RootId
    """Test input."""
    input: TestInput
    kind: Literal["test"] = "test"


@dataclass(frozen=True, slots=True)
class WorkspaceRequestDoc:
    """Generate documentation."""

    """Root handle."""
    handle: RootId
    """Documentation input."""
    input: DocInput
    kind: Literal["doc"] = "doc"


@dataclass(frozen=True, slots=True)
class WorkspaceRequestBench:
    """Run benchmarks."""

    """Root handle."""
    handle: RootId
    """Benchmark input."""
    input: BenchInput
    kind: Literal["bench"] = "bench"


@dataclass(frozen=True, slots=True)
class WorkspaceRequestInfo:
    """Return workspace information."""

    """Root handle."""
    handle: RootId
    """Information input."""
    input: InfoInput
    kind: Literal["info"] = "info"


@dataclass(frozen=True, slots=True)
class WorkspaceRequestTargets:
    """Return configured targets."""

    """Root handle."""
    handle: RootId
    """Targets input."""
    input: TargetsInput
    kind: Literal["targets"] = "targets"


@dataclass(frozen=True, slots=True)
class WorkspaceRequestCache:
    """Return cache locations."""

    """Root handle."""
    handle: RootId
    """Cache input."""
    input: CacheInput
    kind: Literal["cache"] = "cache"


@dataclass(frozen=True, slots=True)
class WorkspaceRequestSettings:
    """Return resolved settings."""

    """Root handle."""
    handle: RootId
    """Settings input."""
    input: SettingsInput
    kind: Literal["settings"] = "settings"


@dataclass(frozen=True, slots=True)
class WorkspaceRequestDoctor:
    """Return workspace health information."""

    """Root handle."""
    handle: RootId
    """Doctor input."""
    input: DoctorInput
    kind: Literal["doctor"] = "doctor"


@dataclass(frozen=True, slots=True)
class WorkspaceRequestTask:
    """Run workspace tasks."""

    """Root handle."""
    handle: RootId
    """Task input."""
    input: TaskInput
    kind: Literal["task"] = "task"


@dataclass(frozen=True, slots=True)
class WorkspaceRequestClean:
    """Clean generated state."""

    """Root handle."""
    handle: RootId
    """Clean input."""
    input: CleanInput
    kind: Literal["clean"] = "clean"


@dataclass(frozen=True, slots=True)
class WorkspaceRequestArtifact:
    """Return one artifact payload."""

    """Root handle."""
    handle: RootId
    """Artifact reference."""
    artifact: ArtifactReference
    kind: Literal["artifact"] = "artifact"


@dataclass(frozen=True, slots=True)
class WorkspaceRequestStore:
    """Store one content payload."""

    """Root handle."""
    handle: RootId
    """Content payload."""
    content: Content
    kind: Literal["store"] = "store"


@dataclass(frozen=True, slots=True)
class WorkspaceRequestLoad:
    """Load one content payload."""

    """Root handle."""
    handle: RootId
    """Content id."""
    content: ContentId
    kind: Literal["load"] = "load"


@dataclass(frozen=True, slots=True)
class WorkspaceRequestExport:
    """Materialize derived outputs."""

    """Root handle."""
    handle: RootId
    """Export request."""
    request: ExportRequest
    kind: Literal["export"] = "export"


@dataclass(frozen=True, slots=True)
class WorkspaceRequestQuery:
    """Execute a query."""

    query: WorkspaceQuery
    kind: Literal["query"] = "query"


"""Requests accepted by the workspace protocol."""
WorkspaceRequest: TypeAlias = (
    WorkspaceRequestHandshake
    | WorkspaceRequestPing
    | WorkspaceRequestCancel
    | WorkspaceRequestShutdown
    | WorkspaceRequestOpenRoot
    | WorkspaceRequestCloseRoot
    | WorkspaceRequestReloadRoot
    | WorkspaceRequestApplyFileOperation
    | WorkspaceRequestApplySourceUpdate
    | WorkspaceRequestStartWatch
    | WorkspaceRequestNextWatchBatch
    | WorkspaceRequestStopWatch
    | WorkspaceRequestCheck
    | WorkspaceRequestLint
    | WorkspaceRequestFormat
    | WorkspaceRequestBuild
    | WorkspaceRequestRun
    | WorkspaceRequestTest
    | WorkspaceRequestDoc
    | WorkspaceRequestBench
    | WorkspaceRequestInfo
    | WorkspaceRequestTargets
    | WorkspaceRequestCache
    | WorkspaceRequestSettings
    | WorkspaceRequestDoctor
    | WorkspaceRequestTask
    | WorkspaceRequestClean
    | WorkspaceRequestArtifact
    | WorkspaceRequestStore
    | WorkspaceRequestLoad
    | WorkspaceRequestExport
    | WorkspaceRequestQuery
)


def encode_workspace_request(writer: Writer, value: WorkspaceRequest) -> None:
    if value.kind == "handshake":
        writer.write_unsigned(0)
        destack._generated.protocol.handshake.encode_handshake_request(
            writer, value.handshake
        )
    elif value.kind == "ping":
        writer.write_unsigned(1)
    elif value.kind == "cancel":
        writer.write_unsigned(2)
        destack._generated.protocol.envelope.encode_request_id(writer, value.id)
    elif value.kind == "shutdown":
        writer.write_unsigned(3)
    elif value.kind == "openRoot":
        writer.write_unsigned(4)
        destack._generated.protocol.root.encode_open_root_request(
            writer, value.open_root
        )
    elif value.kind == "closeRoot":
        writer.write_unsigned(5)
        destack._generated.protocol.root.encode_close_root_request(
            writer, value.close_root
        )
    elif value.kind == "reloadRoot":
        writer.write_unsigned(6)
        destack._generated.protocol.root.encode_reload_root_request(
            writer, value.reload_root
        )
    elif value.kind == "applyFileOperation":
        writer.write_unsigned(7)
        destack._generated.protocol.root.encode_file_operation_request(
            writer, value.apply_file_operation
        )
    elif value.kind == "applySourceUpdate":
        writer.write_unsigned(8)
        destack._generated.protocol.root.encode_source_update_request(
            writer, value.apply_source_update
        )
    elif value.kind == "startWatch":
        writer.write_unsigned(9)
        destack._generated.protocol.watch.encode_watch_start_request(
            writer, value.start_watch
        )
    elif value.kind == "nextWatchBatch":
        writer.write_unsigned(10)
        destack._generated.protocol.watch.encode_watch_next_request(
            writer, value.next_watch_batch
        )
    elif value.kind == "stopWatch":
        writer.write_unsigned(11)
        destack._generated.protocol.watch.encode_watch_stop_request(
            writer, value.stop_watch
        )
    elif value.kind == "check":
        writer.write_unsigned(12)
        destack._generated.protocol.root.encode_root_id(writer, value.handle)
        destack._generated.protocol.workspace.command.check.encode_check_input(
            writer, value.input
        )
    elif value.kind == "lint":
        writer.write_unsigned(13)
        destack._generated.protocol.root.encode_root_id(writer, value.handle)
        destack._generated.protocol.workspace.command.check.encode_lint_input(
            writer, value.input
        )
    elif value.kind == "format":
        writer.write_unsigned(14)
        destack._generated.protocol.root.encode_root_id(writer, value.handle)
        destack._generated.protocol.workspace.command.format.encode_format_input(
            writer, value.input
        )
    elif value.kind == "build":
        writer.write_unsigned(15)
        destack._generated.protocol.root.encode_root_id(writer, value.handle)
        destack._generated.protocol.workspace.command.build.encode_build_input(
            writer, value.input
        )
    elif value.kind == "run":
        writer.write_unsigned(16)
        destack._generated.protocol.root.encode_root_id(writer, value.handle)
        destack._generated.protocol.workspace.command.run.encode_run_input(
            writer, value.input
        )
    elif value.kind == "test":
        writer.write_unsigned(17)
        destack._generated.protocol.root.encode_root_id(writer, value.handle)
        destack._generated.protocol.workspace.command.test.encode_test_input(
            writer, value.input
        )
    elif value.kind == "doc":
        writer.write_unsigned(18)
        destack._generated.protocol.root.encode_root_id(writer, value.handle)
        destack._generated.protocol.workspace.command.doc.encode_doc_input(
            writer, value.input
        )
    elif value.kind == "bench":
        writer.write_unsigned(19)
        destack._generated.protocol.root.encode_root_id(writer, value.handle)
        destack._generated.protocol.workspace.command.bench.encode_bench_input(
            writer, value.input
        )
    elif value.kind == "info":
        writer.write_unsigned(20)
        destack._generated.protocol.root.encode_root_id(writer, value.handle)
        destack._generated.protocol.workspace.command.info.encode_info_input(
            writer, value.input
        )
    elif value.kind == "targets":
        writer.write_unsigned(21)
        destack._generated.protocol.root.encode_root_id(writer, value.handle)
        destack._generated.protocol.workspace.command.targets.encode_targets_input(
            writer, value.input
        )
    elif value.kind == "cache":
        writer.write_unsigned(22)
        destack._generated.protocol.root.encode_root_id(writer, value.handle)
        destack._generated.protocol.workspace.command.cache.encode_cache_input(
            writer, value.input
        )
    elif value.kind == "settings":
        writer.write_unsigned(23)
        destack._generated.protocol.root.encode_root_id(writer, value.handle)
        destack._generated.protocol.workspace.command.settings.encode_settings_input(
            writer, value.input
        )
    elif value.kind == "doctor":
        writer.write_unsigned(24)
        destack._generated.protocol.root.encode_root_id(writer, value.handle)
        destack._generated.protocol.workspace.command.doctor.encode_doctor_input(
            writer, value.input
        )
    elif value.kind == "task":
        writer.write_unsigned(25)
        destack._generated.protocol.root.encode_root_id(writer, value.handle)
        destack._generated.protocol.workspace.command.task.encode_task_input(
            writer, value.input
        )
    elif value.kind == "clean":
        writer.write_unsigned(26)
        destack._generated.protocol.root.encode_root_id(writer, value.handle)
        destack._generated.protocol.workspace.command.clean.encode_clean_input(
            writer, value.input
        )
    elif value.kind == "artifact":
        writer.write_unsigned(27)
        destack._generated.protocol.root.encode_root_id(writer, value.handle)
        destack._generated.protocol.artifact.reference.encode_artifact_reference(
            writer, value.artifact
        )
    elif value.kind == "store":
        writer.write_unsigned(28)
        destack._generated.protocol.root.encode_root_id(writer, value.handle)
        destack._generated.protocol.source.file.model.file.encode_content(
            writer, value.content
        )
    elif value.kind == "load":
        writer.write_unsigned(29)
        destack._generated.protocol.root.encode_root_id(writer, value.handle)
        destack._generated.protocol.source.file.model.file.encode_content_id(
            writer, value.content
        )
    elif value.kind == "export":
        writer.write_unsigned(30)
        destack._generated.protocol.root.encode_root_id(writer, value.handle)
        destack._generated.protocol.workspace.artifact.export.encode_export_request(
            writer, value.request
        )
    elif value.kind == "query":
        writer.write_unsigned(31)
        destack._generated.protocol.query.model.encode_workspace_query(
            writer, value.query
        )
    else:
        raise SerdeError("unknown enum variant")


def decode_workspace_request(reader: Reader) -> WorkspaceRequest:
    variant = reader.read_number()

    if variant == 0:
        return WorkspaceRequestHandshake(
            handshake=destack._generated.protocol.handshake.decode_handshake_request(
                reader
            )
        )
    elif variant == 1:
        return WorkspaceRequestPing()
    elif variant == 2:
        field_0 = destack._generated.protocol.envelope.decode_request_id(reader)

        return WorkspaceRequestCancel(
            id=field_0,
        )
    elif variant == 3:
        return WorkspaceRequestShutdown()
    elif variant == 4:
        return WorkspaceRequestOpenRoot(
            open_root=destack._generated.protocol.root.decode_open_root_request(reader)
        )
    elif variant == 5:
        return WorkspaceRequestCloseRoot(
            close_root=destack._generated.protocol.root.decode_close_root_request(
                reader
            )
        )
    elif variant == 6:
        return WorkspaceRequestReloadRoot(
            reload_root=destack._generated.protocol.root.decode_reload_root_request(
                reader
            )
        )
    elif variant == 7:
        return WorkspaceRequestApplyFileOperation(
            apply_file_operation=destack._generated.protocol.root.decode_file_operation_request(
                reader
            )
        )
    elif variant == 8:
        return WorkspaceRequestApplySourceUpdate(
            apply_source_update=destack._generated.protocol.root.decode_source_update_request(
                reader
            )
        )
    elif variant == 9:
        return WorkspaceRequestStartWatch(
            start_watch=destack._generated.protocol.watch.decode_watch_start_request(
                reader
            )
        )
    elif variant == 10:
        return WorkspaceRequestNextWatchBatch(
            next_watch_batch=destack._generated.protocol.watch.decode_watch_next_request(
                reader
            )
        )
    elif variant == 11:
        return WorkspaceRequestStopWatch(
            stop_watch=destack._generated.protocol.watch.decode_watch_stop_request(
                reader
            )
        )
    elif variant == 12:
        field_0 = destack._generated.protocol.root.decode_root_id(reader)
        field_1 = (
            destack._generated.protocol.workspace.command.check.decode_check_input(
                reader
            )
        )

        return WorkspaceRequestCheck(
            handle=field_0,
            input=field_1,
        )
    elif variant == 13:
        field_0 = destack._generated.protocol.root.decode_root_id(reader)
        field_1 = destack._generated.protocol.workspace.command.check.decode_lint_input(
            reader
        )

        return WorkspaceRequestLint(
            handle=field_0,
            input=field_1,
        )
    elif variant == 14:
        field_0 = destack._generated.protocol.root.decode_root_id(reader)
        field_1 = (
            destack._generated.protocol.workspace.command.format.decode_format_input(
                reader
            )
        )

        return WorkspaceRequestFormat(
            handle=field_0,
            input=field_1,
        )
    elif variant == 15:
        field_0 = destack._generated.protocol.root.decode_root_id(reader)
        field_1 = (
            destack._generated.protocol.workspace.command.build.decode_build_input(
                reader
            )
        )

        return WorkspaceRequestBuild(
            handle=field_0,
            input=field_1,
        )
    elif variant == 16:
        field_0 = destack._generated.protocol.root.decode_root_id(reader)
        field_1 = destack._generated.protocol.workspace.command.run.decode_run_input(
            reader
        )

        return WorkspaceRequestRun(
            handle=field_0,
            input=field_1,
        )
    elif variant == 17:
        field_0 = destack._generated.protocol.root.decode_root_id(reader)
        field_1 = destack._generated.protocol.workspace.command.test.decode_test_input(
            reader
        )

        return WorkspaceRequestTest(
            handle=field_0,
            input=field_1,
        )
    elif variant == 18:
        field_0 = destack._generated.protocol.root.decode_root_id(reader)
        field_1 = destack._generated.protocol.workspace.command.doc.decode_doc_input(
            reader
        )

        return WorkspaceRequestDoc(
            handle=field_0,
            input=field_1,
        )
    elif variant == 19:
        field_0 = destack._generated.protocol.root.decode_root_id(reader)
        field_1 = (
            destack._generated.protocol.workspace.command.bench.decode_bench_input(
                reader
            )
        )

        return WorkspaceRequestBench(
            handle=field_0,
            input=field_1,
        )
    elif variant == 20:
        field_0 = destack._generated.protocol.root.decode_root_id(reader)
        field_1 = destack._generated.protocol.workspace.command.info.decode_info_input(
            reader
        )

        return WorkspaceRequestInfo(
            handle=field_0,
            input=field_1,
        )
    elif variant == 21:
        field_0 = destack._generated.protocol.root.decode_root_id(reader)
        field_1 = (
            destack._generated.protocol.workspace.command.targets.decode_targets_input(
                reader
            )
        )

        return WorkspaceRequestTargets(
            handle=field_0,
            input=field_1,
        )
    elif variant == 22:
        field_0 = destack._generated.protocol.root.decode_root_id(reader)
        field_1 = (
            destack._generated.protocol.workspace.command.cache.decode_cache_input(
                reader
            )
        )

        return WorkspaceRequestCache(
            handle=field_0,
            input=field_1,
        )
    elif variant == 23:
        field_0 = destack._generated.protocol.root.decode_root_id(reader)
        field_1 = destack._generated.protocol.workspace.command.settings.decode_settings_input(
            reader
        )

        return WorkspaceRequestSettings(
            handle=field_0,
            input=field_1,
        )
    elif variant == 24:
        field_0 = destack._generated.protocol.root.decode_root_id(reader)
        field_1 = (
            destack._generated.protocol.workspace.command.doctor.decode_doctor_input(
                reader
            )
        )

        return WorkspaceRequestDoctor(
            handle=field_0,
            input=field_1,
        )
    elif variant == 25:
        field_0 = destack._generated.protocol.root.decode_root_id(reader)
        field_1 = destack._generated.protocol.workspace.command.task.decode_task_input(
            reader
        )

        return WorkspaceRequestTask(
            handle=field_0,
            input=field_1,
        )
    elif variant == 26:
        field_0 = destack._generated.protocol.root.decode_root_id(reader)
        field_1 = (
            destack._generated.protocol.workspace.command.clean.decode_clean_input(
                reader
            )
        )

        return WorkspaceRequestClean(
            handle=field_0,
            input=field_1,
        )
    elif variant == 27:
        field_0 = destack._generated.protocol.root.decode_root_id(reader)
        field_1 = (
            destack._generated.protocol.artifact.reference.decode_artifact_reference(
                reader
            )
        )

        return WorkspaceRequestArtifact(
            handle=field_0,
            artifact=field_1,
        )
    elif variant == 28:
        field_0 = destack._generated.protocol.root.decode_root_id(reader)
        field_1 = destack._generated.protocol.source.file.model.file.decode_content(
            reader
        )

        return WorkspaceRequestStore(
            handle=field_0,
            content=field_1,
        )
    elif variant == 29:
        field_0 = destack._generated.protocol.root.decode_root_id(reader)
        field_1 = destack._generated.protocol.source.file.model.file.decode_content_id(
            reader
        )

        return WorkspaceRequestLoad(
            handle=field_0,
            content=field_1,
        )
    elif variant == 30:
        field_0 = destack._generated.protocol.root.decode_root_id(reader)
        field_1 = (
            destack._generated.protocol.workspace.artifact.export.decode_export_request(
                reader
            )
        )

        return WorkspaceRequestExport(
            handle=field_0,
            request=field_1,
        )
    elif variant == 31:
        return WorkspaceRequestQuery(
            query=destack._generated.protocol.query.model.decode_workspace_query(reader)
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


__all__ = [
    "WorkspaceRequest",
    "encode_workspace_request",
    "decode_workspace_request",
    "WorkspaceRequestHandshake",
    "WorkspaceRequestPing",
    "WorkspaceRequestCancel",
    "WorkspaceRequestShutdown",
    "WorkspaceRequestOpenRoot",
    "WorkspaceRequestCloseRoot",
    "WorkspaceRequestReloadRoot",
    "WorkspaceRequestApplyFileOperation",
    "WorkspaceRequestApplySourceUpdate",
    "WorkspaceRequestStartWatch",
    "WorkspaceRequestNextWatchBatch",
    "WorkspaceRequestStopWatch",
    "WorkspaceRequestCheck",
    "WorkspaceRequestLint",
    "WorkspaceRequestFormat",
    "WorkspaceRequestBuild",
    "WorkspaceRequestRun",
    "WorkspaceRequestTest",
    "WorkspaceRequestDoc",
    "WorkspaceRequestBench",
    "WorkspaceRequestInfo",
    "WorkspaceRequestTargets",
    "WorkspaceRequestCache",
    "WorkspaceRequestSettings",
    "WorkspaceRequestDoctor",
    "WorkspaceRequestTask",
    "WorkspaceRequestClean",
    "WorkspaceRequestArtifact",
    "WorkspaceRequestStore",
    "WorkspaceRequestLoad",
    "WorkspaceRequestExport",
    "WorkspaceRequestQuery",
]
