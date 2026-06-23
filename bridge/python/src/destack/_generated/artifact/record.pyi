# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, Writer

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

def encode_artifact_string(writer: Writer, value: ArtifactString) -> None: ...
def decode_artifact_string(reader: Reader) -> ArtifactString: ...

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

def encode_artifact_record(writer: Writer, value: ArtifactRecord) -> None: ...
def decode_artifact_record(reader: Reader) -> ArtifactRecord: ...

__all__ = [
    "ArtifactString",
    "encode_artifact_string",
    "decode_artifact_string",
    "ArtifactRecord",
    "encode_artifact_record",
    "decode_artifact_record",
]
