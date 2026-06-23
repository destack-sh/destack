# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, Writer

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

def encode_artifact_sidecar_label(
    writer: Writer, value: ArtifactSidecarLabel
) -> None: ...
def decode_artifact_sidecar_label(reader: Reader) -> ArtifactSidecarLabel: ...

@dataclass(frozen=True, slots=True)
class ArtifactSidecar:
    """One named artifact sidecar crossing bridge boundaries."""

    """Sidecar name."""
    name: str
    """Stable labels describing this sidecar."""
    labels: Sequence[ArtifactSidecarLabel]
    """Sidecar content."""
    content: Content

def encode_artifact_sidecar(writer: Writer, value: ArtifactSidecar) -> None: ...
def decode_artifact_sidecar(reader: Reader) -> ArtifactSidecar: ...

__all__ = [
    "ArtifactSidecarLabel",
    "encode_artifact_sidecar_label",
    "decode_artifact_sidecar_label",
    "ArtifactSidecar",
    "encode_artifact_sidecar",
    "decode_artifact_sidecar",
]
