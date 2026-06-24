# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.mir.tree.node
import destack._generated.mir.tree.value

@dataclass(frozen=True, slots=True)
class Call:
    """Shared call facts for one call-like instruction or terminator."""

    # the call arguments
    arguments: destack._generated.mir.tree.value.ValueSlice
    # the signature type for the callee
    signature: destack._generated.mir.tree.node.LocalNodeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Call: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Call: ...

def encode_call(writer: BinaryWriter, value: Call) -> None: ...
def decode_call(reader: BinaryReader) -> Call: ...
def to_json_call(value: Call) -> Json: ...
def from_json_call(value: Json) -> Call: ...

__all__ = [
    "Call",
    "encode_call",
    "decode_call",
    "to_json_call",
    "from_json_call",
]
