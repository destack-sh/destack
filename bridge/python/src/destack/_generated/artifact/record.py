# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.artifact.dependency
import destack._generated.artifact.sidecar
import destack._generated.artifact.version
import destack._generated.diagnostic.diagnostic

if TYPE_CHECKING:
    from destack._generated.artifact.dependency import (
        ArtifactDependency,
    )

    from destack._generated.artifact.sidecar import (
        ArtifactSidecar,
    )

    from destack._generated.artifact.version import (
        ArtifactVersion,
    )

    from destack._generated.diagnostic.diagnostic import (
        Diagnostic,
    )


@dataclass(frozen=True, slots=True)
class ArtifactString:
    """One interned string carried by an artifact record."""

    """Canonical lowercase hex string id."""
    id: str
    """Interned string text."""
    text: str


def encode_artifact_string(writer: Writer, value: ArtifactString) -> None:
    writer.write_string(value.id)
    writer.write_string(value.text)


def decode_artifact_string(reader: Reader) -> ArtifactString:
    field_0 = reader.read_string()
    field_1 = reader.read_string()

    return ArtifactString(
        id=field_0,
        text=field_1,
    )


@dataclass(frozen=True, slots=True)
class ArtifactRecord:
    """Self-contained raw artifact body crossing bridge boundaries."""

    """The exact artifact version."""
    version: ArtifactVersion
    """The predecessor artifact this record was incrementally built from."""
    base: ArtifactVersion | None
    """Serialized artifact payload bytes."""
    payload: bytes | bytearray | Sequence[int]
    """String pool needed to interpret interned ids in the payload."""
    strings: Sequence[ArtifactString]
    """Exact artifact dependencies."""
    dependencies: Sequence[ArtifactDependency]
    """Diagnostics recorded for this artifact version."""
    diagnostics: Sequence[Diagnostic]
    """Artifact sidecars recorded for this artifact version."""
    sidecars: Sequence[ArtifactSidecar]


def encode_artifact_record(writer: Writer, value: ArtifactRecord) -> None:
    destack._generated.artifact.version.encode_artifact_version(writer, value.version)
    if value.base is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.artifact.version.encode_artifact_version(writer, value.base)
    writer.write_byte_slice(value.payload)
    writer.write_unsigned(len(value.strings))
    for item_0 in value.strings:
        encode_artifact_string(writer, item_0)
    writer.write_unsigned(len(value.dependencies))
    for item_0 in value.dependencies:
        destack._generated.artifact.dependency.encode_artifact_dependency(
            writer, item_0
        )
    writer.write_unsigned(len(value.diagnostics))
    for item_0 in value.diagnostics:
        destack._generated.diagnostic.diagnostic.encode_diagnostic(writer, item_0)
    writer.write_unsigned(len(value.sidecars))
    for item_0 in value.sidecars:
        destack._generated.artifact.sidecar.encode_artifact_sidecar(writer, item_0)


def decode_artifact_record(reader: Reader) -> ArtifactRecord:
    field_0 = destack._generated.artifact.version.decode_artifact_version(reader)
    field_1 = reader.read_option(
        lambda: destack._generated.artifact.version.decode_artifact_version(reader)
    )
    field_2 = reader.read_byte_slice()
    field_3 = [decode_artifact_string(reader) for _ in range(reader.read_number())]
    field_4 = [
        destack._generated.artifact.dependency.decode_artifact_dependency(reader)
        for _ in range(reader.read_number())
    ]
    field_5 = [
        destack._generated.diagnostic.diagnostic.decode_diagnostic(reader)
        for _ in range(reader.read_number())
    ]
    field_6 = [
        destack._generated.artifact.sidecar.decode_artifact_sidecar(reader)
        for _ in range(reader.read_number())
    ]

    return ArtifactRecord(
        version=field_0,
        base=field_1,
        payload=field_2,
        strings=field_3,
        dependencies=field_4,
        diagnostics=field_5,
        sidecars=field_6,
    )


__all__ = [
    "ArtifactString",
    "encode_artifact_string",
    "decode_artifact_string",
    "ArtifactRecord",
    "encode_artifact_record",
    "decode_artifact_record",
]
