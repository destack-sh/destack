# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_bool,
    json_field,
    json_int,
    json_object,
    json_string,
)

import destack._generated.artifact.reference
import destack._generated.source.file.model.file


@dataclass(frozen=True, slots=True)
class ExportRequest:
    """Request to materialize derived outputs on the workspace host."""

    # artifact to export
    artifact: destack._generated.artifact.reference.ArtifactReference
    # output directory on the workspace host
    directory: str
    # whether existing files may be overwritten
    overwrite: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_export_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ExportRequest:
        """Decode one ExportRequest."""
        return decode_export_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_export_request(self)

    @classmethod
    def from_json(cls, value: Json) -> ExportRequest:
        """Return one ExportRequest from one JSON value."""
        return from_json_export_request(value)


def encode_export_request(writer: BinaryWriter, value: ExportRequest) -> None:
    """Encode one ExportRequest."""
    destack._generated.artifact.reference.encode_artifact_reference(
        writer, value.artifact
    )
    writer.write_string(value.directory)
    writer.write_bool(value.overwrite)


def decode_export_request(reader: BinaryReader) -> ExportRequest:
    """Decode one ExportRequest."""
    artifact = destack._generated.artifact.reference.decode_artifact_reference(reader)
    directory = reader.read_string()
    overwrite = reader.read_bool()

    return ExportRequest(
        artifact=artifact,
        directory=directory,
        overwrite=overwrite,
    )


def to_json_export_request(value: ExportRequest) -> Json:
    """Return one JSON value for one ExportRequest."""
    return {
        "artifact": destack._generated.artifact.reference.to_json_artifact_reference(
            value.artifact
        ),
        "directory": value.directory,
        "overwrite": value.overwrite,
    }


def from_json_export_request(value: Json) -> ExportRequest:
    """Return one ExportRequest from one JSON value."""
    object_ = json_object(value)

    return ExportRequest(
        artifact=destack._generated.artifact.reference.from_json_artifact_reference(
            json_field(object_, "artifact")
        ),
        directory=json_string(json_field(object_, "directory")),
        overwrite=json_bool(json_field(object_, "overwrite")),
    )


@dataclass(frozen=True, slots=True)
class ExportResult:
    """Result of materializing derived outputs."""

    # files written on the workspace host
    files: Sequence[ExportedFile]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_export_result(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ExportResult:
        """Decode one ExportResult."""
        return decode_export_result(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_export_result(self)

    @classmethod
    def from_json(cls, value: Json) -> ExportResult:
        """Return one ExportResult from one JSON value."""
        return from_json_export_result(value)


def encode_export_result(writer: BinaryWriter, value: ExportResult) -> None:
    """Encode one ExportResult."""
    writer.write_unsigned(len(value.files))
    for item_value_files_0 in value.files:
        encode_exported_file(writer, item_value_files_0)


def decode_export_result(reader: BinaryReader) -> ExportResult:
    """Decode one ExportResult."""
    files = [decode_exported_file(reader) for _ in range(reader.read_number())]

    return ExportResult(
        files=files,
    )


def to_json_export_result(value: ExportResult) -> Json:
    """Return one JSON value for one ExportResult."""
    return {
        "files": [to_json_exported_file(item_0) for item_0 in value.files],
    }


def from_json_export_result(value: Json) -> ExportResult:
    """Return one ExportResult from one JSON value."""
    object_ = json_object(value)

    return ExportResult(
        files=[
            from_json_exported_file(item_0)
            for item_0 in json_array(json_field(object_, "files"))
        ],
    )


@dataclass(frozen=True, slots=True)
class ExportedFile:
    """One file written by export."""

    # path written on the workspace host
    path: str
    # content written to the path
    content: destack._generated.source.file.model.file.ContentId
    # number of bytes written
    size_bytes: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_exported_file(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ExportedFile:
        """Decode one ExportedFile."""
        return decode_exported_file(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_exported_file(self)

    @classmethod
    def from_json(cls, value: Json) -> ExportedFile:
        """Return one ExportedFile from one JSON value."""
        return from_json_exported_file(value)


def encode_exported_file(writer: BinaryWriter, value: ExportedFile) -> None:
    """Encode one ExportedFile."""
    writer.write_string(value.path)
    destack._generated.source.file.model.file.encode_content_id(writer, value.content)
    writer.write_unsigned(value.size_bytes)


def decode_exported_file(reader: BinaryReader) -> ExportedFile:
    """Decode one ExportedFile."""
    path = reader.read_string()
    content = destack._generated.source.file.model.file.decode_content_id(reader)
    size_bytes = reader.read_number()

    return ExportedFile(
        path=path,
        content=content,
        size_bytes=size_bytes,
    )


def to_json_exported_file(value: ExportedFile) -> Json:
    """Return one JSON value for one ExportedFile."""
    return {
        "path": value.path,
        "content": destack._generated.source.file.model.file.to_json_content_id(
            value.content
        ),
        "sizeBytes": value.size_bytes,
    }


def from_json_exported_file(value: Json) -> ExportedFile:
    """Return one ExportedFile from one JSON value."""
    object_ = json_object(value)

    return ExportedFile(
        path=json_string(json_field(object_, "path")),
        content=destack._generated.source.file.model.file.from_json_content_id(
            json_field(object_, "content")
        ),
        size_bytes=json_int(json_field(object_, "sizeBytes")),
    )


__all__ = [
    "ExportRequest",
    "encode_export_request",
    "decode_export_request",
    "to_json_export_request",
    "from_json_export_request",
    "ExportResult",
    "encode_export_result",
    "decode_export_result",
    "to_json_export_result",
    "from_json_export_result",
    "ExportedFile",
    "encode_exported_file",
    "decode_exported_file",
    "to_json_exported_file",
    "from_json_exported_file",
]
