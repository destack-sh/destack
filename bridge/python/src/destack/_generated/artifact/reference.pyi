# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.artifact.core.key
import destack._generated.artifact.core.version

@dataclass(frozen=True, slots=True)
class ArtifactReference:
    """Stable reference to one artifact payload."""

    # artifact key
    key: destack._generated.artifact.core.key.ArtifactKey
    # artifact version
    version: destack._generated.artifact.core.version.ArtifactVersion

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ArtifactReference: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ArtifactReference: ...

def encode_artifact_reference(
    writer: BinaryWriter, value: ArtifactReference
) -> None: ...
def decode_artifact_reference(reader: BinaryReader) -> ArtifactReference: ...
def to_json_artifact_reference(value: ArtifactReference) -> Json: ...
def from_json_artifact_reference(value: Json) -> ArtifactReference: ...

__all__ = [
    "ArtifactReference",
    "encode_artifact_reference",
    "decode_artifact_reference",
    "to_json_artifact_reference",
    "from_json_artifact_reference",
]
