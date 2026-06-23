# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.artifact.core.key
import destack._generated.protocol.artifact.core.version

if TYPE_CHECKING:
    from destack._generated.protocol.artifact.core.key import (
        ArtifactKey,
    )

    from destack._generated.protocol.artifact.core.version import (
        ArtifactVersion,
    )


@dataclass(frozen=True, slots=True)
class ArtifactReference:
    """Stable reference to one artifact payload."""

    """Artifact key."""
    key: ArtifactKey
    """Artifact version."""
    version: ArtifactVersion


def encode_artifact_reference(writer: Writer, value: ArtifactReference) -> None:
    destack._generated.protocol.artifact.core.key.encode_artifact_key(writer, value.key)
    destack._generated.protocol.artifact.core.version.encode_artifact_version(
        writer, value.version
    )


def decode_artifact_reference(reader: Reader) -> ArtifactReference:
    field_0 = destack._generated.protocol.artifact.core.key.decode_artifact_key(reader)
    field_1 = destack._generated.protocol.artifact.core.version.decode_artifact_version(
        reader
    )

    return ArtifactReference(
        key=field_0,
        version=field_1,
    )


__all__ = [
    "ArtifactReference",
    "encode_artifact_reference",
    "decode_artifact_reference",
]
