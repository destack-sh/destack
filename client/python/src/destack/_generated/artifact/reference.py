# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_object,
)

import destack._generated.artifact.core.key
import destack._generated.artifact.core.version


@dataclass(frozen=True, slots=True)
class ArtifactReference:
    """Stable reference to one artifact payload."""

    # artifact key
    key: destack._generated.artifact.core.key.ArtifactKey
    # artifact version
    version: destack._generated.artifact.core.version.ArtifactVersion

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_reference(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ArtifactReference:
        """Decode one ArtifactReference."""
        return decode_artifact_reference(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_reference(self)

    @classmethod
    def from_json(cls, value: Json) -> ArtifactReference:
        """Return one ArtifactReference from one JSON value."""
        return from_json_artifact_reference(value)


def encode_artifact_reference(writer: BinaryWriter, value: ArtifactReference) -> None:
    """Encode one ArtifactReference."""
    destack._generated.artifact.core.key.encode_artifact_key(writer, value.key)
    destack._generated.artifact.core.version.encode_artifact_version(
        writer, value.version
    )


def decode_artifact_reference(reader: BinaryReader) -> ArtifactReference:
    """Decode one ArtifactReference."""
    key = destack._generated.artifact.core.key.decode_artifact_key(reader)
    version = destack._generated.artifact.core.version.decode_artifact_version(reader)

    return ArtifactReference(
        key=key,
        version=version,
    )


def to_json_artifact_reference(value: ArtifactReference) -> Json:
    """Return one JSON value for one ArtifactReference."""
    return {
        "key": destack._generated.artifact.core.key.to_json_artifact_key(value.key),
        "version": destack._generated.artifact.core.version.to_json_artifact_version(
            value.version
        ),
    }


def from_json_artifact_reference(value: Json) -> ArtifactReference:
    """Return one ArtifactReference from one JSON value."""
    object_ = json_object(value)

    return ArtifactReference(
        key=destack._generated.artifact.core.key.from_json_artifact_key(
            json_field(object_, "key")
        ),
        version=destack._generated.artifact.core.version.from_json_artifact_version(
            json_field(object_, "version")
        ),
    )


__all__ = [
    "ArtifactReference",
    "encode_artifact_reference",
    "decode_artifact_reference",
    "to_json_artifact_reference",
    "from_json_artifact_reference",
]
