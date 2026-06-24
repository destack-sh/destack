# generated bridge target, do not edit

from __future__ import annotations

import builtins

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    bytes_from_json,
    bytes_to_json,
    json_array,
    json_field,
    json_object,
    json_optional,
)

import destack._generated.artifact.core.dependency
import destack._generated.artifact.core.version
import destack._generated.artifact.table.entry
import destack._generated.core.string
import destack._generated.source.diagnostic.collector
import destack._generated.source.file.model.file


@dataclass(frozen=True, slots=True)
class ArtifactRecord:
    """Self-contained transport record for one exact artifact."""

    # the exact artifact version
    version: destack._generated.artifact.core.version.ArtifactVersion
    # the predecessor artifact this record was incrementally built from
    base: destack._generated.artifact.core.version.ArtifactVersion | None
    # the serialized artifact payload
    payload: builtins.bytes | bytearray | Sequence[int]
    # string ids needed to interpret interned ids in the payload
    strings: Sequence[destack._generated.core.string.StringId]
    # the exact artifact dependencies
    dependencies: Sequence[
        destack._generated.artifact.core.dependency.ArtifactDependency
    ]
    # source files this artifact transitively depends on
    sources: Sequence[destack._generated.source.file.model.file.FileId]
    # diagnostics recorded for this artifact version
    diagnostics: destack._generated.source.diagnostic.collector.DiagnosticCollection
    # artifact sidecars recorded for this artifact version
    sidecars: Sequence[destack._generated.artifact.table.entry.ArtifactSidecar]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_record(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ArtifactRecord:
        """Decode one ArtifactRecord."""
        return decode_artifact_record(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_record(self)

    @classmethod
    def from_json(cls, value: Json) -> ArtifactRecord:
        """Return one ArtifactRecord from one JSON value."""
        return from_json_artifact_record(value)


def encode_artifact_record(writer: BinaryWriter, value: ArtifactRecord) -> None:
    """Encode one ArtifactRecord."""
    destack._generated.artifact.core.version.encode_artifact_version(
        writer, value.version
    )
    if value.base is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.artifact.core.version.encode_artifact_version(
            writer, value.base
        )
    writer.write_byte_slice(value.payload)
    writer.write_unsigned(len(value.strings))
    for item_value_strings_0 in value.strings:
        destack._generated.core.string.encode_string_id(writer, item_value_strings_0)
    writer.write_unsigned(len(value.dependencies))
    for item_value_dependencies_0 in value.dependencies:
        destack._generated.artifact.core.dependency.encode_artifact_dependency(
            writer, item_value_dependencies_0
        )
    writer.write_unsigned(len(value.sources))
    for item_value_sources_0 in value.sources:
        destack._generated.source.file.model.file.encode_file_id(
            writer, item_value_sources_0
        )
    destack._generated.source.diagnostic.collector.encode_diagnostic_collection(
        writer, value.diagnostics
    )
    writer.write_unsigned(len(value.sidecars))
    for item_value_sidecars_0 in value.sidecars:
        destack._generated.artifact.table.entry.encode_artifact_sidecar(
            writer, item_value_sidecars_0
        )


def decode_artifact_record(reader: BinaryReader) -> ArtifactRecord:
    """Decode one ArtifactRecord."""
    version = destack._generated.artifact.core.version.decode_artifact_version(reader)
    base = reader.read_option(
        lambda: destack._generated.artifact.core.version.decode_artifact_version(reader)
    )
    payload = reader.read_byte_slice()
    strings = [
        destack._generated.core.string.decode_string_id(reader)
        for _ in range(reader.read_number())
    ]
    dependencies = [
        destack._generated.artifact.core.dependency.decode_artifact_dependency(reader)
        for _ in range(reader.read_number())
    ]
    sources = [
        destack._generated.source.file.model.file.decode_file_id(reader)
        for _ in range(reader.read_number())
    ]
    diagnostics = (
        destack._generated.source.diagnostic.collector.decode_diagnostic_collection(
            reader
        )
    )
    sidecars = [
        destack._generated.artifact.table.entry.decode_artifact_sidecar(reader)
        for _ in range(reader.read_number())
    ]

    return ArtifactRecord(
        version=version,
        base=base,
        payload=payload,
        strings=strings,
        dependencies=dependencies,
        sources=sources,
        diagnostics=diagnostics,
        sidecars=sidecars,
    )


def to_json_artifact_record(value: ArtifactRecord) -> Json:
    """Return one JSON value for one ArtifactRecord."""
    return {
        "version": destack._generated.artifact.core.version.to_json_artifact_version(
            value.version
        ),
        **(
            {}
            if value.base is None
            else {
                "base": destack._generated.artifact.core.version.to_json_artifact_version(
                    value.base
                )
            }
        ),
        "payload": bytes_to_json(value.payload),
        "strings": [
            destack._generated.core.string.to_json_string_id(item_0)
            for item_0 in value.strings
        ],
        "dependencies": [
            destack._generated.artifact.core.dependency.to_json_artifact_dependency(
                item_0
            )
            for item_0 in value.dependencies
        ],
        "sources": [
            destack._generated.source.file.model.file.to_json_file_id(item_0)
            for item_0 in value.sources
        ],
        "diagnostics": destack._generated.source.diagnostic.collector.to_json_diagnostic_collection(
            value.diagnostics
        ),
        "sidecars": [
            destack._generated.artifact.table.entry.to_json_artifact_sidecar(item_0)
            for item_0 in value.sidecars
        ],
    }


def from_json_artifact_record(value: Json) -> ArtifactRecord:
    """Return one ArtifactRecord from one JSON value."""
    object_ = json_object(value)

    return ArtifactRecord(
        version=destack._generated.artifact.core.version.from_json_artifact_version(
            json_field(object_, "version")
        ),
        base=json_optional(
            object_,
            "base",
            lambda value: (
                destack._generated.artifact.core.version.from_json_artifact_version(
                    value
                )
            ),
        ),
        payload=bytes_from_json(json_field(object_, "payload")),
        strings=[
            destack._generated.core.string.from_json_string_id(item_0)
            for item_0 in json_array(json_field(object_, "strings"))
        ],
        dependencies=[
            destack._generated.artifact.core.dependency.from_json_artifact_dependency(
                item_0
            )
            for item_0 in json_array(json_field(object_, "dependencies"))
        ],
        sources=[
            destack._generated.source.file.model.file.from_json_file_id(item_0)
            for item_0 in json_array(json_field(object_, "sources"))
        ],
        diagnostics=destack._generated.source.diagnostic.collector.from_json_diagnostic_collection(
            json_field(object_, "diagnostics")
        ),
        sidecars=[
            destack._generated.artifact.table.entry.from_json_artifact_sidecar(item_0)
            for item_0 in json_array(json_field(object_, "sidecars"))
        ],
    )


__all__ = [
    "ArtifactRecord",
    "encode_artifact_record",
    "decode_artifact_record",
    "to_json_artifact_record",
    "from_json_artifact_record",
]
