# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.mir.metadata.data
import destack._generated.mir.metadata.dispatch
import destack._generated.mir.metadata.drop
import destack._generated.mir.metadata.frame
import destack._generated.mir.metadata.function
import destack._generated.mir.metadata.layout
import destack._generated.mir.metadata.memory
import destack._generated.mir.metadata.type

@dataclass(frozen=True, slots=True)
class Metadata:
    """Structured MIR metadata domains."""

    # target data layout
    data_layout: destack._generated.mir.metadata.data.DataLayout
    # canonical type metadata
    types: destack._generated.mir.metadata.type.TypeMetadata
    # canonical layout metadata
    layout: destack._generated.mir.metadata.layout.LayoutMetadata
    # canonical dispatch metadata
    dispatch: destack._generated.mir.metadata.dispatch.DispatchMetadata
    # canonical drop metadata
    drop: destack._generated.mir.metadata.drop.DropMetadata
    # canonical frame metadata
    frame: destack._generated.mir.metadata.frame.FrameTable
    # derived function and call metadata
    functions: destack._generated.mir.metadata.function.FunctionMetadataTable
    # memory and alias metadata
    memory: destack._generated.mir.metadata.memory.MemoryMetadata

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Metadata: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Metadata: ...

def encode_metadata(writer: BinaryWriter, value: Metadata) -> None: ...
def decode_metadata(reader: BinaryReader) -> Metadata: ...
def to_json_metadata(value: Metadata) -> Json: ...
def from_json_metadata(value: Json) -> Metadata: ...

__all__ = [
    "Metadata",
    "encode_metadata",
    "decode_metadata",
    "to_json_metadata",
    "from_json_metadata",
]
