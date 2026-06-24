# generated bridge target, do not edit

from __future__ import annotations

import builtins

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.artifact.core.dependency
import destack._generated.artifact.core.version
import destack._generated.artifact.table.entry
import destack._generated.core.string
import destack._generated.source.diagnostic.collector
import destack._generated.source.file.model.file

@dataclass(frozen=True, slots=True)
class ArtifactRecord:
    """Self-contained transport record for one exact artifact."""

    # the exact artifact version
    version: destack._generated.artifact.core.version.ArtifactVersion
    # the predecessor artifact this record was incrementally built from
    base: destack._generated.artifact.core.version.ArtifactVersion | None
    # the serialized artifact payload
    payload: builtins.bytes | bytearray | Sequence[int]
    # string ids needed to interpret interned ids in the payload
    strings: Sequence[destack._generated.core.string.StringId]
    # the exact artifact dependencies
    dependencies: Sequence[
        destack._generated.artifact.core.dependency.ArtifactDependency
    ]
    # source files this artifact transitively depends on
    sources: Sequence[destack._generated.source.file.model.file.FileId]
    # diagnostics recorded for this artifact version
    diagnostics: destack._generated.source.diagnostic.collector.DiagnosticCollection
    # artifact sidecars recorded for this artifact version
    sidecars: Sequence[destack._generated.artifact.table.entry.ArtifactSidecar]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ArtifactRecord: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ArtifactRecord: ...

def encode_artifact_record(writer: BinaryWriter, value: ArtifactRecord) -> None: ...
def decode_artifact_record(reader: BinaryReader) -> ArtifactRecord: ...
def to_json_artifact_record(value: ArtifactRecord) -> Json: ...
def from_json_artifact_record(value: Json) -> ArtifactRecord: ...

__all__ = [
    "ArtifactRecord",
    "encode_artifact_record",
    "decode_artifact_record",
    "to_json_artifact_record",
    "from_json_artifact_record",
]
