# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.source.file.model.file

@dataclass(frozen=True, slots=True)
class ArtifactSidecar:
    """One named artifact sidecar."""

    # the sidecar name
    name: str
    # the stable labels describing this sidecar
    labels: Mapping[str, str]
    # the sidecar content
    content: destack._generated.source.file.model.file.Content

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ArtifactSidecar: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ArtifactSidecar: ...

def encode_artifact_sidecar(writer: BinaryWriter, value: ArtifactSidecar) -> None: ...
def decode_artifact_sidecar(reader: BinaryReader) -> ArtifactSidecar: ...
def to_json_artifact_sidecar(value: ArtifactSidecar) -> Json: ...
def from_json_artifact_sidecar(value: Json) -> ArtifactSidecar: ...

__all__ = [
    "ArtifactSidecar",
    "encode_artifact_sidecar",
    "decode_artifact_sidecar",
    "to_json_artifact_sidecar",
    "from_json_artifact_sidecar",
]
