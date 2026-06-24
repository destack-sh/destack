# generated client target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

"""Deterministic identity of one artifact's complete semantic dependencies."""
ArtifactFingerprint: typing.TypeAlias = int

def encode_artifact_fingerprint(
    writer: BinaryWriter, value: ArtifactFingerprint
) -> None: ...
def decode_artifact_fingerprint(reader: BinaryReader) -> ArtifactFingerprint: ...
def to_json_artifact_fingerprint(value: ArtifactFingerprint) -> Json: ...
def from_json_artifact_fingerprint(value: Json) -> ArtifactFingerprint: ...

__all__ = [
    "ArtifactFingerprint",
    "encode_artifact_fingerprint",
    "decode_artifact_fingerprint",
    "to_json_artifact_fingerprint",
    "from_json_artifact_fingerprint",
]
