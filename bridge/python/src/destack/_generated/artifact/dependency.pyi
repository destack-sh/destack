# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, Writer

import destack._generated.artifact.version
import destack._generated.source.file

if TYPE_CHECKING:
    from destack._generated.artifact.version import (
        ArtifactVersion,
    )

    from destack._generated.source.file import (
        ContentId,
        FileId,
    )

@dataclass(frozen=True, slots=True)
class ArtifactDependencyArtifact:
    """Another exact artifact version."""

    """The exact artifact version depended on."""
    version: ArtifactVersion
    kind: Literal["artifact"] = "artifact"

@dataclass(frozen=True, slots=True)
class ArtifactDependencySource:
    """One exact primitive source observation."""

    """The source file id."""
    file: FileId
    """The exact source content id."""
    content: ContentId
    kind: Literal["source"] = "source"

"""One exact dependency read while building an artifact."""
ArtifactDependency: TypeAlias = ArtifactDependencyArtifact | ArtifactDependencySource

def encode_artifact_dependency(writer: Writer, value: ArtifactDependency) -> None: ...
def decode_artifact_dependency(reader: Reader) -> ArtifactDependency: ...

__all__ = [
    "ArtifactDependency",
    "encode_artifact_dependency",
    "decode_artifact_dependency",
    "ArtifactDependencyArtifact",
    "ArtifactDependencySource",
]
