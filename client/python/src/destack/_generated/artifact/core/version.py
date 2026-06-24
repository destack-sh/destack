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

import destack._generated.artifact.core.fingerprint
import destack._generated.artifact.core.key


@dataclass(frozen=True, slots=True)
class ArtifactVersion:
    """One exact live artifact version."""

    # the semantic artifact slot
    key: destack._generated.artifact.core.key.ArtifactKey
    # the exact semantic fingerprint
    fingerprint: destack._generated.artifact.core.fingerprint.ArtifactFingerprint

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_version(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ArtifactVersion:
        """Decode one ArtifactVersion."""
        return decode_artifact_version(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_version(self)

    @classmethod
    def from_json(cls, value: Json) -> ArtifactVersion:
        """Return one ArtifactVersion from one JSON value."""
        return from_json_artifact_version(value)


def encode_artifact_version(writer: BinaryWriter, value: ArtifactVersion) -> None:
    """Encode one ArtifactVersion."""
    destack._generated.artifact.core.key.encode_artifact_key(writer, value.key)
    destack._generated.artifact.core.fingerprint.encode_artifact_fingerprint(
        writer, value.fingerprint
    )


def decode_artifact_version(reader: BinaryReader) -> ArtifactVersion:
    """Decode one ArtifactVersion."""
    key = destack._generated.artifact.core.key.decode_artifact_key(reader)
    fingerprint = (
        destack._generated.artifact.core.fingerprint.decode_artifact_fingerprint(reader)
    )

    return ArtifactVersion(
        key=key,
        fingerprint=fingerprint,
    )


def to_json_artifact_version(value: ArtifactVersion) -> Json:
    """Return one JSON value for one ArtifactVersion."""
    return {
        "key": destack._generated.artifact.core.key.to_json_artifact_key(value.key),
        "fingerprint": destack._generated.artifact.core.fingerprint.to_json_artifact_fingerprint(
            value.fingerprint
        ),
    }


def from_json_artifact_version(value: Json) -> ArtifactVersion:
    """Return one ArtifactVersion from one JSON value."""
    object_ = json_object(value)

    return ArtifactVersion(
        key=destack._generated.artifact.core.key.from_json_artifact_key(
            json_field(object_, "key")
        ),
        fingerprint=destack._generated.artifact.core.fingerprint.from_json_artifact_fingerprint(
            json_field(object_, "fingerprint")
        ),
    )


__all__ = [
    "ArtifactVersion",
    "encode_artifact_version",
    "decode_artifact_version",
    "to_json_artifact_version",
    "from_json_artifact_version",
]
