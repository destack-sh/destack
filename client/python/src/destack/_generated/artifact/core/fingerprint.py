# generated client target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json, json_int

"""Deterministic identity of one artifact's complete semantic dependencies."""
ArtifactFingerprint: typing.TypeAlias = int


def encode_artifact_fingerprint(
    writer: BinaryWriter, value: ArtifactFingerprint
) -> None:
    """Encode one ArtifactFingerprint."""
    writer.write_unsigned(value)


def decode_artifact_fingerprint(reader: BinaryReader) -> ArtifactFingerprint:
    """Decode one ArtifactFingerprint."""
    return reader.read_unsigned()


def to_json_artifact_fingerprint(value: ArtifactFingerprint) -> Json:
    """Return one JSON value for one ArtifactFingerprint."""
    return value


def from_json_artifact_fingerprint(value: Json) -> ArtifactFingerprint:
    """Return one ArtifactFingerprint from one JSON value."""
    return json_int(value)


__all__ = [
    "ArtifactFingerprint",
    "encode_artifact_fingerprint",
    "decode_artifact_fingerprint",
    "to_json_artifact_fingerprint",
    "from_json_artifact_fingerprint",
]
