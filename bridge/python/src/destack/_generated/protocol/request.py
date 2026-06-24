# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_field,
    json_object,
    json_string,
)

import destack._generated.artifact.reference
import destack._generated.protocol.envelope
import destack._generated.protocol.handshake
import destack._generated.protocol.query
import destack._generated.protocol.root
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
import destack._generated.source.file.model.file


@dataclass(frozen=True, slots=True)
class WorkspaceRequestHandshake:
    """Negotiate protocol version and capabilities."""

    handshake: destack._generated.protocol.handshake.HandshakeRequest
    kind: typing.Literal["handshake"] = "handshake"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_request(self)


@dataclass(frozen=True, slots=True)
class WorkspaceRequestPing:
    """Check server liveness."""

    kind: typing.Literal["ping"] = "ping"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_request(self)


@dataclass(frozen=True, slots=True)
class WorkspaceRequestCancel:
    """Cancel an in flight request."""

    id: destack._generated.protocol.envelope.RequestId
    kind: typing.Literal["cancel"] = "cancel"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_request(self)


@dataclass(frozen=True, slots=True)
class WorkspaceRequestShutdown:
    """Request server shutdown."""

    kind: typing.Literal["shutdown"] = "shutdown"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_request(self)


@dataclass(frozen=True, slots=True)
class WorkspaceRequestOpenRoot:
    """Open or register a root."""

    open_root: destack._generated.protocol.root.OpenRootRequest
    kind: typing.Literal["openRoot"] = "openRoot"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_request(self)


@dataclass(frozen=True, slots=True)
class WorkspaceRequestCloseRoot:
    """Close a root handle."""

    close_root: destack._generated.protocol.root.CloseRootRequest
    kind: typing.Literal["closeRoot"] = "closeRoot"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_request(self)


@dataclass(frozen=True, slots=True)
class WorkspaceRequestReloadRoot:
    """Reload a root."""

    reload_root: destack._generated.protocol.root.ReloadRootRequest
    kind: typing.Literal["reloadRoot"] = "reloadRoot"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_request(self)


@dataclass(frozen=True, slots=True)
class WorkspaceRequestApplyFileOperation:
    """Apply a file operation to a root."""

    apply_file_operation: destack._generated.protocol.root.FileOperationRequest
    kind: typing.Literal["applyFileOperation"] = "applyFileOperation"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_request(self)


@dataclass(frozen=True, slots=True)
class WorkspaceRequestApplySourceUpdate:
    """Apply a source update to a root."""

    apply_source_update: destack._generated.protocol.root.SourceUpdateRequest
    kind: typing.Literal["applySourceUpdate"] = "applySourceUpdate"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_request(self)


@dataclass(frozen=True, slots=True)
class WorkspaceRequestStartWatch:
    """Start watching a root."""

    start_watch: destack._generated.protocol.watch.WatchStartRequest
    kind: typing.Literal["startWatch"] = "startWatch"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_request(self)


@dataclass(frozen=True, slots=True)
class WorkspaceRequestNextWatchBatch:
    """Receive and apply the next watch batch."""

    next_watch_batch: destack._generated.protocol.watch.WatchNextRequest
    kind: typing.Literal["nextWatchBatch"] = "nextWatchBatch"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_request(self)


@dataclass(frozen=True, slots=True)
class WorkspaceRequestStopWatch:
    """Stop watching a root."""

    stop_watch: destack._generated.protocol.watch.WatchStopRequest
    kind: typing.Literal["stopWatch"] = "stopWatch"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_request(self)


@dataclass(frozen=True, slots=True)
class WorkspaceRequestCheck:
    """Check source state."""

    # root handle
    handle: destack._generated.protocol.root.RootId
    # check input
    input: destack._generated.protocol.workspace.command.check.CheckInput
    kind: typing.Literal["check"] = "check"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_request(self)


@dataclass(frozen=True, slots=True)
class WorkspaceRequestLint:
    """Lint source state."""

    # root handle
    handle: destack._generated.protocol.root.RootId
    # lint input
    input: destack._generated.protocol.workspace.command.check.LintInput
    kind: typing.Literal["lint"] = "lint"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_request(self)


@dataclass(frozen=True, slots=True)
class WorkspaceRequestFormat:
    """Format source files or content."""

    # root handle
    handle: destack._generated.protocol.root.RootId
    # format input
    input: destack._generated.protocol.workspace.command.format.FormatInput
    kind: typing.Literal["format"] = "format"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_request(self)


@dataclass(frozen=True, slots=True)
class WorkspaceRequestBuild:
    """Build target artifacts."""

    # root handle
    handle: destack._generated.protocol.root.RootId
    # build input
    input: destack._generated.protocol.workspace.command.build.BuildInput
    kind: typing.Literal["build"] = "build"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_request(self)


@dataclass(frozen=True, slots=True)
class WorkspaceRequestRun:
    """Run a workspace target."""

    # root handle
    handle: destack._generated.protocol.root.RootId
    # run input
    input: destack._generated.protocol.workspace.command.run.RunInput
    kind: typing.Literal["run"] = "run"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_request(self)


@dataclass(frozen=True, slots=True)
class WorkspaceRequestTest:
    """Run workspace tests."""

    # root handle
    handle: destack._generated.protocol.root.RootId
    # test input
    input: destack._generated.protocol.workspace.command.test.TestInput
    kind: typing.Literal["test"] = "test"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_request(self)


@dataclass(frozen=True, slots=True)
class WorkspaceRequestDoc:
    """Generate documentation."""

    # root handle
    handle: destack._generated.protocol.root.RootId
    # documentation input
    input: destack._generated.protocol.workspace.command.doc.DocInput
    kind: typing.Literal["doc"] = "doc"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_request(self)


@dataclass(frozen=True, slots=True)
class WorkspaceRequestBench:
    """Run benchmarks."""

    # root handle
    handle: destack._generated.protocol.root.RootId
    # benchmark input
    input: destack._generated.protocol.workspace.command.bench.BenchInput
    kind: typing.Literal["bench"] = "bench"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_request(self)


@dataclass(frozen=True, slots=True)
class WorkspaceRequestInfo:
    """Return workspace information."""

    # root handle
    handle: destack._generated.protocol.root.RootId
    # information input
    input: destack._generated.protocol.workspace.command.info.InfoInput
    kind: typing.Literal["info"] = "info"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_request(self)


@dataclass(frozen=True, slots=True)
class WorkspaceRequestTargets:
    """Return configured targets."""

    # root handle
    handle: destack._generated.protocol.root.RootId
    # targets input
    input: destack._generated.protocol.workspace.command.targets.TargetsInput
    kind: typing.Literal["targets"] = "targets"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_request(self)


@dataclass(frozen=True, slots=True)
class WorkspaceRequestCache:
    """Return cache locations."""

    # root handle
    handle: destack._generated.protocol.root.RootId
    # cache input
    input: destack._generated.protocol.workspace.command.cache.CacheInput
    kind: typing.Literal["cache"] = "cache"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_request(self)


@dataclass(frozen=True, slots=True)
class WorkspaceRequestSettings:
    """Return resolved settings."""

    # root handle
    handle: destack._generated.protocol.root.RootId
    # settings input
    input: destack._generated.protocol.workspace.command.settings.SettingsInput
    kind: typing.Literal["settings"] = "settings"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_request(self)


@dataclass(frozen=True, slots=True)
class WorkspaceRequestDoctor:
    """Return workspace health information."""

    # root handle
    handle: destack._generated.protocol.root.RootId
    # doctor input
    input: destack._generated.protocol.workspace.command.doctor.DoctorInput
    kind: typing.Literal["doctor"] = "doctor"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_request(self)


@dataclass(frozen=True, slots=True)
class WorkspaceRequestTask:
    """Run workspace tasks."""

    # root handle
    handle: destack._generated.protocol.root.RootId
    # task input
    input: destack._generated.protocol.workspace.command.task.TaskInput
    kind: typing.Literal["task"] = "task"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_request(self)


@dataclass(frozen=True, slots=True)
class WorkspaceRequestClean:
    """Clean generated state."""

    # root handle
    handle: destack._generated.protocol.root.RootId
    # clean input
    input: destack._generated.protocol.workspace.command.clean.CleanInput
    kind: typing.Literal["clean"] = "clean"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_request(self)


@dataclass(frozen=True, slots=True)
class WorkspaceRequestArtifact:
    """Return one artifact payload."""

    # root handle
    handle: destack._generated.protocol.root.RootId
    # artifact reference
    artifact: destack._generated.artifact.reference.ArtifactReference
    kind: typing.Literal["artifact"] = "artifact"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_request(self)


@dataclass(frozen=True, slots=True)
class WorkspaceRequestStore:
    """Store one content payload."""

    # root handle
    handle: destack._generated.protocol.root.RootId
    # content payload
    content: destack._generated.source.file.model.file.Content
    kind: typing.Literal["store"] = "store"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_request(self)


@dataclass(frozen=True, slots=True)
class WorkspaceRequestLoad:
    """Load one content payload."""

    # root handle
    handle: destack._generated.protocol.root.RootId
    # content id
    content: destack._generated.source.file.model.file.ContentId
    kind: typing.Literal["load"] = "load"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_request(self)


@dataclass(frozen=True, slots=True)
class WorkspaceRequestExport:
    """Materialize derived outputs."""

    # root handle
    handle: destack._generated.protocol.root.RootId
    # export request
    request: destack._generated.protocol.workspace.artifact.export.ExportRequest
    kind: typing.Literal["export"] = "export"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_request(self)


@dataclass(frozen=True, slots=True)
class WorkspaceRequestQuery:
    """Execute a query."""

    query: destack._generated.protocol.query.WorkspaceQuery
    kind: typing.Literal["query"] = "query"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_request(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_request(self)


"""Requests accepted by the workspace protocol."""
WorkspaceRequest: typing.TypeAlias = (
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


def encode_workspace_request(writer: BinaryWriter, value: WorkspaceRequest) -> None:
    """Encode one WorkspaceRequest."""
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
        destack._generated.artifact.reference.encode_artifact_reference(
            writer, value.artifact
        )
    elif value.kind == "store":
        writer.write_unsigned(28)
        destack._generated.protocol.root.encode_root_id(writer, value.handle)
        destack._generated.source.file.model.file.encode_content(writer, value.content)
    elif value.kind == "load":
        writer.write_unsigned(29)
        destack._generated.protocol.root.encode_root_id(writer, value.handle)
        destack._generated.source.file.model.file.encode_content_id(
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
        destack._generated.protocol.query.encode_workspace_query(writer, value.query)
    else:
        raise SerdeError("unknown enum variant")


def decode_workspace_request(reader: BinaryReader) -> WorkspaceRequest:
    """Decode one WorkspaceRequest."""
    variant = reader.read_number()

    if variant == 0:
        handshake = destack._generated.protocol.handshake.decode_handshake_request(
            reader
        )

        return WorkspaceRequestHandshake(handshake=handshake)
    elif variant == 1:
        return WorkspaceRequestPing()
    elif variant == 2:
        id = destack._generated.protocol.envelope.decode_request_id(reader)

        return WorkspaceRequestCancel(
            id=id,
        )
    elif variant == 3:
        return WorkspaceRequestShutdown()
    elif variant == 4:
        open_root = destack._generated.protocol.root.decode_open_root_request(reader)

        return WorkspaceRequestOpenRoot(open_root=open_root)
    elif variant == 5:
        close_root = destack._generated.protocol.root.decode_close_root_request(reader)

        return WorkspaceRequestCloseRoot(close_root=close_root)
    elif variant == 6:
        reload_root = destack._generated.protocol.root.decode_reload_root_request(
            reader
        )

        return WorkspaceRequestReloadRoot(reload_root=reload_root)
    elif variant == 7:
        apply_file_operation = (
            destack._generated.protocol.root.decode_file_operation_request(reader)
        )

        return WorkspaceRequestApplyFileOperation(
            apply_file_operation=apply_file_operation
        )
    elif variant == 8:
        apply_source_update = (
            destack._generated.protocol.root.decode_source_update_request(reader)
        )

        return WorkspaceRequestApplySourceUpdate(
            apply_source_update=apply_source_update
        )
    elif variant == 9:
        start_watch = destack._generated.protocol.watch.decode_watch_start_request(
            reader
        )

        return WorkspaceRequestStartWatch(start_watch=start_watch)
    elif variant == 10:
        next_watch_batch = destack._generated.protocol.watch.decode_watch_next_request(
            reader
        )

        return WorkspaceRequestNextWatchBatch(next_watch_batch=next_watch_batch)
    elif variant == 11:
        stop_watch = destack._generated.protocol.watch.decode_watch_stop_request(reader)

        return WorkspaceRequestStopWatch(stop_watch=stop_watch)
    elif variant == 12:
        handle = destack._generated.protocol.root.decode_root_id(reader)
        input = destack._generated.protocol.workspace.command.check.decode_check_input(
            reader
        )

        return WorkspaceRequestCheck(
            handle=handle,
            input=input,
        )
    elif variant == 13:
        handle = destack._generated.protocol.root.decode_root_id(reader)
        input = destack._generated.protocol.workspace.command.check.decode_lint_input(
            reader
        )

        return WorkspaceRequestLint(
            handle=handle,
            input=input,
        )
    elif variant == 14:
        handle = destack._generated.protocol.root.decode_root_id(reader)
        input = (
            destack._generated.protocol.workspace.command.format.decode_format_input(
                reader
            )
        )

        return WorkspaceRequestFormat(
            handle=handle,
            input=input,
        )
    elif variant == 15:
        handle = destack._generated.protocol.root.decode_root_id(reader)
        input = destack._generated.protocol.workspace.command.build.decode_build_input(
            reader
        )

        return WorkspaceRequestBuild(
            handle=handle,
            input=input,
        )
    elif variant == 16:
        handle = destack._generated.protocol.root.decode_root_id(reader)
        input = destack._generated.protocol.workspace.command.run.decode_run_input(
            reader
        )

        return WorkspaceRequestRun(
            handle=handle,
            input=input,
        )
    elif variant == 17:
        handle = destack._generated.protocol.root.decode_root_id(reader)
        input = destack._generated.protocol.workspace.command.test.decode_test_input(
            reader
        )

        return WorkspaceRequestTest(
            handle=handle,
            input=input,
        )
    elif variant == 18:
        handle = destack._generated.protocol.root.decode_root_id(reader)
        input = destack._generated.protocol.workspace.command.doc.decode_doc_input(
            reader
        )

        return WorkspaceRequestDoc(
            handle=handle,
            input=input,
        )
    elif variant == 19:
        handle = destack._generated.protocol.root.decode_root_id(reader)
        input = destack._generated.protocol.workspace.command.bench.decode_bench_input(
            reader
        )

        return WorkspaceRequestBench(
            handle=handle,
            input=input,
        )
    elif variant == 20:
        handle = destack._generated.protocol.root.decode_root_id(reader)
        input = destack._generated.protocol.workspace.command.info.decode_info_input(
            reader
        )

        return WorkspaceRequestInfo(
            handle=handle,
            input=input,
        )
    elif variant == 21:
        handle = destack._generated.protocol.root.decode_root_id(reader)
        input = (
            destack._generated.protocol.workspace.command.targets.decode_targets_input(
                reader
            )
        )

        return WorkspaceRequestTargets(
            handle=handle,
            input=input,
        )
    elif variant == 22:
        handle = destack._generated.protocol.root.decode_root_id(reader)
        input = destack._generated.protocol.workspace.command.cache.decode_cache_input(
            reader
        )

        return WorkspaceRequestCache(
            handle=handle,
            input=input,
        )
    elif variant == 23:
        handle = destack._generated.protocol.root.decode_root_id(reader)
        input = destack._generated.protocol.workspace.command.settings.decode_settings_input(
            reader
        )

        return WorkspaceRequestSettings(
            handle=handle,
            input=input,
        )
    elif variant == 24:
        handle = destack._generated.protocol.root.decode_root_id(reader)
        input = (
            destack._generated.protocol.workspace.command.doctor.decode_doctor_input(
                reader
            )
        )

        return WorkspaceRequestDoctor(
            handle=handle,
            input=input,
        )
    elif variant == 25:
        handle = destack._generated.protocol.root.decode_root_id(reader)
        input = destack._generated.protocol.workspace.command.task.decode_task_input(
            reader
        )

        return WorkspaceRequestTask(
            handle=handle,
            input=input,
        )
    elif variant == 26:
        handle = destack._generated.protocol.root.decode_root_id(reader)
        input = destack._generated.protocol.workspace.command.clean.decode_clean_input(
            reader
        )

        return WorkspaceRequestClean(
            handle=handle,
            input=input,
        )
    elif variant == 27:
        handle = destack._generated.protocol.root.decode_root_id(reader)
        artifact = destack._generated.artifact.reference.decode_artifact_reference(
            reader
        )

        return WorkspaceRequestArtifact(
            handle=handle,
            artifact=artifact,
        )
    elif variant == 28:
        handle = destack._generated.protocol.root.decode_root_id(reader)
        content = destack._generated.source.file.model.file.decode_content(reader)

        return WorkspaceRequestStore(
            handle=handle,
            content=content,
        )
    elif variant == 29:
        handle = destack._generated.protocol.root.decode_root_id(reader)
        content = destack._generated.source.file.model.file.decode_content_id(reader)

        return WorkspaceRequestLoad(
            handle=handle,
            content=content,
        )
    elif variant == 30:
        handle = destack._generated.protocol.root.decode_root_id(reader)
        request = (
            destack._generated.protocol.workspace.artifact.export.decode_export_request(
                reader
            )
        )

        return WorkspaceRequestExport(
            handle=handle,
            request=request,
        )
    elif variant == 31:
        query = destack._generated.protocol.query.decode_workspace_query(reader)

        return WorkspaceRequestQuery(query=query)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_workspace_request(value: WorkspaceRequest) -> Json:
    """Return one JSON value for one WorkspaceRequest."""
    if value.kind == "handshake":
        return {
            "kind": "handshake",
            "handshake": destack._generated.protocol.handshake.to_json_handshake_request(
                value.handshake
            ),
        }
    elif value.kind == "ping":
        return {
            "kind": "ping",
        }
    elif value.kind == "cancel":
        return {
            "kind": "cancel",
            "id": destack._generated.protocol.envelope.to_json_request_id(value.id),
        }
    elif value.kind == "shutdown":
        return {
            "kind": "shutdown",
        }
    elif value.kind == "openRoot":
        return {
            "kind": "openRoot",
            "open_root": destack._generated.protocol.root.to_json_open_root_request(
                value.open_root
            ),
        }
    elif value.kind == "closeRoot":
        return {
            "kind": "closeRoot",
            "close_root": destack._generated.protocol.root.to_json_close_root_request(
                value.close_root
            ),
        }
    elif value.kind == "reloadRoot":
        return {
            "kind": "reloadRoot",
            "reload_root": destack._generated.protocol.root.to_json_reload_root_request(
                value.reload_root
            ),
        }
    elif value.kind == "applyFileOperation":
        return {
            "kind": "applyFileOperation",
            "apply_file_operation": destack._generated.protocol.root.to_json_file_operation_request(
                value.apply_file_operation
            ),
        }
    elif value.kind == "applySourceUpdate":
        return {
            "kind": "applySourceUpdate",
            "apply_source_update": destack._generated.protocol.root.to_json_source_update_request(
                value.apply_source_update
            ),
        }
    elif value.kind == "startWatch":
        return {
            "kind": "startWatch",
            "start_watch": destack._generated.protocol.watch.to_json_watch_start_request(
                value.start_watch
            ),
        }
    elif value.kind == "nextWatchBatch":
        return {
            "kind": "nextWatchBatch",
            "next_watch_batch": destack._generated.protocol.watch.to_json_watch_next_request(
                value.next_watch_batch
            ),
        }
    elif value.kind == "stopWatch":
        return {
            "kind": "stopWatch",
            "stop_watch": destack._generated.protocol.watch.to_json_watch_stop_request(
                value.stop_watch
            ),
        }
    elif value.kind == "check":
        return {
            "kind": "check",
            "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
            "input": destack._generated.protocol.workspace.command.check.to_json_check_input(
                value.input
            ),
        }
    elif value.kind == "lint":
        return {
            "kind": "lint",
            "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
            "input": destack._generated.protocol.workspace.command.check.to_json_lint_input(
                value.input
            ),
        }
    elif value.kind == "format":
        return {
            "kind": "format",
            "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
            "input": destack._generated.protocol.workspace.command.format.to_json_format_input(
                value.input
            ),
        }
    elif value.kind == "build":
        return {
            "kind": "build",
            "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
            "input": destack._generated.protocol.workspace.command.build.to_json_build_input(
                value.input
            ),
        }
    elif value.kind == "run":
        return {
            "kind": "run",
            "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
            "input": destack._generated.protocol.workspace.command.run.to_json_run_input(
                value.input
            ),
        }
    elif value.kind == "test":
        return {
            "kind": "test",
            "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
            "input": destack._generated.protocol.workspace.command.test.to_json_test_input(
                value.input
            ),
        }
    elif value.kind == "doc":
        return {
            "kind": "doc",
            "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
            "input": destack._generated.protocol.workspace.command.doc.to_json_doc_input(
                value.input
            ),
        }
    elif value.kind == "bench":
        return {
            "kind": "bench",
            "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
            "input": destack._generated.protocol.workspace.command.bench.to_json_bench_input(
                value.input
            ),
        }
    elif value.kind == "info":
        return {
            "kind": "info",
            "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
            "input": destack._generated.protocol.workspace.command.info.to_json_info_input(
                value.input
            ),
        }
    elif value.kind == "targets":
        return {
            "kind": "targets",
            "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
            "input": destack._generated.protocol.workspace.command.targets.to_json_targets_input(
                value.input
            ),
        }
    elif value.kind == "cache":
        return {
            "kind": "cache",
            "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
            "input": destack._generated.protocol.workspace.command.cache.to_json_cache_input(
                value.input
            ),
        }
    elif value.kind == "settings":
        return {
            "kind": "settings",
            "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
            "input": destack._generated.protocol.workspace.command.settings.to_json_settings_input(
                value.input
            ),
        }
    elif value.kind == "doctor":
        return {
            "kind": "doctor",
            "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
            "input": destack._generated.protocol.workspace.command.doctor.to_json_doctor_input(
                value.input
            ),
        }
    elif value.kind == "task":
        return {
            "kind": "task",
            "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
            "input": destack._generated.protocol.workspace.command.task.to_json_task_input(
                value.input
            ),
        }
    elif value.kind == "clean":
        return {
            "kind": "clean",
            "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
            "input": destack._generated.protocol.workspace.command.clean.to_json_clean_input(
                value.input
            ),
        }
    elif value.kind == "artifact":
        return {
            "kind": "artifact",
            "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
            "artifact": destack._generated.artifact.reference.to_json_artifact_reference(
                value.artifact
            ),
        }
    elif value.kind == "store":
        return {
            "kind": "store",
            "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
            "content": destack._generated.source.file.model.file.to_json_content(
                value.content
            ),
        }
    elif value.kind == "load":
        return {
            "kind": "load",
            "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
            "content": destack._generated.source.file.model.file.to_json_content_id(
                value.content
            ),
        }
    elif value.kind == "export":
        return {
            "kind": "export",
            "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
            "request": destack._generated.protocol.workspace.artifact.export.to_json_export_request(
                value.request
            ),
        }
    elif value.kind == "query":
        return {
            "kind": "query",
            "query": destack._generated.protocol.query.to_json_workspace_query(
                value.query
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_workspace_request(value: Json) -> WorkspaceRequest:
    """Return one WorkspaceRequest from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "handshake":
        return WorkspaceRequestHandshake(
            handshake=destack._generated.protocol.handshake.from_json_handshake_request(
                json_field(object_, "handshake")
            )
        )
    elif kind == "ping":
        return WorkspaceRequestPing()
    elif kind == "cancel":
        return WorkspaceRequestCancel(
            id=destack._generated.protocol.envelope.from_json_request_id(
                json_field(object_, "id")
            ),
        )
    elif kind == "shutdown":
        return WorkspaceRequestShutdown()
    elif kind == "openRoot":
        return WorkspaceRequestOpenRoot(
            open_root=destack._generated.protocol.root.from_json_open_root_request(
                json_field(object_, "open_root")
            )
        )
    elif kind == "closeRoot":
        return WorkspaceRequestCloseRoot(
            close_root=destack._generated.protocol.root.from_json_close_root_request(
                json_field(object_, "close_root")
            )
        )
    elif kind == "reloadRoot":
        return WorkspaceRequestReloadRoot(
            reload_root=destack._generated.protocol.root.from_json_reload_root_request(
                json_field(object_, "reload_root")
            )
        )
    elif kind == "applyFileOperation":
        return WorkspaceRequestApplyFileOperation(
            apply_file_operation=destack._generated.protocol.root.from_json_file_operation_request(
                json_field(object_, "apply_file_operation")
            )
        )
    elif kind == "applySourceUpdate":
        return WorkspaceRequestApplySourceUpdate(
            apply_source_update=destack._generated.protocol.root.from_json_source_update_request(
                json_field(object_, "apply_source_update")
            )
        )
    elif kind == "startWatch":
        return WorkspaceRequestStartWatch(
            start_watch=destack._generated.protocol.watch.from_json_watch_start_request(
                json_field(object_, "start_watch")
            )
        )
    elif kind == "nextWatchBatch":
        return WorkspaceRequestNextWatchBatch(
            next_watch_batch=destack._generated.protocol.watch.from_json_watch_next_request(
                json_field(object_, "next_watch_batch")
            )
        )
    elif kind == "stopWatch":
        return WorkspaceRequestStopWatch(
            stop_watch=destack._generated.protocol.watch.from_json_watch_stop_request(
                json_field(object_, "stop_watch")
            )
        )
    elif kind == "check":
        return WorkspaceRequestCheck(
            handle=destack._generated.protocol.root.from_json_root_id(
                json_field(object_, "handle")
            ),
            input=destack._generated.protocol.workspace.command.check.from_json_check_input(
                json_field(object_, "input")
            ),
        )
    elif kind == "lint":
        return WorkspaceRequestLint(
            handle=destack._generated.protocol.root.from_json_root_id(
                json_field(object_, "handle")
            ),
            input=destack._generated.protocol.workspace.command.check.from_json_lint_input(
                json_field(object_, "input")
            ),
        )
    elif kind == "format":
        return WorkspaceRequestFormat(
            handle=destack._generated.protocol.root.from_json_root_id(
                json_field(object_, "handle")
            ),
            input=destack._generated.protocol.workspace.command.format.from_json_format_input(
                json_field(object_, "input")
            ),
        )
    elif kind == "build":
        return WorkspaceRequestBuild(
            handle=destack._generated.protocol.root.from_json_root_id(
                json_field(object_, "handle")
            ),
            input=destack._generated.protocol.workspace.command.build.from_json_build_input(
                json_field(object_, "input")
            ),
        )
    elif kind == "run":
        return WorkspaceRequestRun(
            handle=destack._generated.protocol.root.from_json_root_id(
                json_field(object_, "handle")
            ),
            input=destack._generated.protocol.workspace.command.run.from_json_run_input(
                json_field(object_, "input")
            ),
        )
    elif kind == "test":
        return WorkspaceRequestTest(
            handle=destack._generated.protocol.root.from_json_root_id(
                json_field(object_, "handle")
            ),
            input=destack._generated.protocol.workspace.command.test.from_json_test_input(
                json_field(object_, "input")
            ),
        )
    elif kind == "doc":
        return WorkspaceRequestDoc(
            handle=destack._generated.protocol.root.from_json_root_id(
                json_field(object_, "handle")
            ),
            input=destack._generated.protocol.workspace.command.doc.from_json_doc_input(
                json_field(object_, "input")
            ),
        )
    elif kind == "bench":
        return WorkspaceRequestBench(
            handle=destack._generated.protocol.root.from_json_root_id(
                json_field(object_, "handle")
            ),
            input=destack._generated.protocol.workspace.command.bench.from_json_bench_input(
                json_field(object_, "input")
            ),
        )
    elif kind == "info":
        return WorkspaceRequestInfo(
            handle=destack._generated.protocol.root.from_json_root_id(
                json_field(object_, "handle")
            ),
            input=destack._generated.protocol.workspace.command.info.from_json_info_input(
                json_field(object_, "input")
            ),
        )
    elif kind == "targets":
        return WorkspaceRequestTargets(
            handle=destack._generated.protocol.root.from_json_root_id(
                json_field(object_, "handle")
            ),
            input=destack._generated.protocol.workspace.command.targets.from_json_targets_input(
                json_field(object_, "input")
            ),
        )
    elif kind == "cache":
        return WorkspaceRequestCache(
            handle=destack._generated.protocol.root.from_json_root_id(
                json_field(object_, "handle")
            ),
            input=destack._generated.protocol.workspace.command.cache.from_json_cache_input(
                json_field(object_, "input")
            ),
        )
    elif kind == "settings":
        return WorkspaceRequestSettings(
            handle=destack._generated.protocol.root.from_json_root_id(
                json_field(object_, "handle")
            ),
            input=destack._generated.protocol.workspace.command.settings.from_json_settings_input(
                json_field(object_, "input")
            ),
        )
    elif kind == "doctor":
        return WorkspaceRequestDoctor(
            handle=destack._generated.protocol.root.from_json_root_id(
                json_field(object_, "handle")
            ),
            input=destack._generated.protocol.workspace.command.doctor.from_json_doctor_input(
                json_field(object_, "input")
            ),
        )
    elif kind == "task":
        return WorkspaceRequestTask(
            handle=destack._generated.protocol.root.from_json_root_id(
                json_field(object_, "handle")
            ),
            input=destack._generated.protocol.workspace.command.task.from_json_task_input(
                json_field(object_, "input")
            ),
        )
    elif kind == "clean":
        return WorkspaceRequestClean(
            handle=destack._generated.protocol.root.from_json_root_id(
                json_field(object_, "handle")
            ),
            input=destack._generated.protocol.workspace.command.clean.from_json_clean_input(
                json_field(object_, "input")
            ),
        )
    elif kind == "artifact":
        return WorkspaceRequestArtifact(
            handle=destack._generated.protocol.root.from_json_root_id(
                json_field(object_, "handle")
            ),
            artifact=destack._generated.artifact.reference.from_json_artifact_reference(
                json_field(object_, "artifact")
            ),
        )
    elif kind == "store":
        return WorkspaceRequestStore(
            handle=destack._generated.protocol.root.from_json_root_id(
                json_field(object_, "handle")
            ),
            content=destack._generated.source.file.model.file.from_json_content(
                json_field(object_, "content")
            ),
        )
    elif kind == "load":
        return WorkspaceRequestLoad(
            handle=destack._generated.protocol.root.from_json_root_id(
                json_field(object_, "handle")
            ),
            content=destack._generated.source.file.model.file.from_json_content_id(
                json_field(object_, "content")
            ),
        )
    elif kind == "export":
        return WorkspaceRequestExport(
            handle=destack._generated.protocol.root.from_json_root_id(
                json_field(object_, "handle")
            ),
            request=destack._generated.protocol.workspace.artifact.export.from_json_export_request(
                json_field(object_, "request")
            ),
        )
    elif kind == "query":
        return WorkspaceRequestQuery(
            query=destack._generated.protocol.query.from_json_workspace_query(
                json_field(object_, "query")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "WorkspaceRequest",
    "encode_workspace_request",
    "decode_workspace_request",
    "to_json_workspace_request",
    "from_json_workspace_request",
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
