# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

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


def encode_artifact_dependency(writer: Writer, value: ArtifactDependency) -> None:
    if value.kind == "artifact":
        writer.write_unsigned(0)
        destack._generated.artifact.version.encode_artifact_version(
            writer, value.version
        )
    elif value.kind == "source":
        writer.write_unsigned(1)
        destack._generated.source.file.encode_file_id(writer, value.file)
        destack._generated.source.file.encode_content_id(writer, value.content)
    else:
        raise SerdeError("unknown enum variant")


def decode_artifact_dependency(reader: Reader) -> ArtifactDependency:
    variant = reader.read_number()

    if variant == 0:
        field_0 = destack._generated.artifact.version.decode_artifact_version(reader)

        return ArtifactDependencyArtifact(
            version=field_0,
        )
    elif variant == 1:
        field_0 = destack._generated.source.file.decode_file_id(reader)
        field_1 = destack._generated.source.file.decode_content_id(reader)

        return ArtifactDependencySource(
            file=field_0,
            content=field_1,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


__all__ = [
    "ArtifactDependency",
    "encode_artifact_dependency",
    "decode_artifact_dependency",
    "ArtifactDependencyArtifact",
    "ArtifactDependencySource",
]
