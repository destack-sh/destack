# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class SourceMap:
    """One emitted or linked source map."""

    # the source map version
    version: int
    # the emitted file name when one exists
    file: str | None
    # the source root when one exists
    source_root: str | None
    # the mapped source names
    sources: Sequence[str]
    # the embedded source contents when they exist
    sources_content: Sequence[str | None] | None
    # the recorded symbol names
    names: Sequence[str]
    # the VLQ mapping payload
    mappings: str
    # the debug id when one exists
    debug_id: str | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SourceMap: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SourceMap: ...

def encode_source_map(writer: BinaryWriter, value: SourceMap) -> None: ...
def decode_source_map(reader: BinaryReader) -> SourceMap: ...
def to_json_source_map(value: SourceMap) -> Json: ...
def from_json_source_map(value: Json) -> SourceMap: ...

__all__ = [
    "SourceMap",
    "encode_source_map",
    "decode_source_map",
    "to_json_source_map",
    "from_json_source_map",
]
