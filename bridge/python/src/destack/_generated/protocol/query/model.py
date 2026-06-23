# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.notification
import destack._generated.protocol.payload
import destack._generated.protocol.query.core.protocol
import destack._generated.protocol.query.core.target
import destack._generated.protocol.repository.config.formatter
import destack._generated.protocol.repository.revision
import destack._generated.protocol.root
import destack._generated.protocol.source.diagnostic.diagnostic
import destack._generated.protocol.source.file.model.file
import destack._generated.protocol.source.file.model.profile
import destack._generated.protocol.source.file.path.uri
import destack._generated.protocol.workspace.file.image

if TYPE_CHECKING:
    from destack._generated.protocol.notification import (
        DiagnosticBatch,
    )

    from destack._generated.protocol.payload import (
        BinaryPayload,
    )

    from destack._generated.protocol.query.core.protocol import (
        QueryRequest,
        QueryResponse,
    )

    from destack._generated.protocol.query.core.target import (
        QueryModule,
    )

    from destack._generated.protocol.repository.config.formatter import (
        FormatterOptions,
    )

    from destack._generated.protocol.repository.revision import (
        Revision,
    )

    from destack._generated.protocol.root import (
        RootId,
    )

    from destack._generated.protocol.source.diagnostic.diagnostic import (
        Diagnostic,
    )

    from destack._generated.protocol.source.file.model.file import (
        FileId,
    )

    from destack._generated.protocol.source.file.model.profile import (
        ProfileId,
    )

    from destack._generated.protocol.source.file.path.uri import (
        Uri,
    )

    from destack._generated.protocol.workspace.file.image import (
        FileImage,
    )


@dataclass(frozen=True, slots=True)
class WorkspaceQueryDiagnostics:
    """Request diagnostics snapshot."""

    handle: RootId
    kind: Literal["diagnostics"] = "diagnostics"


@dataclass(frozen=True, slots=True)
class WorkspaceQueryDiagnosticSnapshots:
    """Request rich diagnostics with file images."""

    handle: RootId
    kind: Literal["diagnosticSnapshots"] = "diagnosticSnapshots"


@dataclass(frozen=True, slots=True)
class WorkspaceQueryFileDiagnostics:
    """Request diagnostics for one file."""

    """Root handle."""
    handle: RootId
    """Source path."""
    path: str
    kind: Literal["fileDiagnostics"] = "fileDiagnostics"


@dataclass(frozen=True, slots=True)
class WorkspaceQueryFileOpen:
    """Request whether one source file is open."""

    """Root handle."""
    handle: RootId
    """Source path."""
    path: str
    kind: Literal["fileOpen"] = "fileOpen"


@dataclass(frozen=True, slots=True)
class WorkspaceQueryCurrentRevision:
    """Request the current semantic revision."""

    handle: RootId
    kind: Literal["currentRevision"] = "currentRevision"


@dataclass(frozen=True, slots=True)
class WorkspaceQueryRootSnapshot:
    """Request query context for one root."""

    """Root handle."""
    handle: RootId
    """Target name used to select query profiles."""
    target: str | None
    kind: Literal["rootSnapshot"] = "rootSnapshot"


@dataclass(frozen=True, slots=True)
class WorkspaceQueryFileSnapshot:
    """Request a source file snapshot."""

    """Root handle."""
    handle: RootId
    """Snapshot request."""
    request: FileSnapshotRequest
    kind: Literal["fileSnapshot"] = "fileSnapshot"


@dataclass(frozen=True, slots=True)
class WorkspaceQueryFileImages:
    """Request source file images for a revision."""

    """Root handle."""
    handle: RootId
    """File image request."""
    request: FileImagesRequest
    kind: Literal["fileImages"] = "fileImages"


@dataclass(frozen=True, slots=True)
class WorkspaceQueryExecute:
    """Execute a query."""

    """Root handle."""
    handle: RootId
    """Encoded query request payload."""
    request: QueryRequestPayload
    kind: Literal["execute"] = "execute"


@dataclass(frozen=True, slots=True)
class WorkspaceQueryExecuteBatch:
    """Execute a batch of queries."""

    """Root handle."""
    handle: RootId
    """Encoded query request payloads."""
    requests: Sequence[QueryRequestPayload]
    kind: Literal["executeBatch"] = "executeBatch"


"""Query request payloads."""
WorkspaceQuery: TypeAlias = (
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


def encode_workspace_query(writer: Writer, value: WorkspaceQuery) -> None:
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
        for item_0 in value.requests:
            encode_query_request_payload(writer, item_0)
    else:
        raise SerdeError("unknown enum variant")


def decode_workspace_query(reader: Reader) -> WorkspaceQuery:
    variant = reader.read_number()

    if variant == 0:
        field_0 = destack._generated.protocol.root.decode_root_id(reader)

        return WorkspaceQueryDiagnostics(
            handle=field_0,
        )
    elif variant == 1:
        field_0 = destack._generated.protocol.root.decode_root_id(reader)

        return WorkspaceQueryDiagnosticSnapshots(
            handle=field_0,
        )
    elif variant == 2:
        field_0 = destack._generated.protocol.root.decode_root_id(reader)
        field_1 = reader.read_string()

        return WorkspaceQueryFileDiagnostics(
            handle=field_0,
            path=field_1,
        )
    elif variant == 3:
        field_0 = destack._generated.protocol.root.decode_root_id(reader)
        field_1 = reader.read_string()

        return WorkspaceQueryFileOpen(
            handle=field_0,
            path=field_1,
        )
    elif variant == 4:
        field_0 = destack._generated.protocol.root.decode_root_id(reader)

        return WorkspaceQueryCurrentRevision(
            handle=field_0,
        )
    elif variant == 5:
        field_0 = destack._generated.protocol.root.decode_root_id(reader)
        field_1 = reader.read_option(lambda: reader.read_string())

        return WorkspaceQueryRootSnapshot(
            handle=field_0,
            target=field_1,
        )
    elif variant == 6:
        field_0 = destack._generated.protocol.root.decode_root_id(reader)
        field_1 = decode_file_snapshot_request(reader)

        return WorkspaceQueryFileSnapshot(
            handle=field_0,
            request=field_1,
        )
    elif variant == 7:
        field_0 = destack._generated.protocol.root.decode_root_id(reader)
        field_1 = decode_file_images_request(reader)

        return WorkspaceQueryFileImages(
            handle=field_0,
            request=field_1,
        )
    elif variant == 8:
        field_0 = destack._generated.protocol.root.decode_root_id(reader)
        field_1 = decode_query_request_payload(reader)

        return WorkspaceQueryExecute(
            handle=field_0,
            request=field_1,
        )
    elif variant == 9:
        field_0 = destack._generated.protocol.root.decode_root_id(reader)
        field_1 = [
            decode_query_request_payload(reader) for _ in range(reader.read_number())
        ]

        return WorkspaceQueryExecuteBatch(
            handle=field_0,
            requests=field_1,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class FileSnapshotRequest:
    """Request for one source file snapshot."""

    """Path to the source file."""
    path: str
    """Target name used to select the query profile."""
    target: str | None


def encode_file_snapshot_request(writer: Writer, value: FileSnapshotRequest) -> None:
    writer.write_string(value.path)
    if value.target is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.target)


def decode_file_snapshot_request(reader: Reader) -> FileSnapshotRequest:
    field_0 = reader.read_string()
    field_1 = reader.read_option(lambda: reader.read_string())

    return FileSnapshotRequest(
        path=field_0,
        target=field_1,
    )


@dataclass(frozen=True, slots=True)
class FileImagesRequest:
    """Request for source file images in one revision."""

    """Revision containing the requested files."""
    revision: Revision
    """File ids to resolve."""
    file_ids: Sequence[FileId]


def encode_file_images_request(writer: Writer, value: FileImagesRequest) -> None:
    destack._generated.protocol.repository.revision.encode_revision(
        writer, value.revision
    )
    writer.write_unsigned(len(value.file_ids))
    for item_0 in value.file_ids:
        destack._generated.protocol.source.file.model.file.encode_file_id(
            writer, item_0
        )


def decode_file_images_request(reader: Reader) -> FileImagesRequest:
    field_0 = destack._generated.protocol.repository.revision.decode_revision(reader)
    field_1 = [
        destack._generated.protocol.source.file.model.file.decode_file_id(reader)
        for _ in range(reader.read_number())
    ]

    return FileImagesRequest(
        revision=field_0,
        file_ids=field_1,
    )


@dataclass(frozen=True, slots=True)
class WorkspaceQueryResponseDiagnostics:
    """Diagnostics snapshot."""

    diagnostics: Sequence[DiagnosticBatch]
    kind: Literal["diagnostics"] = "diagnostics"


@dataclass(frozen=True, slots=True)
class WorkspaceQueryResponseDiagnosticSnapshots:
    """Rich diagnostics with file images."""

    diagnostic_snapshots: Sequence[DiagnosticSnapshot]
    kind: Literal["diagnosticSnapshots"] = "diagnosticSnapshots"


@dataclass(frozen=True, slots=True)
class WorkspaceQueryResponseFileDiagnostics:
    """Diagnostics for one file."""

    file_diagnostics: DiagnosticSnapshot | None
    kind: Literal["fileDiagnostics"] = "fileDiagnostics"


@dataclass(frozen=True, slots=True)
class WorkspaceQueryResponseFileOpen:
    """Whether one source file is open."""

    file_open: bool
    kind: Literal["fileOpen"] = "fileOpen"


@dataclass(frozen=True, slots=True)
class WorkspaceQueryResponseCurrentRevision:
    """The current semantic revision."""

    current_revision: Revision
    kind: Literal["currentRevision"] = "currentRevision"


@dataclass(frozen=True, slots=True)
class WorkspaceQueryResponseRootSnapshot:
    """Query context for one root."""

    root_snapshot: RootSnapshot
    kind: Literal["rootSnapshot"] = "rootSnapshot"


@dataclass(frozen=True, slots=True)
class WorkspaceQueryResponseFileSnapshot:
    """Source file snapshot."""

    file_snapshot: FileSnapshot | None
    kind: Literal["fileSnapshot"] = "fileSnapshot"


@dataclass(frozen=True, slots=True)
class WorkspaceQueryResponseFileImages:
    """Source file images."""

    file_images: Sequence[FileImage]
    kind: Literal["fileImages"] = "fileImages"


@dataclass(frozen=True, slots=True)
class WorkspaceQueryResponseQuery:
    """Encoded query response payload."""

    query: QueryResponsePayload
    kind: Literal["query"] = "query"


@dataclass(frozen=True, slots=True)
class WorkspaceQueryResponseQueryBatch:
    """Encoded query batch response payloads."""

    query_batch: Sequence[QueryResponsePayload]
    kind: Literal["queryBatch"] = "queryBatch"


"""Query response payloads."""
WorkspaceQueryResponse: TypeAlias = (
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
    writer: Writer, value: WorkspaceQueryResponse
) -> None:
    if value.kind == "diagnostics":
        writer.write_unsigned(0)
        writer.write_unsigned(len(value.diagnostics))
        for item_0 in value.diagnostics:
            destack._generated.protocol.notification.encode_diagnostic_batch(
                writer, item_0
            )
    elif value.kind == "diagnosticSnapshots":
        writer.write_unsigned(1)
        writer.write_unsigned(len(value.diagnostic_snapshots))
        for item_0 in value.diagnostic_snapshots:
            encode_diagnostic_snapshot(writer, item_0)
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
        destack._generated.protocol.repository.revision.encode_revision(
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
        for item_0 in value.file_images:
            destack._generated.protocol.workspace.file.image.encode_file_image(
                writer, item_0
            )
    elif value.kind == "query":
        writer.write_unsigned(8)
        encode_query_response_payload(writer, value.query)
    elif value.kind == "queryBatch":
        writer.write_unsigned(9)
        writer.write_unsigned(len(value.query_batch))
        for item_0 in value.query_batch:
            encode_query_response_payload(writer, item_0)
    else:
        raise SerdeError("unknown enum variant")


def decode_workspace_query_response(reader: Reader) -> WorkspaceQueryResponse:
    variant = reader.read_number()

    if variant == 0:
        return WorkspaceQueryResponseDiagnostics(
            diagnostics=[
                destack._generated.protocol.notification.decode_diagnostic_batch(reader)
                for _ in range(reader.read_number())
            ]
        )
    elif variant == 1:
        return WorkspaceQueryResponseDiagnosticSnapshots(
            diagnostic_snapshots=[
                decode_diagnostic_snapshot(reader) for _ in range(reader.read_number())
            ]
        )
    elif variant == 2:
        return WorkspaceQueryResponseFileDiagnostics(
            file_diagnostics=reader.read_option(
                lambda: decode_diagnostic_snapshot(reader)
            )
        )
    elif variant == 3:
        return WorkspaceQueryResponseFileOpen(file_open=reader.read_bool())
    elif variant == 4:
        return WorkspaceQueryResponseCurrentRevision(
            current_revision=destack._generated.protocol.repository.revision.decode_revision(
                reader
            )
        )
    elif variant == 5:
        return WorkspaceQueryResponseRootSnapshot(
            root_snapshot=decode_root_snapshot(reader)
        )
    elif variant == 6:
        return WorkspaceQueryResponseFileSnapshot(
            file_snapshot=reader.read_option(lambda: decode_file_snapshot(reader))
        )
    elif variant == 7:
        return WorkspaceQueryResponseFileImages(
            file_images=[
                destack._generated.protocol.workspace.file.image.decode_file_image(
                    reader
                )
                for _ in range(reader.read_number())
            ]
        )
    elif variant == 8:
        return WorkspaceQueryResponseQuery(query=decode_query_response_payload(reader))
    elif variant == 9:
        return WorkspaceQueryResponseQueryBatch(
            query_batch=[
                decode_query_response_payload(reader)
                for _ in range(reader.read_number())
            ]
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class DiagnosticSnapshot:
    """Diagnostics and file image for one source file."""

    """Revision containing the diagnostics."""
    revision: Revision
    """File image used for range conversion."""
    file: FileImage
    """Diagnostic uri."""
    diagnostic_uri: Uri
    """Protocol file version when the file is open."""
    diagnostic_version: int | None
    """Diagnostics for the file."""
    diagnostics: Sequence[Diagnostic]


def encode_diagnostic_snapshot(writer: Writer, value: DiagnosticSnapshot) -> None:
    destack._generated.protocol.repository.revision.encode_revision(
        writer, value.revision
    )
    destack._generated.protocol.workspace.file.image.encode_file_image(
        writer, value.file
    )
    destack._generated.protocol.source.file.path.uri.encode_uri(
        writer, value.diagnostic_uri
    )
    if value.diagnostic_version is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_signed(value.diagnostic_version)
    writer.write_unsigned(len(value.diagnostics))
    for item_0 in value.diagnostics:
        destack._generated.protocol.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_0
        )


def decode_diagnostic_snapshot(reader: Reader) -> DiagnosticSnapshot:
    field_0 = destack._generated.protocol.repository.revision.decode_revision(reader)
    field_1 = destack._generated.protocol.workspace.file.image.decode_file_image(reader)
    field_2 = destack._generated.protocol.source.file.path.uri.decode_uri(reader)
    field_3 = reader.read_option(lambda: reader.read_signed_number())
    field_4 = [
        destack._generated.protocol.source.diagnostic.diagnostic.decode_diagnostic(
            reader
        )
        for _ in range(reader.read_number())
    ]

    return DiagnosticSnapshot(
        revision=field_0,
        file=field_1,
        diagnostic_uri=field_2,
        diagnostic_version=field_3,
        diagnostics=field_4,
    )


@dataclass(frozen=True, slots=True)
class RootSnapshot:
    """Query context for one root handle."""

    """Current semantic revision for the root."""
    revision: Revision
    """Profiles selected for the requested target."""
    profile_ids: Sequence[ProfileId]


def encode_root_snapshot(writer: Writer, value: RootSnapshot) -> None:
    destack._generated.protocol.repository.revision.encode_revision(
        writer, value.revision
    )
    writer.write_unsigned(len(value.profile_ids))
    for item_0 in value.profile_ids:
        destack._generated.protocol.source.file.model.profile.encode_profile_id(
            writer, item_0
        )


def decode_root_snapshot(reader: Reader) -> RootSnapshot:
    field_0 = destack._generated.protocol.repository.revision.decode_revision(reader)
    field_1 = [
        destack._generated.protocol.source.file.model.profile.decode_profile_id(reader)
        for _ in range(reader.read_number())
    ]

    return RootSnapshot(
        revision=field_0,
        profile_ids=field_1,
    )


@dataclass(frozen=True, slots=True)
class FileSnapshot:
    """Source file snapshot for editor adapters."""

    """Current semantic revision for the file root."""
    revision: Revision
    """The source file id."""
    file_id: FileId
    """Query module for the requested target."""
    module: QueryModule | None
    """Formatter options selected for the file."""
    formatter: FormatterOptions
    """File image used for range conversion."""
    file: FileImage


def encode_file_snapshot(writer: Writer, value: FileSnapshot) -> None:
    destack._generated.protocol.repository.revision.encode_revision(
        writer, value.revision
    )
    destack._generated.protocol.source.file.model.file.encode_file_id(
        writer, value.file_id
    )
    if value.module is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.protocol.query.core.target.encode_query_module(
            writer, value.module
        )
    destack._generated.protocol.repository.config.formatter.encode_formatter_options(
        writer, value.formatter
    )
    destack._generated.protocol.workspace.file.image.encode_file_image(
        writer, value.file
    )


def decode_file_snapshot(reader: Reader) -> FileSnapshot:
    field_0 = destack._generated.protocol.repository.revision.decode_revision(reader)
    field_1 = destack._generated.protocol.source.file.model.file.decode_file_id(reader)
    field_2 = reader.read_option(
        lambda: destack._generated.protocol.query.core.target.decode_query_module(
            reader
        )
    )
    field_3 = destack._generated.protocol.repository.config.formatter.decode_formatter_options(
        reader
    )
    field_4 = destack._generated.protocol.workspace.file.image.decode_file_image(reader)

    return FileSnapshot(
        revision=field_0,
        file_id=field_1,
        module=field_2,
        formatter=field_3,
        file=field_4,
    )


@dataclass(frozen=True, slots=True)
class QueryRequestBody:
    """Request payload for one query."""

    """Expected workspace semantic revision."""
    expected_revision: Revision | None
    """Query request."""
    request: QueryRequest


def encode_query_request_body(writer: Writer, value: QueryRequestBody) -> None:
    if value.expected_revision is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.protocol.repository.revision.encode_revision(
            writer, value.expected_revision
        )
    destack._generated.protocol.query.core.protocol.encode_query_request(
        writer, value.request
    )


def decode_query_request_body(reader: Reader) -> QueryRequestBody:
    field_0 = reader.read_option(
        lambda: destack._generated.protocol.repository.revision.decode_revision(reader)
    )
    field_1 = destack._generated.protocol.query.core.protocol.decode_query_request(
        reader
    )

    return QueryRequestBody(
        expected_revision=field_0,
        request=field_1,
    )


@dataclass(frozen=True, slots=True)
class QueryRequestPayload:
    """Encoded query request payload."""

    """Encoded request payload."""
    payload: BinaryPayload


def encode_query_request_payload(writer: Writer, value: QueryRequestPayload) -> None:
    destack._generated.protocol.payload.encode_binary_payload(writer, value.payload)


def decode_query_request_payload(reader: Reader) -> QueryRequestPayload:
    field_0 = destack._generated.protocol.payload.decode_binary_payload(reader)

    return QueryRequestPayload(
        payload=field_0,
    )


@dataclass(frozen=True, slots=True)
class QueryResponseBody:
    """Response payload for one query."""

    """Workspace semantic revision after request execution."""
    revision: Revision
    """Query response."""
    response: QueryResponse


def encode_query_response_body(writer: Writer, value: QueryResponseBody) -> None:
    destack._generated.protocol.repository.revision.encode_revision(
        writer, value.revision
    )
    destack._generated.protocol.query.core.protocol.encode_query_response(
        writer, value.response
    )


def decode_query_response_body(reader: Reader) -> QueryResponseBody:
    field_0 = destack._generated.protocol.repository.revision.decode_revision(reader)
    field_1 = destack._generated.protocol.query.core.protocol.decode_query_response(
        reader
    )

    return QueryResponseBody(
        revision=field_0,
        response=field_1,
    )


@dataclass(frozen=True, slots=True)
class QueryResponsePayload:
    """Encoded query response payload."""

    """Encoded response payload."""
    payload: BinaryPayload


def encode_query_response_payload(writer: Writer, value: QueryResponsePayload) -> None:
    destack._generated.protocol.payload.encode_binary_payload(writer, value.payload)


def decode_query_response_payload(reader: Reader) -> QueryResponsePayload:
    field_0 = destack._generated.protocol.payload.decode_binary_payload(reader)

    return QueryResponsePayload(
        payload=field_0,
    )


__all__ = [
    "WorkspaceQuery",
    "encode_workspace_query",
    "decode_workspace_query",
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
    "FileImagesRequest",
    "encode_file_images_request",
    "decode_file_images_request",
    "WorkspaceQueryResponse",
    "encode_workspace_query_response",
    "decode_workspace_query_response",
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
    "RootSnapshot",
    "encode_root_snapshot",
    "decode_root_snapshot",
    "FileSnapshot",
    "encode_file_snapshot",
    "decode_file_snapshot",
    "QueryRequestBody",
    "encode_query_request_body",
    "decode_query_request_body",
    "QueryRequestPayload",
    "encode_query_request_payload",
    "decode_query_request_payload",
    "QueryResponseBody",
    "encode_query_response_body",
    "decode_query_response_body",
    "QueryResponsePayload",
    "encode_query_response_payload",
    "decode_query_response_payload",
]
