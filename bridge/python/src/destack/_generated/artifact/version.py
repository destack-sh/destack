# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.artifact.key

if TYPE_CHECKING:
    from destack._generated.artifact.key import (
        ArtifactKey,
    )


@dataclass(frozen=True, slots=True)
class ArtifactVersion:
    """External artifact version crossing bridge boundaries."""

    """Semantic artifact slot."""
    key: ArtifactKey
    """Exact semantic fingerprint."""
    fingerprint: str


def encode_artifact_version(writer: Writer, value: ArtifactVersion) -> None:
    destack._generated.artifact.key.encode_artifact_key(writer, value.key)
    writer.write_string(value.fingerprint)


def decode_artifact_version(reader: Reader) -> ArtifactVersion:
    field_0 = destack._generated.artifact.key.decode_artifact_key(reader)
    field_1 = reader.read_string()

    return ArtifactVersion(
        key=field_0,
        fingerprint=field_1,
    )


__all__ = [
    "ArtifactVersion",
    "encode_artifact_version",
    "decode_artifact_version",
]
