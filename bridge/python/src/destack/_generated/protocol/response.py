# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.artifact.core.version
import destack._generated.protocol.envelope
import destack._generated.protocol.error
import destack._generated.protocol.handshake
import destack._generated.protocol.query.model
import destack._generated.protocol.root
import destack._generated.protocol.source.file.model.file
import destack._generated.protocol.watch
import destack._generated.protocol.workspace.artifact.export
import destack._generated.protocol.workspace.command.output

if TYPE_CHECKING:
    from destack._generated.protocol.artifact.core.version import (
        ArtifactVersion,
    )

    from destack._generated.protocol.envelope import (
        RequestId,
    )

    from destack._generated.protocol.error import (
        ProtocolError,
    )

    from destack._generated.protocol.handshake import (
        HandshakeResponse,
    )

    from destack._generated.protocol.query.model import (
        WorkspaceQueryResponse,
    )

    from destack._generated.protocol.root import (
        FileOperationResponse,
        RootClosedResponse,
        RootOpenedResponse,
        RootReloadResponse,
        SourceUpdateResponse,
    )

    from destack._generated.protocol.source.file.model.file import (
        Content,
        ContentId,
    )

    from destack._generated.protocol.watch import (
        WatchBatchResponse,
        WatchStartedResponse,
        WatchStoppedResponse,
    )

    from destack._generated.protocol.workspace.artifact.export import (
        ExportResult,
    )

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


@dataclass(frozen=True, slots=True)
class WorkspaceResponseHandshake:
    """Successful handshake response."""

    handshake: HandshakeResponse
    kind: Literal["handshake"] = "handshake"


@dataclass(frozen=True, slots=True)
class WorkspaceResponsePong:
    """Response to ping."""

    kind: Literal["pong"] = "pong"


@dataclass(frozen=True, slots=True)
class WorkspaceResponseCanceled:
    """Response to request cancellation."""

    id: RequestId
    kind: Literal["canceled"] = "canceled"


@dataclass(frozen=True, slots=True)
class WorkspaceResponseShutdownAck:
    """Response to shutdown request."""

    kind: Literal["shutdownAck"] = "shutdownAck"


@dataclass(frozen=True, slots=True)
class WorkspaceResponseRootOpened:
    """Root open response."""

    root_opened: RootOpenedResponse
    kind: Literal["rootOpened"] = "rootOpened"


@dataclass(frozen=True, slots=True)
class WorkspaceResponseRootClosed:
    """Root close response."""

    root_closed: RootClosedResponse
    kind: Literal["rootClosed"] = "rootClosed"


@dataclass(frozen=True, slots=True)
class WorkspaceResponseRootReloaded:
    """Root reload response."""

    root_reloaded: RootReloadResponse
    kind: Literal["rootReloaded"] = "rootReloaded"


@dataclass(frozen=True, slots=True)
class WorkspaceResponseFileOperationApplied:
    """File operation response."""

    file_operation_applied: FileOperationResponse
    kind: Literal["fileOperationApplied"] = "fileOperationApplied"


@dataclass(frozen=True, slots=True)
class WorkspaceResponseSourceUpdated:
    """Source update response."""

    source_updated: SourceUpdateResponse
    kind: Literal["sourceUpdated"] = "sourceUpdated"


@dataclass(frozen=True, slots=True)
class WorkspaceResponseWatchStarted:
    """Watch start response."""

    watch_started: WatchStartedResponse
    kind: Literal["watchStarted"] = "watchStarted"


@dataclass(frozen=True, slots=True)
class WorkspaceResponseWatchBatchReady:
    """Watch batch response."""

    watch_batch_ready: WatchBatchResponse
    kind: Literal["watchBatchReady"] = "watchBatchReady"


@dataclass(frozen=True, slots=True)
class WorkspaceResponseWatchStopped:
    """Watch stop response."""

    watch_stopped: WatchStoppedResponse
    kind: Literal["watchStopped"] = "watchStopped"


@dataclass(frozen=True, slots=True)
class WorkspaceResponseCheck:
    """Check response."""

    check: CheckOutput
    kind: Literal["check"] = "check"


@dataclass(frozen=True, slots=True)
class WorkspaceResponseLint:
    """Lint response."""

    lint: LintOutput
    kind: Literal["lint"] = "lint"


@dataclass(frozen=True, slots=True)
class WorkspaceResponseFormat:
    """Format response."""

    format: FormatOutput
    kind: Literal["format"] = "format"


@dataclass(frozen=True, slots=True)
class WorkspaceResponseBuild:
    """Build response."""

    build: BuildOutput
    kind: Literal["build"] = "build"


@dataclass(frozen=True, slots=True)
class WorkspaceResponseRun:
    """Run response."""

    run: RunOutput
    kind: Literal["run"] = "run"


@dataclass(frozen=True, slots=True)
class WorkspaceResponseTest:
    """Test response."""

    test: TestOutput
    kind: Literal["test"] = "test"


@dataclass(frozen=True, slots=True)
class WorkspaceResponseDoc:
    """Documentation response."""

    doc: DocOutput
    kind: Literal["doc"] = "doc"


@dataclass(frozen=True, slots=True)
class WorkspaceResponseBench:
    """Benchmark response."""

    bench: BenchOutput
    kind: Literal["bench"] = "bench"


@dataclass(frozen=True, slots=True)
class WorkspaceResponseInfo:
    """Information response."""

    info: InfoOutput
    kind: Literal["info"] = "info"


@dataclass(frozen=True, slots=True)
class WorkspaceResponseTargets:
    """Targets response."""

    targets: TargetsOutput
    kind: Literal["targets"] = "targets"


@dataclass(frozen=True, slots=True)
class WorkspaceResponseCache:
    """Cache response."""

    cache: CacheOutput
    kind: Literal["cache"] = "cache"


@dataclass(frozen=True, slots=True)
class WorkspaceResponseSettings:
    """Settings response."""

    settings: SettingsOutput
    kind: Literal["settings"] = "settings"


@dataclass(frozen=True, slots=True)
class WorkspaceResponseDoctor:
    """Doctor response."""

    doctor: DoctorOutput
    kind: Literal["doctor"] = "doctor"


@dataclass(frozen=True, slots=True)
class WorkspaceResponseTask:
    """Task response."""

    task: TaskOutput
    kind: Literal["task"] = "task"


@dataclass(frozen=True, slots=True)
class WorkspaceResponseClean:
    """Clean response."""

    clean: CleanOutput
    kind: Literal["clean"] = "clean"


@dataclass(frozen=True, slots=True)
class WorkspaceResponseArtifactResult:
    """Artifact blob response."""

    artifact_result: ArtifactBlob
    kind: Literal["artifactResult"] = "artifactResult"


@dataclass(frozen=True, slots=True)
class WorkspaceResponseStoreResult:
    """Store response."""

    store_result: ContentId
    kind: Literal["storeResult"] = "storeResult"


@dataclass(frozen=True, slots=True)
class WorkspaceResponseLoadResult:
    """Content payload response."""

    load_result: Content
    kind: Literal["loadResult"] = "loadResult"


@dataclass(frozen=True, slots=True)
class WorkspaceResponseExportResult:
    """Export response."""

    export_result: ExportResult
    kind: Literal["exportResult"] = "exportResult"


@dataclass(frozen=True, slots=True)
class WorkspaceResponseQueryResult:
    """Query response."""

    query_result: WorkspaceQueryResponse
    kind: Literal["queryResult"] = "queryResult"


@dataclass(frozen=True, slots=True)
class WorkspaceResponseError:
    """Error response."""

    error: ProtocolError
    kind: Literal["error"] = "error"


"""Responses emitted by the workspace protocol."""
WorkspaceResponse: TypeAlias = (
    WorkspaceResponseHandshake
    | WorkspaceResponsePong
    | WorkspaceResponseCanceled
    | WorkspaceResponseShutdownAck
    | WorkspaceResponseRootOpened
    | WorkspaceResponseRootClosed
    | WorkspaceResponseRootReloaded
    | WorkspaceResponseFileOperationApplied
    | WorkspaceResponseSourceUpdated
    | WorkspaceResponseWatchStarted
    | WorkspaceResponseWatchBatchReady
    | WorkspaceResponseWatchStopped
    | WorkspaceResponseCheck
    | WorkspaceResponseLint
    | WorkspaceResponseFormat
    | WorkspaceResponseBuild
    | WorkspaceResponseRun
    | WorkspaceResponseTest
    | WorkspaceResponseDoc
    | WorkspaceResponseBench
    | WorkspaceResponseInfo
    | WorkspaceResponseTargets
    | WorkspaceResponseCache
    | WorkspaceResponseSettings
    | WorkspaceResponseDoctor
    | WorkspaceResponseTask
    | WorkspaceResponseClean
    | WorkspaceResponseArtifactResult
    | WorkspaceResponseStoreResult
    | WorkspaceResponseLoadResult
    | WorkspaceResponseExportResult
    | WorkspaceResponseQueryResult
    | WorkspaceResponseError
)


def encode_workspace_response(writer: Writer, value: WorkspaceResponse) -> None:
    if value.kind == "handshake":
        writer.write_unsigned(0)
        destack._generated.protocol.handshake.encode_handshake_response(
            writer, value.handshake
        )
    elif value.kind == "pong":
        writer.write_unsigned(1)
    elif value.kind == "canceled":
        writer.write_unsigned(2)
        destack._generated.protocol.envelope.encode_request_id(writer, value.id)
    elif value.kind == "shutdownAck":
        writer.write_unsigned(3)
    elif value.kind == "rootOpened":
        writer.write_unsigned(4)
        destack._generated.protocol.root.encode_root_opened_response(
            writer, value.root_opened
        )
    elif value.kind == "rootClosed":
        writer.write_unsigned(5)
        destack._generated.protocol.root.encode_root_closed_response(
            writer, value.root_closed
        )
    elif value.kind == "rootReloaded":
        writer.write_unsigned(6)
        destack._generated.protocol.root.encode_root_reload_response(
            writer, value.root_reloaded
        )
    elif value.kind == "fileOperationApplied":
        writer.write_unsigned(7)
        destack._generated.protocol.root.encode_file_operation_response(
            writer, value.file_operation_applied
        )
    elif value.kind == "sourceUpdated":
        writer.write_unsigned(8)
        destack._generated.protocol.root.encode_source_update_response(
            writer, value.source_updated
        )
    elif value.kind == "watchStarted":
        writer.write_unsigned(9)
        destack._generated.protocol.watch.encode_watch_started_response(
            writer, value.watch_started
        )
    elif value.kind == "watchBatchReady":
        writer.write_unsigned(10)
        destack._generated.protocol.watch.encode_watch_batch_response(
            writer, value.watch_batch_ready
        )
    elif value.kind == "watchStopped":
        writer.write_unsigned(11)
        destack._generated.protocol.watch.encode_watch_stopped_response(
            writer, value.watch_stopped
        )
    elif value.kind == "check":
        writer.write_unsigned(12)
        destack._generated.protocol.workspace.command.output.encode_check_output(
            writer, value.check
        )
    elif value.kind == "lint":
        writer.write_unsigned(13)
        destack._generated.protocol.workspace.command.output.encode_lint_output(
            writer, value.lint
        )
    elif value.kind == "format":
        writer.write_unsigned(14)
        destack._generated.protocol.workspace.command.output.encode_format_output(
            writer, value.format
        )
    elif value.kind == "build":
        writer.write_unsigned(15)
        destack._generated.protocol.workspace.command.output.encode_build_output(
            writer, value.build
        )
    elif value.kind == "run":
        writer.write_unsigned(16)
        destack._generated.protocol.workspace.command.output.encode_run_output(
            writer, value.run
        )
    elif value.kind == "test":
        writer.write_unsigned(17)
        destack._generated.protocol.workspace.command.output.encode_test_output(
            writer, value.test
        )
    elif value.kind == "doc":
        writer.write_unsigned(18)
        destack._generated.protocol.workspace.command.output.encode_doc_output(
            writer, value.doc
        )
    elif value.kind == "bench":
        writer.write_unsigned(19)
        destack._generated.protocol.workspace.command.output.encode_bench_output(
            writer, value.bench
        )
    elif value.kind == "info":
        writer.write_unsigned(20)
        destack._generated.protocol.workspace.command.output.encode_info_output(
            writer, value.info
        )
    elif value.kind == "targets":
        writer.write_unsigned(21)
        destack._generated.protocol.workspace.command.output.encode_targets_output(
            writer, value.targets
        )
    elif value.kind == "cache":
        writer.write_unsigned(22)
        destack._generated.protocol.workspace.command.output.encode_cache_output(
            writer, value.cache
        )
    elif value.kind == "settings":
        writer.write_unsigned(23)
        destack._generated.protocol.workspace.command.output.encode_settings_output(
            writer, value.settings
        )
    elif value.kind == "doctor":
        writer.write_unsigned(24)
        destack._generated.protocol.workspace.command.output.encode_doctor_output(
            writer, value.doctor
        )
    elif value.kind == "task":
        writer.write_unsigned(25)
        destack._generated.protocol.workspace.command.output.encode_task_output(
            writer, value.task
        )
    elif value.kind == "clean":
        writer.write_unsigned(26)
        destack._generated.protocol.workspace.command.output.encode_clean_output(
            writer, value.clean
        )
    elif value.kind == "artifactResult":
        writer.write_unsigned(27)
        encode_artifact_blob(writer, value.artifact_result)
    elif value.kind == "storeResult":
        writer.write_unsigned(28)
        destack._generated.protocol.source.file.model.file.encode_content_id(
            writer, value.store_result
        )
    elif value.kind == "loadResult":
        writer.write_unsigned(29)
        destack._generated.protocol.source.file.model.file.encode_content(
            writer, value.load_result
        )
    elif value.kind == "exportResult":
        writer.write_unsigned(30)
        destack._generated.protocol.workspace.artifact.export.encode_export_result(
            writer, value.export_result
        )
    elif value.kind == "queryResult":
        writer.write_unsigned(31)
        destack._generated.protocol.query.model.encode_workspace_query_response(
            writer, value.query_result
        )
    elif value.kind == "error":
        writer.write_unsigned(32)
        destack._generated.protocol.error.encode_protocol_error(writer, value.error)
    else:
        raise SerdeError("unknown enum variant")


def decode_workspace_response(reader: Reader) -> WorkspaceResponse:
    variant = reader.read_number()

    if variant == 0:
        return WorkspaceResponseHandshake(
            handshake=destack._generated.protocol.handshake.decode_handshake_response(
                reader
            )
        )
    elif variant == 1:
        return WorkspaceResponsePong()
    elif variant == 2:
        field_0 = destack._generated.protocol.envelope.decode_request_id(reader)

        return WorkspaceResponseCanceled(
            id=field_0,
        )
    elif variant == 3:
        return WorkspaceResponseShutdownAck()
    elif variant == 4:
        return WorkspaceResponseRootOpened(
            root_opened=destack._generated.protocol.root.decode_root_opened_response(
                reader
            )
        )
    elif variant == 5:
        return WorkspaceResponseRootClosed(
            root_closed=destack._generated.protocol.root.decode_root_closed_response(
                reader
            )
        )
    elif variant == 6:
        return WorkspaceResponseRootReloaded(
            root_reloaded=destack._generated.protocol.root.decode_root_reload_response(
                reader
            )
        )
    elif variant == 7:
        return WorkspaceResponseFileOperationApplied(
            file_operation_applied=destack._generated.protocol.root.decode_file_operation_response(
                reader
            )
        )
    elif variant == 8:
        return WorkspaceResponseSourceUpdated(
            source_updated=destack._generated.protocol.root.decode_source_update_response(
                reader
            )
        )
    elif variant == 9:
        return WorkspaceResponseWatchStarted(
            watch_started=destack._generated.protocol.watch.decode_watch_started_response(
                reader
            )
        )
    elif variant == 10:
        return WorkspaceResponseWatchBatchReady(
            watch_batch_ready=destack._generated.protocol.watch.decode_watch_batch_response(
                reader
            )
        )
    elif variant == 11:
        return WorkspaceResponseWatchStopped(
            watch_stopped=destack._generated.protocol.watch.decode_watch_stopped_response(
                reader
            )
        )
    elif variant == 12:
        return WorkspaceResponseCheck(
            check=destack._generated.protocol.workspace.command.output.decode_check_output(
                reader
            )
        )
    elif variant == 13:
        return WorkspaceResponseLint(
            lint=destack._generated.protocol.workspace.command.output.decode_lint_output(
                reader
            )
        )
    elif variant == 14:
        return WorkspaceResponseFormat(
            format=destack._generated.protocol.workspace.command.output.decode_format_output(
                reader
            )
        )
    elif variant == 15:
        return WorkspaceResponseBuild(
            build=destack._generated.protocol.workspace.command.output.decode_build_output(
                reader
            )
        )
    elif variant == 16:
        return WorkspaceResponseRun(
            run=destack._generated.protocol.workspace.command.output.decode_run_output(
                reader
            )
        )
    elif variant == 17:
        return WorkspaceResponseTest(
            test=destack._generated.protocol.workspace.command.output.decode_test_output(
                reader
            )
        )
    elif variant == 18:
        return WorkspaceResponseDoc(
            doc=destack._generated.protocol.workspace.command.output.decode_doc_output(
                reader
            )
        )
    elif variant == 19:
        return WorkspaceResponseBench(
            bench=destack._generated.protocol.workspace.command.output.decode_bench_output(
                reader
            )
        )
    elif variant == 20:
        return WorkspaceResponseInfo(
            info=destack._generated.protocol.workspace.command.output.decode_info_output(
                reader
            )
        )
    elif variant == 21:
        return WorkspaceResponseTargets(
            targets=destack._generated.protocol.workspace.command.output.decode_targets_output(
                reader
            )
        )
    elif variant == 22:
        return WorkspaceResponseCache(
            cache=destack._generated.protocol.workspace.command.output.decode_cache_output(
                reader
            )
        )
    elif variant == 23:
        return WorkspaceResponseSettings(
            settings=destack._generated.protocol.workspace.command.output.decode_settings_output(
                reader
            )
        )
    elif variant == 24:
        return WorkspaceResponseDoctor(
            doctor=destack._generated.protocol.workspace.command.output.decode_doctor_output(
                reader
            )
        )
    elif variant == 25:
        return WorkspaceResponseTask(
            task=destack._generated.protocol.workspace.command.output.decode_task_output(
                reader
            )
        )
    elif variant == 26:
        return WorkspaceResponseClean(
            clean=destack._generated.protocol.workspace.command.output.decode_clean_output(
                reader
            )
        )
    elif variant == 27:
        return WorkspaceResponseArtifactResult(
            artifact_result=decode_artifact_blob(reader)
        )
    elif variant == 28:
        return WorkspaceResponseStoreResult(
            store_result=destack._generated.protocol.source.file.model.file.decode_content_id(
                reader
            )
        )
    elif variant == 29:
        return WorkspaceResponseLoadResult(
            load_result=destack._generated.protocol.source.file.model.file.decode_content(
                reader
            )
        )
    elif variant == 30:
        return WorkspaceResponseExportResult(
            export_result=destack._generated.protocol.workspace.artifact.export.decode_export_result(
                reader
            )
        )
    elif variant == 31:
        return WorkspaceResponseQueryResult(
            query_result=destack._generated.protocol.query.model.decode_workspace_query_response(
                reader
            )
        )
    elif variant == 32:
        return WorkspaceResponseError(
            error=destack._generated.protocol.error.decode_protocol_error(reader)
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class ArtifactBlob:
    """Serialized artifact payload returned by the workspace protocol."""

    """Exact artifact version."""
    version: ArtifactVersion
    """Serialized artifact payload bytes."""
    bytes: bytes | bytearray | Sequence[int]


def encode_artifact_blob(writer: Writer, value: ArtifactBlob) -> None:
    destack._generated.protocol.artifact.core.version.encode_artifact_version(
        writer, value.version
    )
    writer.write_byte_slice(value.bytes)


def decode_artifact_blob(reader: Reader) -> ArtifactBlob:
    field_0 = destack._generated.protocol.artifact.core.version.decode_artifact_version(
        reader
    )
    field_1 = reader.read_byte_slice()

    return ArtifactBlob(
        version=field_0,
        bytes=field_1,
    )


__all__ = [
    "WorkspaceResponse",
    "encode_workspace_response",
    "decode_workspace_response",
    "WorkspaceResponseHandshake",
    "WorkspaceResponsePong",
    "WorkspaceResponseCanceled",
    "WorkspaceResponseShutdownAck",
    "WorkspaceResponseRootOpened",
    "WorkspaceResponseRootClosed",
    "WorkspaceResponseRootReloaded",
    "WorkspaceResponseFileOperationApplied",
    "WorkspaceResponseSourceUpdated",
    "WorkspaceResponseWatchStarted",
    "WorkspaceResponseWatchBatchReady",
    "WorkspaceResponseWatchStopped",
    "WorkspaceResponseCheck",
    "WorkspaceResponseLint",
    "WorkspaceResponseFormat",
    "WorkspaceResponseBuild",
    "WorkspaceResponseRun",
    "WorkspaceResponseTest",
    "WorkspaceResponseDoc",
    "WorkspaceResponseBench",
    "WorkspaceResponseInfo",
    "WorkspaceResponseTargets",
    "WorkspaceResponseCache",
    "WorkspaceResponseSettings",
    "WorkspaceResponseDoctor",
    "WorkspaceResponseTask",
    "WorkspaceResponseClean",
    "WorkspaceResponseArtifactResult",
    "WorkspaceResponseStoreResult",
    "WorkspaceResponseLoadResult",
    "WorkspaceResponseExportResult",
    "WorkspaceResponseQueryResult",
    "WorkspaceResponseError",
    "ArtifactBlob",
    "encode_artifact_blob",
    "decode_artifact_blob",
]
