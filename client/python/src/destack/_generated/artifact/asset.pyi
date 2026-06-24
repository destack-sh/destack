# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.artifact.map
import destack._generated.source.file.model.file
import destack._generated.source.file.model.type
import destack._generated.source.file.path.uri

@dataclass(frozen=True, slots=True)
class Asset:
    """One opaque linker input for a target."""

    # the asset file type
    file_type: destack._generated.source.file.model.type.FileType
    # the asset content identity
    content: destack._generated.source.file.model.file.ContentId
    # the source module URI when one exists
    source: destack._generated.source.file.path.uri.Uri | None
    # the source map when one exists
    map: destack._generated.artifact.map.SourceMap | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Asset: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Asset: ...

def encode_asset(writer: BinaryWriter, value: Asset) -> None: ...
def decode_asset(reader: BinaryReader) -> Asset: ...
def to_json_asset(value: Asset) -> Json: ...
def from_json_asset(value: Json) -> Asset: ...

__all__ = [
    "Asset",
    "encode_asset",
    "decode_asset",
    "to_json_asset",
    "from_json_asset",
]
