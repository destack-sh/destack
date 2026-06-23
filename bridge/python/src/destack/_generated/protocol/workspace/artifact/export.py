# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.artifact.reference
import destack._generated.protocol.source.file.model.file

if TYPE_CHECKING:
    from destack._generated.protocol.artifact.reference import (
        ArtifactReference,
    )

    from destack._generated.protocol.source.file.model.file import (
        ContentId,
    )


@dataclass(frozen=True, slots=True)
class ExportRequest:
    """Request to materialize derived outputs on the workspace host."""

    """Artifact to export."""
    artifact: ArtifactReference
    """Output directory on the workspace host."""
    directory: str
    """Whether existing files may be overwritten."""
    overwrite: bool


def encode_export_request(writer: Writer, value: ExportRequest) -> None:
    destack._generated.protocol.artifact.reference.encode_artifact_reference(
        writer, value.artifact
    )
    writer.write_string(value.directory)
    writer.write_bool(value.overwrite)


def decode_export_request(reader: Reader) -> ExportRequest:
    field_0 = destack._generated.protocol.artifact.reference.decode_artifact_reference(
        reader
    )
    field_1 = reader.read_string()
    field_2 = reader.read_bool()

    return ExportRequest(
        artifact=field_0,
        directory=field_1,
        overwrite=field_2,
    )


@dataclass(frozen=True, slots=True)
class ExportResult:
    """Result of materializing derived outputs."""

    """Files written on the workspace host."""
    files: Sequence[ExportedFile]


def encode_export_result(writer: Writer, value: ExportResult) -> None:
    writer.write_unsigned(len(value.files))
    for item_0 in value.files:
        encode_exported_file(writer, item_0)


def decode_export_result(reader: Reader) -> ExportResult:
    field_0 = [decode_exported_file(reader) for _ in range(reader.read_number())]

    return ExportResult(
        files=field_0,
    )


@dataclass(frozen=True, slots=True)
class ExportedFile:
    """One file written by export."""

    """Path written on the workspace host."""
    path: str
    """Content written to the path."""
    content: ContentId
    """Number of bytes written."""
    size_bytes: int


def encode_exported_file(writer: Writer, value: ExportedFile) -> None:
    writer.write_string(value.path)
    destack._generated.protocol.source.file.model.file.encode_content_id(
        writer, value.content
    )
    writer.write_unsigned(value.size_bytes)


def decode_exported_file(reader: Reader) -> ExportedFile:
    field_0 = reader.read_string()
    field_1 = destack._generated.protocol.source.file.model.file.decode_content_id(
        reader
    )
    field_2 = reader.read_number()

    return ExportedFile(
        path=field_0,
        content=field_1,
        size_bytes=field_2,
    )


__all__ = [
    "ExportRequest",
    "encode_export_request",
    "decode_export_request",
    "ExportResult",
    "encode_export_result",
    "decode_export_result",
    "ExportedFile",
    "encode_exported_file",
    "decode_exported_file",
]
