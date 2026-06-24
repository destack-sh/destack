# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.artifact.core.fingerprint
import destack._generated.artifact.core.key

@dataclass(frozen=True, slots=True)
class ArtifactVersion:
    """One exact live artifact version."""

    # the semantic artifact slot
    key: destack._generated.artifact.core.key.ArtifactKey
    # the exact semantic fingerprint
    fingerprint: destack._generated.artifact.core.fingerprint.ArtifactFingerprint

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ArtifactVersion: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ArtifactVersion: ...

def encode_artifact_version(writer: BinaryWriter, value: ArtifactVersion) -> None: ...
def decode_artifact_version(reader: BinaryReader) -> ArtifactVersion: ...
def to_json_artifact_version(value: ArtifactVersion) -> Json: ...
def from_json_artifact_version(value: Json) -> ArtifactVersion: ...

__all__ = [
    "ArtifactVersion",
    "encode_artifact_version",
    "decode_artifact_version",
    "to_json_artifact_version",
    "from_json_artifact_version",
]
