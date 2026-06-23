# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.source.file

if TYPE_CHECKING:
    from destack._generated.source.file import (
        Content,
    )


@dataclass(frozen=True, slots=True)
class ArtifactSidecarLabel:
    """One stable sidecar label crossing bridge boundaries."""

    """Label key."""
    key: str
    """Label value."""
    value: str


def encode_artifact_sidecar_label(writer: Writer, value: ArtifactSidecarLabel) -> None:
    writer.write_string(value.key)
    writer.write_string(value.value)


def decode_artifact_sidecar_label(reader: Reader) -> ArtifactSidecarLabel:
    field_0 = reader.read_string()
    field_1 = reader.read_string()

    return ArtifactSidecarLabel(
        key=field_0,
        value=field_1,
    )


@dataclass(frozen=True, slots=True)
class ArtifactSidecar:
    """One named artifact sidecar crossing bridge boundaries."""

    """Sidecar name."""
    name: str
    """Stable labels describing this sidecar."""
    labels: Sequence[ArtifactSidecarLabel]
    """Sidecar content."""
    content: Content


def encode_artifact_sidecar(writer: Writer, value: ArtifactSidecar) -> None:
    writer.write_string(value.name)
    writer.write_unsigned(len(value.labels))
    for item_0 in value.labels:
        encode_artifact_sidecar_label(writer, item_0)
    destack._generated.source.file.encode_content(writer, value.content)


def decode_artifact_sidecar(reader: Reader) -> ArtifactSidecar:
    field_0 = reader.read_string()
    field_1 = [
        decode_artifact_sidecar_label(reader) for _ in range(reader.read_number())
    ]
    field_2 = destack._generated.source.file.decode_content(reader)

    return ArtifactSidecar(
        name=field_0,
        labels=field_1,
        content=field_2,
    )


__all__ = [
    "ArtifactSidecarLabel",
    "encode_artifact_sidecar_label",
    "decode_artifact_sidecar_label",
    "ArtifactSidecar",
    "encode_artifact_sidecar",
    "decode_artifact_sidecar",
]
