# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.mir.tree.node
import destack._generated.mir.tree.type

@dataclass(frozen=True, slots=True)
class Local:
    """Local variable (stack slot) in a function."""

    # the type of the value stored in this slot
    ty: destack._generated.mir.tree.node.LocalNodeId
    # whether this local can be mutated after initialization
    mutability: destack._generated.mir.tree.type.Mutability

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Local: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Local: ...

def encode_local(writer: BinaryWriter, value: Local) -> None: ...
def decode_local(reader: BinaryReader) -> Local: ...
def to_json_local(value: Local) -> Json: ...
def from_json_local(value: Json) -> Local: ...

__all__ = [
    "Local",
    "encode_local",
    "decode_local",
    "to_json_local",
    "from_json_local",
]
