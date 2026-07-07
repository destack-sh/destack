# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_array,
    json_bool,
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.protocol.notification
import destack._generated.protocol.payload
import destack._generated.protocol.root
import destack._generated.protocol.workspace.file.image
import destack._generated.query.protocol.message
import destack._generated.query.protocol.target
import destack._generated.repository.config.formatter
import destack._generated.repository.revision
import destack._generated.source.diagnostic.diagnostic
import destack._generated.source.file.model.file
import destack._generated.source.file.model.profile
import destack._generated.source.file.path.uri


@dataclass(frozen=True, slots=True)
class WorkspaceQueryDiagnostics:
    """Request diagnostics snapshot."""

    handle: destack._generated.protocol.root.RootId
    kind: typing.Literal["diagnostics"] = "diagnostics"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_query(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_query(self)


@dataclass(frozen=True, slots=True)
class WorkspaceQueryDiagnosticSnapshots:
    """Request rich diagnostics with file images."""

    handle: destack._generated.protocol.root.RootId
    kind: typing.Literal["diagnosticSnapshots"] = "diagnosticSnapshots"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_query(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_query(self)


@dataclass(frozen=True, slots=True)
class WorkspaceQueryFileDiagnostics:
    """Request diagnostics for one file."""

    # root handle
    handle: destack._generated.protocol.root.RootId
    # source path
    path: str
    kind: typing.Literal["fileDiagnostics"] = "fileDiagnostics"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_query(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_query(self)


@dataclass(frozen=True, slots=True)
class WorkspaceQueryFileOpen:
    """Request whether one source file is open."""

    # root handle
    handle: destack._generated.protocol.root.RootId
    # source path
    path: str
    kind: typing.Literal["fileOpen"] = "fileOpen"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_query(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_query(self)


@dataclass(frozen=True, slots=True)
class WorkspaceQueryCurrentRevision:
    """Request the current semantic revision."""

    handle: destack._generated.protocol.root.RootId
    kind: typing.Literal["currentRevision"] = "currentRevision"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_query(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_query(self)


@dataclass(frozen=True, slots=True)
class WorkspaceQueryRootSnapshot:
    """Request query context for one root."""

    # root handle
    handle: destack._generated.protocol.root.RootId
    # target name used to select query profiles
    target: str | None
    kind: typing.Literal["rootSnapshot"] = "rootSnapshot"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_query(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_query(self)


@dataclass(frozen=True, slots=True)
class WorkspaceQueryFileSnapshot:
    """Request a source file snapshot."""

    # root handle
    handle: destack._generated.protocol.root.RootId
    # snapshot request
    request: FileSnapshotRequest
    kind: typing.Literal["fileSnapshot"] = "fileSnapshot"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_query(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_query(self)


@dataclass(frozen=True, slots=True)
class WorkspaceQueryFileImages:
    """Request source file images for a revision."""

    # root handle
    handle: destack._generated.protocol.root.RootId
    # file image request
    request: FileImagesRequest
    kind: typing.Literal["fileImages"] = "fileImages"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_query(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_query(self)


@dataclass(frozen=True, slots=True)
class WorkspaceQueryExecute:
    """Execute a query."""

    # root handle
    handle: destack._generated.protocol.root.RootId
    # encoded query request payload
    request: QueryRequestPayload
    kind: typing.Literal["execute"] = "execute"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_query(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_query(self)


@dataclass(frozen=True, slots=True)
class WorkspaceQueryExecuteBatch:
    """Execute a batch of queries."""

    # root handle
    handle: destack._generated.protocol.root.RootId
    # encoded query request payloads
    requests: Sequence[QueryRequestPayload]
    kind: typing.Literal["executeBatch"] = "executeBatch"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_query(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_query(self)


"""Query request payloads."""
WorkspaceQuery: typing.TypeAlias = (
    WorkspaceQueryDiagnostics
    | WorkspaceQueryDiagnosticSnapshots
    | WorkspaceQueryFileDiagnostics
    | WorkspaceQueryFileOpen
    | WorkspaceQueryCurrentRevision
    | WorkspaceQueryRootSnapshot
    | WorkspaceQueryFileSnapshot
    | WorkspaceQueryFileImages
    | WorkspaceQueryExecute
    | WorkspaceQueryExecuteBatch
)


def encode_workspace_query(writer: BinaryWriter, value: WorkspaceQuery) -> None:
    """Encode one WorkspaceQuery."""
    if value.kind == "diagnostics":
        writer.write_unsigned(0)
        destack._generated.protocol.root.encode_root_id(writer, value.handle)
    elif value.kind == "diagnosticSnapshots":
        writer.write_unsigned(1)
        destack._generated.protocol.root.encode_root_id(writer, value.handle)
    elif value.kind == "fileDiagnostics":
        writer.write_unsigned(2)
        destack._generated.protocol.root.encode_root_id(writer, value.handle)
        writer.write_string(value.path)
    elif value.kind == "fileOpen":
        writer.write_unsigned(3)
        destack._generated.protocol.root.encode_root_id(writer, value.handle)
        writer.write_string(value.path)
    elif value.kind == "currentRevision":
        writer.write_unsigned(4)
        destack._generated.protocol.root.encode_root_id(writer, value.handle)
    elif value.kind == "rootSnapshot":
        writer.write_unsigned(5)
        destack._generated.protocol.root.encode_root_id(writer, value.handle)
        if value.target is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            writer.write_string(value.target)
    elif value.kind == "fileSnapshot":
        writer.write_unsigned(6)
        destack._generated.protocol.root.encode_root_id(writer, value.handle)
        encode_file_snapshot_request(writer, value.request)
    elif value.kind == "fileImages":
        writer.write_unsigned(7)
        destack._generated.protocol.root.encode_root_id(writer, value.handle)
        encode_file_images_request(writer, value.request)
    elif value.kind == "execute":
        writer.write_unsigned(8)
        destack._generated.protocol.root.encode_root_id(writer, value.handle)
        encode_query_request_payload(writer, value.request)
    elif value.kind == "executeBatch":
        writer.write_unsigned(9)
        destack._generated.protocol.root.encode_root_id(writer, value.handle)
        writer.write_unsigned(len(value.requests))
        for item_value_requests_0 in value.requests:
            encode_query_request_payload(writer, item_value_requests_0)
    else:
        raise SerdeError("unknown enum variant")


def decode_workspace_query(reader: BinaryReader) -> WorkspaceQuery:
    """Decode one WorkspaceQuery."""
    variant = reader.read_number()

    if variant == 0:
        handle = destack._generated.protocol.root.decode_root_id(reader)

        return WorkspaceQueryDiagnostics(
            handle=handle,
        )
    elif variant == 1:
        handle = destack._generated.protocol.root.decode_root_id(reader)

        return WorkspaceQueryDiagnosticSnapshots(
            handle=handle,
        )
    elif variant == 2:
        handle = destack._generated.protocol.root.decode_root_id(reader)
        path = reader.read_string()

        return WorkspaceQueryFileDiagnostics(
            handle=handle,
            path=path,
        )
    elif variant == 3:
        handle = destack._generated.protocol.root.decode_root_id(reader)
        path = reader.read_string()

        return WorkspaceQueryFileOpen(
            handle=handle,
            path=path,
        )
    elif variant == 4:
        handle = destack._generated.protocol.root.decode_root_id(reader)

        return WorkspaceQueryCurrentRevision(
            handle=handle,
        )
    elif variant == 5:
        handle = destack._generated.protocol.root.decode_root_id(reader)
        target = reader.read_option(lambda: reader.read_string())

        return WorkspaceQueryRootSnapshot(
            handle=handle,
            target=target,
        )
    elif variant == 6:
        handle = destack._generated.protocol.root.decode_root_id(reader)
        request = decode_file_snapshot_request(reader)

        return WorkspaceQueryFileSnapshot(
            handle=handle,
            request=request,
        )
    elif variant == 7:
        handle = destack._generated.protocol.root.decode_root_id(reader)
        request = decode_file_images_request(reader)

        return WorkspaceQueryFileImages(
            handle=handle,
            request=request,
        )
    elif variant == 8:
        handle = destack._generated.protocol.root.decode_root_id(reader)
        request = decode_query_request_payload(reader)

        return WorkspaceQueryExecute(
            handle=handle,
            request=request,
        )
    elif variant == 9:
        handle = destack._generated.protocol.root.decode_root_id(reader)
        requests = [
            decode_query_request_payload(reader) for _ in range(reader.read_number())
        ]

        return WorkspaceQueryExecuteBatch(
            handle=handle,
            requests=requests,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_workspace_query(value: WorkspaceQuery) -> Json:
    """Return one JSON value for one WorkspaceQuery."""
    if value.kind == "diagnostics":
        return {
            "kind": "diagnostics",
            "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
        }
    elif value.kind == "diagnosticSnapshots":
        return {
            "kind": "diagnosticSnapshots",
            "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
        }
    elif value.kind == "fileDiagnostics":
        return {
            "kind": "fileDiagnostics",
            "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
            "path": value.path,
        }
    elif value.kind == "fileOpen":
        return {
            "kind": "fileOpen",
            "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
            "path": value.path,
        }
    elif value.kind == "currentRevision":
        return {
            "kind": "currentRevision",
            "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
        }
    elif value.kind == "rootSnapshot":
        return {
            "kind": "rootSnapshot",
            "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
            **({} if value.target is None else {"target": value.target}),
        }
    elif value.kind == "fileSnapshot":
        return {
            "kind": "fileSnapshot",
            "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
            "request": to_json_file_snapshot_request(value.request),
        }
    elif value.kind == "fileImages":
        return {
            "kind": "fileImages",
            "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
            "request": to_json_file_images_request(value.request),
        }
    elif value.kind == "execute":
        return {
            "kind": "execute",
            "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
            "request": to_json_query_request_payload(value.request),
        }
    elif value.kind == "executeBatch":
        return {
            "kind": "executeBatch",
            "handle": destack._generated.protocol.root.to_json_root_id(value.handle),
            "requests": [
                to_json_query_request_payload(item_0) for item_0 in value.requests
            ],
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_workspace_query(value: Json) -> WorkspaceQuery:
    """Return one WorkspaceQuery from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "diagnostics":
        return WorkspaceQueryDiagnostics(
            handle=destack._generated.protocol.root.from_json_root_id(
                json_field(object_, "handle")
            ),
        )
    elif kind == "diagnosticSnapshots":
        return WorkspaceQueryDiagnosticSnapshots(
            handle=destack._generated.protocol.root.from_json_root_id(
                json_field(object_, "handle")
            ),
        )
    elif kind == "fileDiagnostics":
        return WorkspaceQueryFileDiagnostics(
            handle=destack._generated.protocol.root.from_json_root_id(
                json_field(object_, "handle")
            ),
            path=json_string(json_field(object_, "path")),
        )
    elif kind == "fileOpen":
        return WorkspaceQueryFileOpen(
            handle=destack._generated.protocol.root.from_json_root_id(
                json_field(object_, "handle")
            ),
            path=json_string(json_field(object_, "path")),
        )
    elif kind == "currentRevision":
        return WorkspaceQueryCurrentRevision(
            handle=destack._generated.protocol.root.from_json_root_id(
                json_field(object_, "handle")
            ),
        )
    elif kind == "rootSnapshot":
        return WorkspaceQueryRootSnapshot(
            handle=destack._generated.protocol.root.from_json_root_id(
                json_field(object_, "handle")
            ),
            target=json_optional(object_, "target", lambda value: json_string(value)),
        )
    elif kind == "fileSnapshot":
        return WorkspaceQueryFileSnapshot(
            handle=destack._generated.protocol.root.from_json_root_id(
                json_field(object_, "handle")
            ),
            request=from_json_file_snapshot_request(json_field(object_, "request")),
        )
    elif kind == "fileImages":
        return WorkspaceQueryFileImages(
            handle=destack._generated.protocol.root.from_json_root_id(
                json_field(object_, "handle")
            ),
            request=from_json_file_images_request(json_field(object_, "request")),
        )
    elif kind == "execute":
        return WorkspaceQueryExecute(
            handle=destack._generated.protocol.root.from_json_root_id(
                json_field(object_, "handle")
            ),
            request=from_json_query_request_payload(json_field(object_, "request")),
        )
    elif kind == "executeBatch":
        return WorkspaceQueryExecuteBatch(
            handle=destack._generated.protocol.root.from_json_root_id(
                json_field(object_, "handle")
            ),
            requests=[
                from_json_query_request_payload(item_0)
                for item_0 in json_array(json_field(object_, "requests"))
            ],
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class FileSnapshotRequest:
    """Request for one source file snapshot."""

    # path to the source file
    path: str
    # target name used to select the query profile
    target: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_file_snapshot_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FileSnapshotRequest:
        """Decode one FileSnapshotRequest."""
        return decode_file_snapshot_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_file_snapshot_request(self)

    @classmethod
    def from_json(cls, value: Json) -> FileSnapshotRequest:
        """Return one FileSnapshotRequest from one JSON value."""
        return from_json_file_snapshot_request(value)


def encode_file_snapshot_request(
    writer: BinaryWriter, value: FileSnapshotRequest
) -> None:
    """Encode one FileSnapshotRequest."""
    writer.write_string(value.path)
    if value.target is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.target)


def decode_file_snapshot_request(reader: BinaryReader) -> FileSnapshotRequest:
    """Decode one FileSnapshotRequest."""
    path = reader.read_string()
    target = reader.read_option(lambda: reader.read_string())

    return FileSnapshotRequest(
        path=path,
        target=target,
    )


def to_json_file_snapshot_request(value: FileSnapshotRequest) -> Json:
    """Return one JSON value for one FileSnapshotRequest."""
    return {
        "path": value.path,
        **({} if value.target is None else {"target": value.target}),
    }


def from_json_file_snapshot_request(value: Json) -> FileSnapshotRequest:
    """Return one FileSnapshotRequest from one JSON value."""
    object_ = json_object(value)

    return FileSnapshotRequest(
        path=json_string(json_field(object_, "path")),
        target=json_optional(object_, "target", lambda value: json_string(value)),
    )


@dataclass(frozen=True, slots=True)
class FileImagesRequest:
    """Request for source file images in one revision."""

    # revision containing the requested files
    revision: destack._generated.repository.revision.Revision
    # file ids to resolve
    file_ids: Sequence[destack._generated.source.file.model.file.FileId]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_file_images_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FileImagesRequest:
        """Decode one FileImagesRequest."""
        return decode_file_images_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_file_images_request(self)

    @classmethod
    def from_json(cls, value: Json) -> FileImagesRequest:
        """Return one FileImagesRequest from one JSON value."""
        return from_json_file_images_request(value)


def encode_file_images_request(writer: BinaryWriter, value: FileImagesRequest) -> None:
    """Encode one FileImagesRequest."""
    destack._generated.repository.revision.encode_revision(writer, value.revision)
    writer.write_unsigned(len(value.file_ids))
    for item_value_file_ids_0 in value.file_ids:
        destack._generated.source.file.model.file.encode_file_id(
            writer, item_value_file_ids_0
        )


def decode_file_images_request(reader: BinaryReader) -> FileImagesRequest:
    """Decode one FileImagesRequest."""
    revision = destack._generated.repository.revision.decode_revision(reader)
    file_ids = [
        destack._generated.source.file.model.file.decode_file_id(reader)
        for _ in range(reader.read_number())
    ]

    return FileImagesRequest(
        revision=revision,
        file_ids=file_ids,
    )


def to_json_file_images_request(value: FileImagesRequest) -> Json:
    """Return one JSON value for one FileImagesRequest."""
    return {
        "revision": destack._generated.repository.revision.to_json_revision(
            value.revision
        ),
        "fileIds": [
            destack._generated.source.file.model.file.to_json_file_id(item_0)
            for item_0 in value.file_ids
        ],
    }


def from_json_file_images_request(value: Json) -> FileImagesRequest:
    """Return one FileImagesRequest from one JSON value."""
    object_ = json_object(value)

    return FileImagesRequest(
        revision=destack._generated.repository.revision.from_json_revision(
            json_field(object_, "revision")
        ),
        file_ids=[
            destack._generated.source.file.model.file.from_json_file_id(item_0)
            for item_0 in json_array(json_field(object_, "fileIds"))
        ],
    )


@dataclass(frozen=True, slots=True)
class WorkspaceQueryResponseDiagnostics:
    """Diagnostics snapshot."""

    diagnostics: Sequence[destack._generated.protocol.notification.DiagnosticBatch]
    kind: typing.Literal["diagnostics"] = "diagnostics"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_query_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceQueryResponseDiagnosticSnapshots:
    """Rich diagnostics with file images."""

    diagnostic_snapshots: Sequence[DiagnosticSnapshot]
    kind: typing.Literal["diagnosticSnapshots"] = "diagnosticSnapshots"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_query_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceQueryResponseFileDiagnostics:
    """Diagnostics for one file."""

    file_diagnostics: DiagnosticSnapshot | None
    kind: typing.Literal["fileDiagnostics"] = "fileDiagnostics"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_query_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceQueryResponseFileOpen:
    """Whether one source file is open."""

    file_open: bool
    kind: typing.Literal["fileOpen"] = "fileOpen"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_query_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceQueryResponseCurrentRevision:
    """The current semantic revision."""

    current_revision: destack._generated.repository.revision.Revision
    kind: typing.Literal["currentRevision"] = "currentRevision"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_query_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceQueryResponseRootSnapshot:
    """Query context for one root."""

    root_snapshot: RootSnapshot
    kind: typing.Literal["rootSnapshot"] = "rootSnapshot"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_query_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceQueryResponseFileSnapshot:
    """Source file snapshot."""

    file_snapshot: FileSnapshot | None
    kind: typing.Literal["fileSnapshot"] = "fileSnapshot"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_query_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceQueryResponseFileImages:
    """Source file images."""

    file_images: Sequence[destack._generated.protocol.workspace.file.image.FileImage]
    kind: typing.Literal["fileImages"] = "fileImages"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_query_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceQueryResponseQuery:
    """Encoded query response payload."""

    query: QueryResponsePayload
    kind: typing.Literal["query"] = "query"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_query_response(self)


@dataclass(frozen=True, slots=True)
class WorkspaceQueryResponseQueryBatch:
    """Encoded query batch response payloads."""

    query_batch: Sequence[QueryResponsePayload]
    kind: typing.Literal["queryBatch"] = "queryBatch"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_query_response(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_query_response(self)


"""Query response payloads."""
WorkspaceQueryResponse: typing.TypeAlias = (
    WorkspaceQueryResponseDiagnostics
    | WorkspaceQueryResponseDiagnosticSnapshots
    | WorkspaceQueryResponseFileDiagnostics
    | WorkspaceQueryResponseFileOpen
    | WorkspaceQueryResponseCurrentRevision
    | WorkspaceQueryResponseRootSnapshot
    | WorkspaceQueryResponseFileSnapshot
    | WorkspaceQueryResponseFileImages
    | WorkspaceQueryResponseQuery
    | WorkspaceQueryResponseQueryBatch
)


def encode_workspace_query_response(
    writer: BinaryWriter, value: WorkspaceQueryResponse
) -> None:
    """Encode one WorkspaceQueryResponse."""
    if value.kind == "diagnostics":
        writer.write_unsigned(0)
        writer.write_unsigned(len(value.diagnostics))
        for item_value_diagnostics_0 in value.diagnostics:
            destack._generated.protocol.notification.encode_diagnostic_batch(
                writer, item_value_diagnostics_0
            )
    elif value.kind == "diagnosticSnapshots":
        writer.write_unsigned(1)
        writer.write_unsigned(len(value.diagnostic_snapshots))
        for item_value_diagnostic_snapshots_0 in value.diagnostic_snapshots:
            encode_diagnostic_snapshot(writer, item_value_diagnostic_snapshots_0)
    elif value.kind == "fileDiagnostics":
        writer.write_unsigned(2)
        if value.file_diagnostics is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_diagnostic_snapshot(writer, value.file_diagnostics)
    elif value.kind == "fileOpen":
        writer.write_unsigned(3)
        writer.write_bool(value.file_open)
    elif value.kind == "currentRevision":
        writer.write_unsigned(4)
        destack._generated.repository.revision.encode_revision(
            writer, value.current_revision
        )
    elif value.kind == "rootSnapshot":
        writer.write_unsigned(5)
        encode_root_snapshot(writer, value.root_snapshot)
    elif value.kind == "fileSnapshot":
        writer.write_unsigned(6)
        if value.file_snapshot is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_file_snapshot(writer, value.file_snapshot)
    elif value.kind == "fileImages":
        writer.write_unsigned(7)
        writer.write_unsigned(len(value.file_images))
        for item_value_file_images_0 in value.file_images:
            destack._generated.protocol.workspace.file.image.encode_file_image(
                writer, item_value_file_images_0
            )
    elif value.kind == "query":
        writer.write_unsigned(8)
        encode_query_response_payload(writer, value.query)
    elif value.kind == "queryBatch":
        writer.write_unsigned(9)
        writer.write_unsigned(len(value.query_batch))
        for item_value_query_batch_0 in value.query_batch:
            encode_query_response_payload(writer, item_value_query_batch_0)
    else:
        raise SerdeError("unknown enum variant")


def decode_workspace_query_response(reader: BinaryReader) -> WorkspaceQueryResponse:
    """Decode one WorkspaceQueryResponse."""
    variant = reader.read_number()

    if variant == 0:
        diagnostics = [
            destack._generated.protocol.notification.decode_diagnostic_batch(reader)
            for _ in range(reader.read_number())
        ]

        return WorkspaceQueryResponseDiagnostics(diagnostics=diagnostics)
    elif variant == 1:
        diagnostic_snapshots = [
            decode_diagnostic_snapshot(reader) for _ in range(reader.read_number())
        ]

        return WorkspaceQueryResponseDiagnosticSnapshots(
            diagnostic_snapshots=diagnostic_snapshots
        )
    elif variant == 2:
        file_diagnostics = reader.read_option(
            lambda: decode_diagnostic_snapshot(reader)
        )

        return WorkspaceQueryResponseFileDiagnostics(file_diagnostics=file_diagnostics)
    elif variant == 3:
        file_open = reader.read_bool()

        return WorkspaceQueryResponseFileOpen(file_open=file_open)
    elif variant == 4:
        current_revision = destack._generated.repository.revision.decode_revision(
            reader
        )

        return WorkspaceQueryResponseCurrentRevision(current_revision=current_revision)
    elif variant == 5:
        root_snapshot = decode_root_snapshot(reader)

        return WorkspaceQueryResponseRootSnapshot(root_snapshot=root_snapshot)
    elif variant == 6:
        file_snapshot = reader.read_option(lambda: decode_file_snapshot(reader))

        return WorkspaceQueryResponseFileSnapshot(file_snapshot=file_snapshot)
    elif variant == 7:
        file_images = [
            destack._generated.protocol.workspace.file.image.decode_file_image(reader)
            for _ in range(reader.read_number())
        ]

        return WorkspaceQueryResponseFileImages(file_images=file_images)
    elif variant == 8:
        query = decode_query_response_payload(reader)

        return WorkspaceQueryResponseQuery(query=query)
    elif variant == 9:
        query_batch = [
            decode_query_response_payload(reader) for _ in range(reader.read_number())
        ]

        return WorkspaceQueryResponseQueryBatch(query_batch=query_batch)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_workspace_query_response(value: WorkspaceQueryResponse) -> Json:
    """Return one JSON value for one WorkspaceQueryResponse."""
    if value.kind == "diagnostics":
        return {
            "kind": "diagnostics",
            "diagnostics": [
                destack._generated.protocol.notification.to_json_diagnostic_batch(
                    item_0
                )
                for item_0 in value.diagnostics
            ],
        }
    elif value.kind == "diagnosticSnapshots":
        return {
            "kind": "diagnosticSnapshots",
            "diagnostic_snapshots": [
                to_json_diagnostic_snapshot(item_0)
                for item_0 in value.diagnostic_snapshots
            ],
        }
    elif value.kind == "fileDiagnostics":
        return {
            "kind": "fileDiagnostics",
            "file_diagnostics": None
            if value.file_diagnostics is None
            else to_json_diagnostic_snapshot(value.file_diagnostics),
        }
    elif value.kind == "fileOpen":
        return {
            "kind": "fileOpen",
            "file_open": value.file_open,
        }
    elif value.kind == "currentRevision":
        return {
            "kind": "currentRevision",
            "current_revision": destack._generated.repository.revision.to_json_revision(
                value.current_revision
            ),
        }
    elif value.kind == "rootSnapshot":
        return {
            "kind": "rootSnapshot",
            "root_snapshot": to_json_root_snapshot(value.root_snapshot),
        }
    elif value.kind == "fileSnapshot":
        return {
            "kind": "fileSnapshot",
            "file_snapshot": None
            if value.file_snapshot is None
            else to_json_file_snapshot(value.file_snapshot),
        }
    elif value.kind == "fileImages":
        return {
            "kind": "fileImages",
            "file_images": [
                destack._generated.protocol.workspace.file.image.to_json_file_image(
                    item_0
                )
                for item_0 in value.file_images
            ],
        }
    elif value.kind == "query":
        return {
            "kind": "query",
            "query": to_json_query_response_payload(value.query),
        }
    elif value.kind == "queryBatch":
        return {
            "kind": "queryBatch",
            "query_batch": [
                to_json_query_response_payload(item_0) for item_0 in value.query_batch
            ],
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_workspace_query_response(value: Json) -> WorkspaceQueryResponse:
    """Return one WorkspaceQueryResponse from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "diagnostics":
        return WorkspaceQueryResponseDiagnostics(
            diagnostics=[
                destack._generated.protocol.notification.from_json_diagnostic_batch(
                    item_0
                )
                for item_0 in json_array(json_field(object_, "diagnostics"))
            ]
        )
    elif kind == "diagnosticSnapshots":
        return WorkspaceQueryResponseDiagnosticSnapshots(
            diagnostic_snapshots=[
                from_json_diagnostic_snapshot(item_0)
                for item_0 in json_array(json_field(object_, "diagnostic_snapshots"))
            ]
        )
    elif kind == "fileDiagnostics":
        return WorkspaceQueryResponseFileDiagnostics(
            file_diagnostics=None
            if json_field(object_, "file_diagnostics") is None
            else from_json_diagnostic_snapshot(json_field(object_, "file_diagnostics"))
        )
    elif kind == "fileOpen":
        return WorkspaceQueryResponseFileOpen(
            file_open=json_bool(json_field(object_, "file_open"))
        )
    elif kind == "currentRevision":
        return WorkspaceQueryResponseCurrentRevision(
            current_revision=destack._generated.repository.revision.from_json_revision(
                json_field(object_, "current_revision")
            )
        )
    elif kind == "rootSnapshot":
        return WorkspaceQueryResponseRootSnapshot(
            root_snapshot=from_json_root_snapshot(json_field(object_, "root_snapshot"))
        )
    elif kind == "fileSnapshot":
        return WorkspaceQueryResponseFileSnapshot(
            file_snapshot=None
            if json_field(object_, "file_snapshot") is None
            else from_json_file_snapshot(json_field(object_, "file_snapshot"))
        )
    elif kind == "fileImages":
        return WorkspaceQueryResponseFileImages(
            file_images=[
                destack._generated.protocol.workspace.file.image.from_json_file_image(
                    item_0
                )
                for item_0 in json_array(json_field(object_, "file_images"))
            ]
        )
    elif kind == "query":
        return WorkspaceQueryResponseQuery(
            query=from_json_query_response_payload(json_field(object_, "query"))
        )
    elif kind == "queryBatch":
        return WorkspaceQueryResponseQueryBatch(
            query_batch=[
                from_json_query_response_payload(item_0)
                for item_0 in json_array(json_field(object_, "query_batch"))
            ]
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class DiagnosticSnapshot:
    """Diagnostics and file image for one source file."""

    # revision containing the diagnostics
    revision: destack._generated.repository.revision.Revision
    # file image used for range conversion
    file: destack._generated.protocol.workspace.file.image.FileImage
    # diagnostic uri
    diagnostic_uri: destack._generated.source.file.path.uri.Uri
    # protocol file version when the file is open
    diagnostic_version: int | None
    # diagnostics for the file
    diagnostics: Sequence[destack._generated.source.diagnostic.diagnostic.Diagnostic]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_diagnostic_snapshot(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DiagnosticSnapshot:
        """Decode one DiagnosticSnapshot."""
        return decode_diagnostic_snapshot(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_diagnostic_snapshot(self)

    @classmethod
    def from_json(cls, value: Json) -> DiagnosticSnapshot:
        """Return one DiagnosticSnapshot from one JSON value."""
        return from_json_diagnostic_snapshot(value)


def encode_diagnostic_snapshot(writer: BinaryWriter, value: DiagnosticSnapshot) -> None:
    """Encode one DiagnosticSnapshot."""
    destack._generated.repository.revision.encode_revision(writer, value.revision)
    destack._generated.protocol.workspace.file.image.encode_file_image(
        writer, value.file
    )
    destack._generated.source.file.path.uri.encode_uri(writer, value.diagnostic_uri)
    if value.diagnostic_version is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_signed(value.diagnostic_version)
    writer.write_unsigned(len(value.diagnostics))
    for item_value_diagnostics_0 in value.diagnostics:
        destack._generated.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_value_diagnostics_0
        )


def decode_diagnostic_snapshot(reader: BinaryReader) -> DiagnosticSnapshot:
    """Decode one DiagnosticSnapshot."""
    revision = destack._generated.repository.revision.decode_revision(reader)
    file = destack._generated.protocol.workspace.file.image.decode_file_image(reader)
    diagnostic_uri = destack._generated.source.file.path.uri.decode_uri(reader)
    diagnostic_version = reader.read_option(lambda: reader.read_signed_number())
    diagnostics = [
        destack._generated.source.diagnostic.diagnostic.decode_diagnostic(reader)
        for _ in range(reader.read_number())
    ]

    return DiagnosticSnapshot(
        revision=revision,
        file=file,
        diagnostic_uri=diagnostic_uri,
        diagnostic_version=diagnostic_version,
        diagnostics=diagnostics,
    )


def to_json_diagnostic_snapshot(value: DiagnosticSnapshot) -> Json:
    """Return one JSON value for one DiagnosticSnapshot."""
    return {
        "revision": destack._generated.repository.revision.to_json_revision(
            value.revision
        ),
        "file": destack._generated.protocol.workspace.file.image.to_json_file_image(
            value.file
        ),
        "diagnosticUri": destack._generated.source.file.path.uri.to_json_uri(
            value.diagnostic_uri
        ),
        **(
            {}
            if value.diagnostic_version is None
            else {"diagnosticVersion": value.diagnostic_version}
        ),
        "diagnostics": [
            destack._generated.source.diagnostic.diagnostic.to_json_diagnostic(item_0)
            for item_0 in value.diagnostics
        ],
    }


def from_json_diagnostic_snapshot(value: Json) -> DiagnosticSnapshot:
    """Return one DiagnosticSnapshot from one JSON value."""
    object_ = json_object(value)

    return DiagnosticSnapshot(
        revision=destack._generated.repository.revision.from_json_revision(
            json_field(object_, "revision")
        ),
        file=destack._generated.protocol.workspace.file.image.from_json_file_image(
            json_field(object_, "file")
        ),
        diagnostic_uri=destack._generated.source.file.path.uri.from_json_uri(
            json_field(object_, "diagnosticUri")
        ),
        diagnostic_version=json_optional(
            object_, "diagnosticVersion", lambda value: json_int(value)
        ),
        diagnostics=[
            destack._generated.source.diagnostic.diagnostic.from_json_diagnostic(item_0)
            for item_0 in json_array(json_field(object_, "diagnostics"))
        ],
    )


@dataclass(frozen=True, slots=True)
class RootSnapshot:
    """Query context for one root handle."""

    # current semantic revision for the root
    revision: destack._generated.repository.revision.Revision
    # profiles selected for the requested target
    profile_ids: Sequence[destack._generated.source.file.model.profile.ProfileId]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_root_snapshot(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> RootSnapshot:
        """Decode one RootSnapshot."""
        return decode_root_snapshot(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_root_snapshot(self)

    @classmethod
    def from_json(cls, value: Json) -> RootSnapshot:
        """Return one RootSnapshot from one JSON value."""
        return from_json_root_snapshot(value)


def encode_root_snapshot(writer: BinaryWriter, value: RootSnapshot) -> None:
    """Encode one RootSnapshot."""
    destack._generated.repository.revision.encode_revision(writer, value.revision)
    writer.write_unsigned(len(value.profile_ids))
    for item_value_profile_ids_0 in value.profile_ids:
        destack._generated.source.file.model.profile.encode_profile_id(
            writer, item_value_profile_ids_0
        )


def decode_root_snapshot(reader: BinaryReader) -> RootSnapshot:
    """Decode one RootSnapshot."""
    revision = destack._generated.repository.revision.decode_revision(reader)
    profile_ids = [
        destack._generated.source.file.model.profile.decode_profile_id(reader)
        for _ in range(reader.read_number())
    ]

    return RootSnapshot(
        revision=revision,
        profile_ids=profile_ids,
    )


def to_json_root_snapshot(value: RootSnapshot) -> Json:
    """Return one JSON value for one RootSnapshot."""
    return {
        "revision": destack._generated.repository.revision.to_json_revision(
            value.revision
        ),
        "profileIds": [
            destack._generated.source.file.model.profile.to_json_profile_id(item_0)
            for item_0 in value.profile_ids
        ],
    }


def from_json_root_snapshot(value: Json) -> RootSnapshot:
    """Return one RootSnapshot from one JSON value."""
    object_ = json_object(value)

    return RootSnapshot(
        revision=destack._generated.repository.revision.from_json_revision(
            json_field(object_, "revision")
        ),
        profile_ids=[
            destack._generated.source.file.model.profile.from_json_profile_id(item_0)
            for item_0 in json_array(json_field(object_, "profileIds"))
        ],
    )


@dataclass(frozen=True, slots=True)
class FileSnapshot:
    """Source file snapshot for editor adapters."""

    # current semantic revision for the file root
    revision: destack._generated.repository.revision.Revision
    # the source file id
    file_id: destack._generated.source.file.model.file.FileId
    # module for the requested target
    module: destack._generated.query.protocol.target.Module | None
    # formatter options selected for the file
    formatter: destack._generated.repository.config.formatter.FormatterOptions
    # file image used for range conversion
    file: destack._generated.protocol.workspace.file.image.FileImage

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_file_snapshot(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FileSnapshot:
        """Decode one FileSnapshot."""
        return decode_file_snapshot(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_file_snapshot(self)

    @classmethod
    def from_json(cls, value: Json) -> FileSnapshot:
        """Return one FileSnapshot from one JSON value."""
        return from_json_file_snapshot(value)


def encode_file_snapshot(writer: BinaryWriter, value: FileSnapshot) -> None:
    """Encode one FileSnapshot."""
    destack._generated.repository.revision.encode_revision(writer, value.revision)
    destack._generated.source.file.model.file.encode_file_id(writer, value.file_id)
    if value.module is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.query.protocol.target.encode_module(writer, value.module)
    destack._generated.repository.config.formatter.encode_formatter_options(
        writer, value.formatter
    )
    destack._generated.protocol.workspace.file.image.encode_file_image(
        writer, value.file
    )


def decode_file_snapshot(reader: BinaryReader) -> FileSnapshot:
    """Decode one FileSnapshot."""
    revision = destack._generated.repository.revision.decode_revision(reader)
    file_id = destack._generated.source.file.model.file.decode_file_id(reader)
    module = reader.read_option(
        lambda: destack._generated.query.protocol.target.decode_module(reader)
    )
    formatter = destack._generated.repository.config.formatter.decode_formatter_options(
        reader
    )
    file = destack._generated.protocol.workspace.file.image.decode_file_image(reader)

    return FileSnapshot(
        revision=revision,
        file_id=file_id,
        module=module,
        formatter=formatter,
        file=file,
    )


def to_json_file_snapshot(value: FileSnapshot) -> Json:
    """Return one JSON value for one FileSnapshot."""
    return {
        "revision": destack._generated.repository.revision.to_json_revision(
            value.revision
        ),
        "fileId": destack._generated.source.file.model.file.to_json_file_id(
            value.file_id
        ),
        **(
            {}
            if value.module is None
            else {
                "module": destack._generated.query.protocol.target.to_json_module(
                    value.module
                )
            }
        ),
        "formatter": destack._generated.repository.config.formatter.to_json_formatter_options(
            value.formatter
        ),
        "file": destack._generated.protocol.workspace.file.image.to_json_file_image(
            value.file
        ),
    }


def from_json_file_snapshot(value: Json) -> FileSnapshot:
    """Return one FileSnapshot from one JSON value."""
    object_ = json_object(value)

    return FileSnapshot(
        revision=destack._generated.repository.revision.from_json_revision(
            json_field(object_, "revision")
        ),
        file_id=destack._generated.source.file.model.file.from_json_file_id(
            json_field(object_, "fileId")
        ),
        module=json_optional(
            object_,
            "module",
            lambda value: destack._generated.query.protocol.target.from_json_module(
                value
            ),
        ),
        formatter=destack._generated.repository.config.formatter.from_json_formatter_options(
            json_field(object_, "formatter")
        ),
        file=destack._generated.protocol.workspace.file.image.from_json_file_image(
            json_field(object_, "file")
        ),
    )


@dataclass(frozen=True, slots=True)
class QueryRequestBody:
    """Request payload for one query."""

    # expected workspace semantic revision
    expected_revision: destack._generated.repository.revision.Revision | None
    # query request
    request: destack._generated.query.protocol.message.QueryRequest

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request_body(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> QueryRequestBody:
        """Decode one QueryRequestBody."""
        return decode_query_request_body(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request_body(self)

    @classmethod
    def from_json(cls, value: Json) -> QueryRequestBody:
        """Return one QueryRequestBody from one JSON value."""
        return from_json_query_request_body(value)


def encode_query_request_body(writer: BinaryWriter, value: QueryRequestBody) -> None:
    """Encode one QueryRequestBody."""
    if value.expected_revision is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.repository.revision.encode_revision(
            writer, value.expected_revision
        )
    destack._generated.query.protocol.message.encode_query_request(
        writer, value.request
    )


def decode_query_request_body(reader: BinaryReader) -> QueryRequestBody:
    """Decode one QueryRequestBody."""
    expected_revision = reader.read_option(
        lambda: destack._generated.repository.revision.decode_revision(reader)
    )
    request = destack._generated.query.protocol.message.decode_query_request(reader)

    return QueryRequestBody(
        expected_revision=expected_revision,
        request=request,
    )


def to_json_query_request_body(value: QueryRequestBody) -> Json:
    """Return one JSON value for one QueryRequestBody."""
    return {
        **(
            {}
            if value.expected_revision is None
            else {
                "expectedRevision": destack._generated.repository.revision.to_json_revision(
                    value.expected_revision
                )
            }
        ),
        "request": destack._generated.query.protocol.message.to_json_query_request(
            value.request
        ),
    }


def from_json_query_request_body(value: Json) -> QueryRequestBody:
    """Return one QueryRequestBody from one JSON value."""
    object_ = json_object(value)

    return QueryRequestBody(
        expected_revision=json_optional(
            object_,
            "expectedRevision",
            lambda value: destack._generated.repository.revision.from_json_revision(
                value
            ),
        ),
        request=destack._generated.query.protocol.message.from_json_query_request(
            json_field(object_, "request")
        ),
    )


@dataclass(frozen=True, slots=True)
class QueryRequestPayload:
    """Encoded query request payload."""

    # encoded request payload
    payload: destack._generated.protocol.payload.BinaryPayload

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_request_payload(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> QueryRequestPayload:
        """Decode one QueryRequestPayload."""
        return decode_query_request_payload(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_request_payload(self)

    @classmethod
    def from_json(cls, value: Json) -> QueryRequestPayload:
        """Return one QueryRequestPayload from one JSON value."""
        return from_json_query_request_payload(value)


def encode_query_request_payload(
    writer: BinaryWriter, value: QueryRequestPayload
) -> None:
    """Encode one QueryRequestPayload."""
    destack._generated.protocol.payload.encode_binary_payload(writer, value.payload)


def decode_query_request_payload(reader: BinaryReader) -> QueryRequestPayload:
    """Decode one QueryRequestPayload."""
    payload = destack._generated.protocol.payload.decode_binary_payload(reader)

    return QueryRequestPayload(
        payload=payload,
    )


def to_json_query_request_payload(value: QueryRequestPayload) -> Json:
    """Return one JSON value for one QueryRequestPayload."""
    return {
        "payload": destack._generated.protocol.payload.to_json_binary_payload(
            value.payload
        ),
    }


def from_json_query_request_payload(value: Json) -> QueryRequestPayload:
    """Return one QueryRequestPayload from one JSON value."""
    object_ = json_object(value)

    return QueryRequestPayload(
        payload=destack._generated.protocol.payload.from_json_binary_payload(
            json_field(object_, "payload")
        ),
    )


@dataclass(frozen=True, slots=True)
class QueryResponseBody:
    """Response payload for one query."""

    # workspace semantic revision after request execution
    revision: destack._generated.repository.revision.Revision
    # query response
    response: destack._generated.query.protocol.message.QueryResponse

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response_body(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> QueryResponseBody:
        """Decode one QueryResponseBody."""
        return decode_query_response_body(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response_body(self)

    @classmethod
    def from_json(cls, value: Json) -> QueryResponseBody:
        """Return one QueryResponseBody from one JSON value."""
        return from_json_query_response_body(value)


def encode_query_response_body(writer: BinaryWriter, value: QueryResponseBody) -> None:
    """Encode one QueryResponseBody."""
    destack._generated.repository.revision.encode_revision(writer, value.revision)
    destack._generated.query.protocol.message.encode_query_response(
        writer, value.response
    )


def decode_query_response_body(reader: BinaryReader) -> QueryResponseBody:
    """Decode one QueryResponseBody."""
    revision = destack._generated.repository.revision.decode_revision(reader)
    response = destack._generated.query.protocol.message.decode_query_response(reader)

    return QueryResponseBody(
        revision=revision,
        response=response,
    )


def to_json_query_response_body(value: QueryResponseBody) -> Json:
    """Return one JSON value for one QueryResponseBody."""
    return {
        "revision": destack._generated.repository.revision.to_json_revision(
            value.revision
        ),
        "response": destack._generated.query.protocol.message.to_json_query_response(
            value.response
        ),
    }


def from_json_query_response_body(value: Json) -> QueryResponseBody:
    """Return one QueryResponseBody from one JSON value."""
    object_ = json_object(value)

    return QueryResponseBody(
        revision=destack._generated.repository.revision.from_json_revision(
            json_field(object_, "revision")
        ),
        response=destack._generated.query.protocol.message.from_json_query_response(
            json_field(object_, "response")
        ),
    )


@dataclass(frozen=True, slots=True)
class QueryResponsePayload:
    """Encoded query response payload."""

    # encoded response payload
    payload: destack._generated.protocol.payload.BinaryPayload

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_query_response_payload(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> QueryResponsePayload:
        """Decode one QueryResponsePayload."""
        return decode_query_response_payload(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_query_response_payload(self)

    @classmethod
    def from_json(cls, value: Json) -> QueryResponsePayload:
        """Return one QueryResponsePayload from one JSON value."""
        return from_json_query_response_payload(value)


def encode_query_response_payload(
    writer: BinaryWriter, value: QueryResponsePayload
) -> None:
    """Encode one QueryResponsePayload."""
    destack._generated.protocol.payload.encode_binary_payload(writer, value.payload)


def decode_query_response_payload(reader: BinaryReader) -> QueryResponsePayload:
    """Decode one QueryResponsePayload."""
    payload = destack._generated.protocol.payload.decode_binary_payload(reader)

    return QueryResponsePayload(
        payload=payload,
    )


def to_json_query_response_payload(value: QueryResponsePayload) -> Json:
    """Return one JSON value for one QueryResponsePayload."""
    return {
        "payload": destack._generated.protocol.payload.to_json_binary_payload(
            value.payload
        ),
    }


def from_json_query_response_payload(value: Json) -> QueryResponsePayload:
    """Return one QueryResponsePayload from one JSON value."""
    object_ = json_object(value)

    return QueryResponsePayload(
        payload=destack._generated.protocol.payload.from_json_binary_payload(
            json_field(object_, "payload")
        ),
    )


__all__ = [
    "WorkspaceQuery",
    "encode_workspace_query",
    "decode_workspace_query",
    "to_json_workspace_query",
    "from_json_workspace_query",
    "WorkspaceQueryDiagnostics",
    "WorkspaceQueryDiagnosticSnapshots",
    "WorkspaceQueryFileDiagnostics",
    "WorkspaceQueryFileOpen",
    "WorkspaceQueryCurrentRevision",
    "WorkspaceQueryRootSnapshot",
    "WorkspaceQueryFileSnapshot",
    "WorkspaceQueryFileImages",
    "WorkspaceQueryExecute",
    "WorkspaceQueryExecuteBatch",
    "FileSnapshotRequest",
    "encode_file_snapshot_request",
    "decode_file_snapshot_request",
    "to_json_file_snapshot_request",
    "from_json_file_snapshot_request",
    "FileImagesRequest",
    "encode_file_images_request",
    "decode_file_images_request",
    "to_json_file_images_request",
    "from_json_file_images_request",
    "WorkspaceQueryResponse",
    "encode_workspace_query_response",
    "decode_workspace_query_response",
    "to_json_workspace_query_response",
    "from_json_workspace_query_response",
    "WorkspaceQueryResponseDiagnostics",
    "WorkspaceQueryResponseDiagnosticSnapshots",
    "WorkspaceQueryResponseFileDiagnostics",
    "WorkspaceQueryResponseFileOpen",
    "WorkspaceQueryResponseCurrentRevision",
    "WorkspaceQueryResponseRootSnapshot",
    "WorkspaceQueryResponseFileSnapshot",
    "WorkspaceQueryResponseFileImages",
    "WorkspaceQueryResponseQuery",
    "WorkspaceQueryResponseQueryBatch",
    "DiagnosticSnapshot",
    "encode_diagnostic_snapshot",
    "decode_diagnostic_snapshot",
    "to_json_diagnostic_snapshot",
    "from_json_diagnostic_snapshot",
    "RootSnapshot",
    "encode_root_snapshot",
    "decode_root_snapshot",
    "to_json_root_snapshot",
    "from_json_root_snapshot",
    "FileSnapshot",
    "encode_file_snapshot",
    "decode_file_snapshot",
    "to_json_file_snapshot",
    "from_json_file_snapshot",
    "QueryRequestBody",
    "encode_query_request_body",
    "decode_query_request_body",
    "to_json_query_request_body",
    "from_json_query_request_body",
    "QueryRequestPayload",
    "encode_query_request_payload",
    "decode_query_request_payload",
    "to_json_query_request_payload",
    "from_json_query_request_payload",
    "QueryResponseBody",
    "encode_query_response_body",
    "decode_query_response_body",
    "to_json_query_response_body",
    "from_json_query_response_body",
    "QueryResponsePayload",
    "encode_query_response_payload",
    "decode_query_response_payload",
    "to_json_query_response_payload",
    "from_json_query_response_payload",
]
