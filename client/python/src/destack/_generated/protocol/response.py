# generated client target, do not edit

from __future__ import annotations

import builtins

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    bytes_from_json,
    bytes_to_json,
    json_field,
    json_object,
    json_string,
)

import destack._generated.artifact.core.version
import destack._generated.protocol.envelope
import destack._generated.protocol.error
import destack._generated.protocol.handshake
import destack._generated.protocol.query
import destack._generated.protocol.root
import destack._generated.protocol.watch
import destack._generated.protocol.workspace.artifact.export
import destack._generated.protocol.workspace.command.output
import destack._generated.source.file.model.file


@dataclass(frozen=True, slots=True)
class WorkspaceResponseHandshake:
    """Successful handshake response."""

    handshake: destack._generated.protocol.handshake.HandshakeResponse
    kind: typing.Literal["handshake"] = "handshake"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceResponsePong:
    """Response to ping."""

    kind: typing.Literal["pong"] = "pong"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceResponseCanceled:
    """Response to request cancellation."""

    id: destack._generated.protocol.envelope.RequestId
    kind: typing.Literal["canceled"] = "canceled"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceResponseShutdownAck:
    """Response to shutdown request."""

    kind: typing.Literal["shutdownAck"] = "shutdownAck"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceResponseRootOpened:
    """Root open response."""

    root_opened: destack._generated.protocol.root.RootOpenedResponse
    kind: typing.Literal["rootOpened"] = "rootOpened"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceResponseRootClosed:
    """Root close response."""

    root_closed: destack._generated.protocol.root.RootClosedResponse
    kind: typing.Literal["rootClosed"] = "rootClosed"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceResponseRootReloaded:
    """Root reload response."""

    root_reloaded: destack._generated.protocol.root.RootReloadResponse
    kind: typing.Literal["rootReloaded"] = "rootReloaded"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceResponseFileOperationApplied:
    """File operation response."""

    file_operation_applied: destack._generated.protocol.root.FileOperationResponse
    kind: typing.Literal["fileOperationApplied"] = "fileOperationApplied"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceResponseSourceUpdated:
    """Source update response."""

    source_updated: destack._generated.protocol.root.SourceUpdateResponse
    kind: typing.Literal["sourceUpdated"] = "sourceUpdated"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceResponseWatchStarted:
    """Watch start response."""

    watch_started: destack._generated.protocol.watch.WatchStartedResponse
    kind: typing.Literal["watchStarted"] = "watchStarted"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceResponseWatchBatchReady:
    """Watch batch response."""

    watch_batch_ready: destack._generated.protocol.watch.WatchBatchResponse
    kind: typing.Literal["watchBatchReady"] = "watchBatchReady"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceResponseWatchStopped:
    """Watch stop response."""

    watch_stopped: destack._generated.protocol.watch.WatchStoppedResponse
    kind: typing.Literal["watchStopped"] = "watchStopped"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceResponseCheck:
    """Check response."""

    check: destack._generated.protocol.workspace.command.output.CheckOutput
    kind: typing.Literal["check"] = "check"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceResponseLint:
    """Lint response."""

    lint: destack._generated.protocol.workspace.command.output.LintOutput
    kind: typing.Literal["lint"] = "lint"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceResponseFormat:
    """Format response."""

    format: destack._generated.protocol.workspace.command.output.FormatOutput
    kind: typing.Literal["format"] = "format"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceResponseBuild:
    """Build response."""

    build: destack._generated.protocol.workspace.command.output.BuildOutput
    kind: typing.Literal["build"] = "build"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceResponseRun:
    """Run response."""

    run: destack._generated.protocol.workspace.command.output.RunOutput
    kind: typing.Literal["run"] = "run"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceResponseTest:
    """Test response."""

    test: destack._generated.protocol.workspace.command.output.TestOutput
    kind: typing.Literal["test"] = "test"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceResponseDoc:
    """Documentation response."""

    doc: destack._generated.protocol.workspace.command.output.DocOutput
    kind: typing.Literal["doc"] = "doc"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceResponseBench:
    """Benchmark response."""

    bench: destack._generated.protocol.workspace.command.output.BenchOutput
    kind: typing.Literal["bench"] = "bench"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceResponseInfo:
    """Information response."""

    info: destack._generated.protocol.workspace.command.output.InfoOutput
    kind: typing.Literal["info"] = "info"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceResponseTargets:
    """Targets response."""

    targets: destack._generated.protocol.workspace.command.output.TargetsOutput
    kind: typing.Literal["targets"] = "targets"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceResponseCache:
    """Cache response."""

    cache: destack._generated.protocol.workspace.command.output.CacheOutput
    kind: typing.Literal["cache"] = "cache"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceResponseSettings:
    """Settings response."""

    settings: destack._generated.protocol.workspace.command.output.SettingsOutput
    kind: typing.Literal["settings"] = "settings"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceResponseDoctor:
    """Doctor response."""

    doctor: destack._generated.protocol.workspace.command.output.DoctorOutput
    kind: typing.Literal["doctor"] = "doctor"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceResponseTask:
    """Task response."""

    task: destack._generated.protocol.workspace.command.output.TaskOutput
    kind: typing.Literal["task"] = "task"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceResponseClean:
    """Clean response."""

    clean: destack._generated.protocol.workspace.command.output.CleanOutput
    kind: typing.Literal["clean"] = "clean"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceResponseArtifactResult:
    """Artifact blob response."""

    artifact_result: ArtifactBlob
    kind: typing.Literal["artifactResult"] = "artifactResult"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceResponseStoreResult:
    """Store response."""

    store_result: destack._generated.source.file.model.file.ContentId
    kind: typing.Literal["storeResult"] = "storeResult"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceResponseLoadResult:
    """Content payload response."""

    load_result: destack._generated.source.file.model.file.Content
    kind: typing.Literal["loadResult"] = "loadResult"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceResponseExportResult:
    """Export response."""

    export_result: destack._generated.protocol.workspace.artifact.export.ExportResult
    kind: typing.Literal["exportResult"] = "exportResult"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceResponseQueryResult:
    """Query response."""

    query_result: destack._generated.protocol.query.WorkspaceQueryResponse
    kind: typing.Literal["queryResult"] = "queryResult"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceResponseError:
    """Error response."""

    error: destack._generated.protocol.error.ProtocolError
    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_response(self)


"""Responses emitted by the workspace protocol."""
WorkspaceResponse: typing.TypeAlias = (
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


def encode_workspace_response(writer: BinaryWriter, value: WorkspaceResponse) -> None:
    """Encode one WorkspaceResponse."""
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
        destack._generated.source.file.model.file.encode_content_id(
            writer, value.store_result
        )
    elif value.kind == "loadResult":
        writer.write_unsigned(29)
        destack._generated.source.file.model.file.encode_content(
            writer, value.load_result
        )
    elif value.kind == "exportResult":
        writer.write_unsigned(30)
        destack._generated.protocol.workspace.artifact.export.encode_export_result(
            writer, value.export_result
        )
    elif value.kind == "queryResult":
        writer.write_unsigned(31)
        destack._generated.protocol.query.encode_workspace_query_response(
            writer, value.query_result
        )
    elif value.kind == "error":
        writer.write_unsigned(32)
        destack._generated.protocol.error.encode_protocol_error(writer, value.error)
    else:
        raise SerdeError("unknown enum variant")


def decode_workspace_response(reader: BinaryReader) -> WorkspaceResponse:
    """Decode one WorkspaceResponse."""
    variant = reader.read_number()

    if variant == 0:
        handshake = destack._generated.protocol.handshake.decode_handshake_response(
            reader
        )

        return WorkspaceResponseHandshake(handshake=handshake)
    elif variant == 1:
        return WorkspaceResponsePong()
    elif variant == 2:
        id = destack._generated.protocol.envelope.decode_request_id(reader)

        return WorkspaceResponseCanceled(
            id=id,
        )
    elif variant == 3:
        return WorkspaceResponseShutdownAck()
    elif variant == 4:
        root_opened = destack._generated.protocol.root.decode_root_opened_response(
            reader
        )

        return WorkspaceResponseRootOpened(root_opened=root_opened)
    elif variant == 5:
        root_closed = destack._generated.protocol.root.decode_root_closed_response(
            reader
        )

        return WorkspaceResponseRootClosed(root_closed=root_closed)
    elif variant == 6:
        root_reloaded = destack._generated.protocol.root.decode_root_reload_response(
            reader
        )

        return WorkspaceResponseRootReloaded(root_reloaded=root_reloaded)
    elif variant == 7:
        file_operation_applied = (
            destack._generated.protocol.root.decode_file_operation_response(reader)
        )

        return WorkspaceResponseFileOperationApplied(
            file_operation_applied=file_operation_applied
        )
    elif variant == 8:
        source_updated = destack._generated.protocol.root.decode_source_update_response(
            reader
        )

        return WorkspaceResponseSourceUpdated(source_updated=source_updated)
    elif variant == 9:
        watch_started = destack._generated.protocol.watch.decode_watch_started_response(
            reader
        )

        return WorkspaceResponseWatchStarted(watch_started=watch_started)
    elif variant == 10:
        watch_batch_ready = (
            destack._generated.protocol.watch.decode_watch_batch_response(reader)
        )

        return WorkspaceResponseWatchBatchReady(watch_batch_ready=watch_batch_ready)
    elif variant == 11:
        watch_stopped = destack._generated.protocol.watch.decode_watch_stopped_response(
            reader
        )

        return WorkspaceResponseWatchStopped(watch_stopped=watch_stopped)
    elif variant == 12:
        check = (
            destack._generated.protocol.workspace.command.output.decode_check_output(
                reader
            )
        )

        return WorkspaceResponseCheck(check=check)
    elif variant == 13:
        lint = destack._generated.protocol.workspace.command.output.decode_lint_output(
            reader
        )

        return WorkspaceResponseLint(lint=lint)
    elif variant == 14:
        format = (
            destack._generated.protocol.workspace.command.output.decode_format_output(
                reader
            )
        )

        return WorkspaceResponseFormat(format=format)
    elif variant == 15:
        build = (
            destack._generated.protocol.workspace.command.output.decode_build_output(
                reader
            )
        )

        return WorkspaceResponseBuild(build=build)
    elif variant == 16:
        run = destack._generated.protocol.workspace.command.output.decode_run_output(
            reader
        )

        return WorkspaceResponseRun(run=run)
    elif variant == 17:
        test = destack._generated.protocol.workspace.command.output.decode_test_output(
            reader
        )

        return WorkspaceResponseTest(test=test)
    elif variant == 18:
        doc = destack._generated.protocol.workspace.command.output.decode_doc_output(
            reader
        )

        return WorkspaceResponseDoc(doc=doc)
    elif variant == 19:
        bench = (
            destack._generated.protocol.workspace.command.output.decode_bench_output(
                reader
            )
        )

        return WorkspaceResponseBench(bench=bench)
    elif variant == 20:
        info = destack._generated.protocol.workspace.command.output.decode_info_output(
            reader
        )

        return WorkspaceResponseInfo(info=info)
    elif variant == 21:
        targets = (
            destack._generated.protocol.workspace.command.output.decode_targets_output(
                reader
            )
        )

        return WorkspaceResponseTargets(targets=targets)
    elif variant == 22:
        cache = (
            destack._generated.protocol.workspace.command.output.decode_cache_output(
                reader
            )
        )

        return WorkspaceResponseCache(cache=cache)
    elif variant == 23:
        settings = (
            destack._generated.protocol.workspace.command.output.decode_settings_output(
                reader
            )
        )

        return WorkspaceResponseSettings(settings=settings)
    elif variant == 24:
        doctor = (
            destack._generated.protocol.workspace.command.output.decode_doctor_output(
                reader
            )
        )

        return WorkspaceResponseDoctor(doctor=doctor)
    elif variant == 25:
        task = destack._generated.protocol.workspace.command.output.decode_task_output(
            reader
        )

        return WorkspaceResponseTask(task=task)
    elif variant == 26:
        clean = (
            destack._generated.protocol.workspace.command.output.decode_clean_output(
                reader
            )
        )

        return WorkspaceResponseClean(clean=clean)
    elif variant == 27:
        artifact_result = decode_artifact_blob(reader)

        return WorkspaceResponseArtifactResult(artifact_result=artifact_result)
    elif variant == 28:
        store_result = destack._generated.source.file.model.file.decode_content_id(
            reader
        )

        return WorkspaceResponseStoreResult(store_result=store_result)
    elif variant == 29:
        load_result = destack._generated.source.file.model.file.decode_content(reader)

        return WorkspaceResponseLoadResult(load_result=load_result)
    elif variant == 30:
        export_result = (
            destack._generated.protocol.workspace.artifact.export.decode_export_result(
                reader
            )
        )

        return WorkspaceResponseExportResult(export_result=export_result)
    elif variant == 31:
        query_result = (
            destack._generated.protocol.query.decode_workspace_query_response(reader)
        )

        return WorkspaceResponseQueryResult(query_result=query_result)
    elif variant == 32:
        error = destack._generated.protocol.error.decode_protocol_error(reader)

        return WorkspaceResponseError(error=error)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_workspace_response(value: WorkspaceResponse) -> Json:
    """Return one JSON value for one WorkspaceResponse."""
    if value.kind == "handshake":
        return {
            "kind": "handshake",
            "handshake": destack._generated.protocol.handshake.to_json_handshake_response(
                value.handshake
            ),
        }
    elif value.kind == "pong":
        return {
            "kind": "pong",
        }
    elif value.kind == "canceled":
        return {
            "kind": "canceled",
            "id": destack._generated.protocol.envelope.to_json_request_id(value.id),
        }
    elif value.kind == "shutdownAck":
        return {
            "kind": "shutdownAck",
        }
    elif value.kind == "rootOpened":
        return {
            "kind": "rootOpened",
            "root_opened": destack._generated.protocol.root.to_json_root_opened_response(
                value.root_opened
            ),
        }
    elif value.kind == "rootClosed":
        return {
            "kind": "rootClosed",
            "root_closed": destack._generated.protocol.root.to_json_root_closed_response(
                value.root_closed
            ),
        }
    elif value.kind == "rootReloaded":
        return {
            "kind": "rootReloaded",
            "root_reloaded": destack._generated.protocol.root.to_json_root_reload_response(
                value.root_reloaded
            ),
        }
    elif value.kind == "fileOperationApplied":
        return {
            "kind": "fileOperationApplied",
            "file_operation_applied": destack._generated.protocol.root.to_json_file_operation_response(
                value.file_operation_applied
            ),
        }
    elif value.kind == "sourceUpdated":
        return {
            "kind": "sourceUpdated",
            "source_updated": destack._generated.protocol.root.to_json_source_update_response(
                value.source_updated
            ),
        }
    elif value.kind == "watchStarted":
        return {
            "kind": "watchStarted",
            "watch_started": destack._generated.protocol.watch.to_json_watch_started_response(
                value.watch_started
            ),
        }
    elif value.kind == "watchBatchReady":
        return {
            "kind": "watchBatchReady",
            "watch_batch_ready": destack._generated.protocol.watch.to_json_watch_batch_response(
                value.watch_batch_ready
            ),
        }
    elif value.kind == "watchStopped":
        return {
            "kind": "watchStopped",
            "watch_stopped": destack._generated.protocol.watch.to_json_watch_stopped_response(
                value.watch_stopped
            ),
        }
    elif value.kind == "check":
        return {
            "kind": "check",
            "check": destack._generated.protocol.workspace.command.output.to_json_check_output(
                value.check
            ),
        }
    elif value.kind == "lint":
        return {
            "kind": "lint",
            "lint": destack._generated.protocol.workspace.command.output.to_json_lint_output(
                value.lint
            ),
        }
    elif value.kind == "format":
        return {
            "kind": "format",
            "format": destack._generated.protocol.workspace.command.output.to_json_format_output(
                value.format
            ),
        }
    elif value.kind == "build":
        return {
            "kind": "build",
            "build": destack._generated.protocol.workspace.command.output.to_json_build_output(
                value.build
            ),
        }
    elif value.kind == "run":
        return {
            "kind": "run",
            "run": destack._generated.protocol.workspace.command.output.to_json_run_output(
                value.run
            ),
        }
    elif value.kind == "test":
        return {
            "kind": "test",
            "test": destack._generated.protocol.workspace.command.output.to_json_test_output(
                value.test
            ),
        }
    elif value.kind == "doc":
        return {
            "kind": "doc",
            "doc": destack._generated.protocol.workspace.command.output.to_json_doc_output(
                value.doc
            ),
        }
    elif value.kind == "bench":
        return {
            "kind": "bench",
            "bench": destack._generated.protocol.workspace.command.output.to_json_bench_output(
                value.bench
            ),
        }
    elif value.kind == "info":
        return {
            "kind": "info",
            "info": destack._generated.protocol.workspace.command.output.to_json_info_output(
                value.info
            ),
        }
    elif value.kind == "targets":
        return {
            "kind": "targets",
            "targets": destack._generated.protocol.workspace.command.output.to_json_targets_output(
                value.targets
            ),
        }
    elif value.kind == "cache":
        return {
            "kind": "cache",
            "cache": destack._generated.protocol.workspace.command.output.to_json_cache_output(
                value.cache
            ),
        }
    elif value.kind == "settings":
        return {
            "kind": "settings",
            "settings": destack._generated.protocol.workspace.command.output.to_json_settings_output(
                value.settings
            ),
        }
    elif value.kind == "doctor":
        return {
            "kind": "doctor",
            "doctor": destack._generated.protocol.workspace.command.output.to_json_doctor_output(
                value.doctor
            ),
        }
    elif value.kind == "task":
        return {
            "kind": "task",
            "task": destack._generated.protocol.workspace.command.output.to_json_task_output(
                value.task
            ),
        }
    elif value.kind == "clean":
        return {
            "kind": "clean",
            "clean": destack._generated.protocol.workspace.command.output.to_json_clean_output(
                value.clean
            ),
        }
    elif value.kind == "artifactResult":
        return {
            "kind": "artifactResult",
            "artifact_result": to_json_artifact_blob(value.artifact_result),
        }
    elif value.kind == "storeResult":
        return {
            "kind": "storeResult",
            "store_result": destack._generated.source.file.model.file.to_json_content_id(
                value.store_result
            ),
        }
    elif value.kind == "loadResult":
        return {
            "kind": "loadResult",
            "load_result": destack._generated.source.file.model.file.to_json_content(
                value.load_result
            ),
        }
    elif value.kind == "exportResult":
        return {
            "kind": "exportResult",
            "export_result": destack._generated.protocol.workspace.artifact.export.to_json_export_result(
                value.export_result
            ),
        }
    elif value.kind == "queryResult":
        return {
            "kind": "queryResult",
            "query_result": destack._generated.protocol.query.to_json_workspace_query_response(
                value.query_result
            ),
        }
    elif value.kind == "error":
        return {
            "kind": "error",
            "error": destack._generated.protocol.error.to_json_protocol_error(
                value.error
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_workspace_response(value: Json) -> WorkspaceResponse:
    """Return one WorkspaceResponse from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "handshake":
        return WorkspaceResponseHandshake(
            handshake=destack._generated.protocol.handshake.from_json_handshake_response(
                json_field(object_, "handshake")
            )
        )
    elif kind == "pong":
        return WorkspaceResponsePong()
    elif kind == "canceled":
        return WorkspaceResponseCanceled(
            id=destack._generated.protocol.envelope.from_json_request_id(
                json_field(object_, "id")
            ),
        )
    elif kind == "shutdownAck":
        return WorkspaceResponseShutdownAck()
    elif kind == "rootOpened":
        return WorkspaceResponseRootOpened(
            root_opened=destack._generated.protocol.root.from_json_root_opened_response(
                json_field(object_, "root_opened")
            )
        )
    elif kind == "rootClosed":
        return WorkspaceResponseRootClosed(
            root_closed=destack._generated.protocol.root.from_json_root_closed_response(
                json_field(object_, "root_closed")
            )
        )
    elif kind == "rootReloaded":
        return WorkspaceResponseRootReloaded(
            root_reloaded=destack._generated.protocol.root.from_json_root_reload_response(
                json_field(object_, "root_reloaded")
            )
        )
    elif kind == "fileOperationApplied":
        return WorkspaceResponseFileOperationApplied(
            file_operation_applied=destack._generated.protocol.root.from_json_file_operation_response(
                json_field(object_, "file_operation_applied")
            )
        )
    elif kind == "sourceUpdated":
        return WorkspaceResponseSourceUpdated(
            source_updated=destack._generated.protocol.root.from_json_source_update_response(
                json_field(object_, "source_updated")
            )
        )
    elif kind == "watchStarted":
        return WorkspaceResponseWatchStarted(
            watch_started=destack._generated.protocol.watch.from_json_watch_started_response(
                json_field(object_, "watch_started")
            )
        )
    elif kind == "watchBatchReady":
        return WorkspaceResponseWatchBatchReady(
            watch_batch_ready=destack._generated.protocol.watch.from_json_watch_batch_response(
                json_field(object_, "watch_batch_ready")
            )
        )
    elif kind == "watchStopped":
        return WorkspaceResponseWatchStopped(
            watch_stopped=destack._generated.protocol.watch.from_json_watch_stopped_response(
                json_field(object_, "watch_stopped")
            )
        )
    elif kind == "check":
        return WorkspaceResponseCheck(
            check=destack._generated.protocol.workspace.command.output.from_json_check_output(
                json_field(object_, "check")
            )
        )
    elif kind == "lint":
        return WorkspaceResponseLint(
            lint=destack._generated.protocol.workspace.command.output.from_json_lint_output(
                json_field(object_, "lint")
            )
        )
    elif kind == "format":
        return WorkspaceResponseFormat(
            format=destack._generated.protocol.workspace.command.output.from_json_format_output(
                json_field(object_, "format")
            )
        )
    elif kind == "build":
        return WorkspaceResponseBuild(
            build=destack._generated.protocol.workspace.command.output.from_json_build_output(
                json_field(object_, "build")
            )
        )
    elif kind == "run":
        return WorkspaceResponseRun(
            run=destack._generated.protocol.workspace.command.output.from_json_run_output(
                json_field(object_, "run")
            )
        )
    elif kind == "test":
        return WorkspaceResponseTest(
            test=destack._generated.protocol.workspace.command.output.from_json_test_output(
                json_field(object_, "test")
            )
        )
    elif kind == "doc":
        return WorkspaceResponseDoc(
            doc=destack._generated.protocol.workspace.command.output.from_json_doc_output(
                json_field(object_, "doc")
            )
        )
    elif kind == "bench":
        return WorkspaceResponseBench(
            bench=destack._generated.protocol.workspace.command.output.from_json_bench_output(
                json_field(object_, "bench")
            )
        )
    elif kind == "info":
        return WorkspaceResponseInfo(
            info=destack._generated.protocol.workspace.command.output.from_json_info_output(
                json_field(object_, "info")
            )
        )
    elif kind == "targets":
        return WorkspaceResponseTargets(
            targets=destack._generated.protocol.workspace.command.output.from_json_targets_output(
                json_field(object_, "targets")
            )
        )
    elif kind == "cache":
        return WorkspaceResponseCache(
            cache=destack._generated.protocol.workspace.command.output.from_json_cache_output(
                json_field(object_, "cache")
            )
        )
    elif kind == "settings":
        return WorkspaceResponseSettings(
            settings=destack._generated.protocol.workspace.command.output.from_json_settings_output(
                json_field(object_, "settings")
            )
        )
    elif kind == "doctor":
        return WorkspaceResponseDoctor(
            doctor=destack._generated.protocol.workspace.command.output.from_json_doctor_output(
                json_field(object_, "doctor")
            )
        )
    elif kind == "task":
        return WorkspaceResponseTask(
            task=destack._generated.protocol.workspace.command.output.from_json_task_output(
                json_field(object_, "task")
            )
        )
    elif kind == "clean":
        return WorkspaceResponseClean(
            clean=destack._generated.protocol.workspace.command.output.from_json_clean_output(
                json_field(object_, "clean")
            )
        )
    elif kind == "artifactResult":
        return WorkspaceResponseArtifactResult(
            artifact_result=from_json_artifact_blob(
                json_field(object_, "artifact_result")
            )
        )
    elif kind == "storeResult":
        return WorkspaceResponseStoreResult(
            store_result=destack._generated.source.file.model.file.from_json_content_id(
                json_field(object_, "store_result")
            )
        )
    elif kind == "loadResult":
        return WorkspaceResponseLoadResult(
            load_result=destack._generated.source.file.model.file.from_json_content(
                json_field(object_, "load_result")
            )
        )
    elif kind == "exportResult":
        return WorkspaceResponseExportResult(
            export_result=destack._generated.protocol.workspace.artifact.export.from_json_export_result(
                json_field(object_, "export_result")
            )
        )
    elif kind == "queryResult":
        return WorkspaceResponseQueryResult(
            query_result=destack._generated.protocol.query.from_json_workspace_query_response(
                json_field(object_, "query_result")
            )
        )
    elif kind == "error":
        return WorkspaceResponseError(
            error=destack._generated.protocol.error.from_json_protocol_error(
                json_field(object_, "error")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class ArtifactBlob:
    """Serialized artifact payload returned by the workspace protocol."""

    # exact artifact version
    version: destack._generated.artifact.core.version.ArtifactVersion
    # serialized artifact payload bytes
    bytes: builtins.bytes | bytearray | Sequence[int]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_blob(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ArtifactBlob:
        """Decode one ArtifactBlob."""
        return decode_artifact_blob(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_blob(self)

    @classmethod
    def from_json(cls, value: Json) -> ArtifactBlob:
        """Return one ArtifactBlob from one JSON value."""
        return from_json_artifact_blob(value)


def encode_artifact_blob(writer: BinaryWriter, value: ArtifactBlob) -> None:
    """Encode one ArtifactBlob."""
    destack._generated.artifact.core.version.encode_artifact_version(
        writer, value.version
    )
    writer.write_byte_slice(value.bytes)


def decode_artifact_blob(reader: BinaryReader) -> ArtifactBlob:
    """Decode one ArtifactBlob."""
    version = destack._generated.artifact.core.version.decode_artifact_version(reader)
    bytes = reader.read_byte_slice()

    return ArtifactBlob(
        version=version,
        bytes=bytes,
    )


def to_json_artifact_blob(value: ArtifactBlob) -> Json:
    """Return one JSON value for one ArtifactBlob."""
    return {
        "version": destack._generated.artifact.core.version.to_json_artifact_version(
            value.version
        ),
        "bytes": bytes_to_json(value.bytes),
    }


def from_json_artifact_blob(value: Json) -> ArtifactBlob:
    """Return one ArtifactBlob from one JSON value."""
    object_ = json_object(value)

    return ArtifactBlob(
        version=destack._generated.artifact.core.version.from_json_artifact_version(
            json_field(object_, "version")
        ),
        bytes=bytes_from_json(json_field(object_, "bytes")),
    )


__all__ = [
    "WorkspaceResponse",
    "encode_workspace_response",
    "decode_workspace_response",
    "to_json_workspace_response",
    "from_json_workspace_response",
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
    "to_json_artifact_blob",
    "from_json_artifact_blob",
]
