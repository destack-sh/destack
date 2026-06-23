# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.artifact.core.fingerprint
import destack._generated.protocol.artifact.core.key

if TYPE_CHECKING:
    from destack._generated.protocol.artifact.core.fingerprint import (
        ArtifactFingerprint,
    )

    from destack._generated.protocol.artifact.core.key import (
        ArtifactKey,
    )


@dataclass(frozen=True, slots=True)
class ArtifactVersion:
    """One exact live artifact version."""

    """The semantic artifact slot."""
    key: ArtifactKey
    """The exact semantic fingerprint."""
    fingerprint: ArtifactFingerprint


def encode_artifact_version(writer: Writer, value: ArtifactVersion) -> None:
    destack._generated.protocol.artifact.core.key.encode_artifact_key(writer, value.key)
    destack._generated.protocol.artifact.core.fingerprint.encode_artifact_fingerprint(
        writer, value.fingerprint
    )


def decode_artifact_version(reader: Reader) -> ArtifactVersion:
    field_0 = destack._generated.protocol.artifact.core.key.decode_artifact_key(reader)
    field_1 = destack._generated.protocol.artifact.core.fingerprint.decode_artifact_fingerprint(
        reader
    )

    return ArtifactVersion(
        key=field_0,
        fingerprint=field_1,
    )


__all__ = [
    "ArtifactVersion",
    "encode_artifact_version",
    "decode_artifact_version",
]
