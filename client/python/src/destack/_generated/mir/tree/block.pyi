# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string
import destack._generated.mir.tree.node
import destack._generated.mir.tree.parameter

@dataclass(frozen=True, slots=True)
class Block:
    """A basic block is a sequence of instructions with."""

    # optional explicit block label
    name: destack._generated.core.string.StringId | None
    # SSA parameters passed from predecessor blocks
    parameters: Sequence[destack._generated.mir.tree.parameter.BlockParameter]
    # instructions in execution order
    instructions: Sequence[destack._generated.mir.tree.node.LocalNodeId]
    # how control flow leaves this block
    terminator: destack._generated.mir.tree.node.LocalNodeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Block: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Block: ...

def encode_block(writer: BinaryWriter, value: Block) -> None: ...
def decode_block(reader: BinaryReader) -> Block: ...
def to_json_block(value: Block) -> Json: ...
def from_json_block(value: Json) -> Block: ...

__all__ = [
    "Block",
    "encode_block",
    "decode_block",
    "to_json_block",
    "from_json_block",
]
