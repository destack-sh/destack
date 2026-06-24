# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

from destack._impl.dir.tree.block import (
    BlockImpl,
)

import destack._generated.dir.tree.node

@dataclass(frozen=True, slots=True)
class Block(BlockImpl):
    """A Block is a block of statements."""

    # the block context
    context: BlockContext
    # the structural form of the block
    form: BlockForm
    # the leading expressions whose values are discarded
    leading_expressions: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the optional tail expression whose value becomes the block value
    tail_expression: destack._generated.dir.tree.node.LocalNodeId | None

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

"""How a block is interpreted."""
BlockContext: typing.TypeAlias = (
    typing.Literal["expression"] | typing.Literal["statement"]
)

def encode_block_context(writer: BinaryWriter, value: BlockContext) -> None: ...
def decode_block_context(reader: BinaryReader) -> BlockContext: ...
def to_json_block_context(value: BlockContext) -> Json: ...
def from_json_block_context(value: Json) -> BlockContext: ...

"""The structural form of a block."""
BlockForm: typing.TypeAlias = (
    typing.Literal["explicit"] | typing.Literal["do"] | typing.Literal["implicit"]
)

def encode_block_form(writer: BinaryWriter, value: BlockForm) -> None: ...
def decode_block_form(reader: BinaryReader) -> BlockForm: ...
def to_json_block_form(value: BlockForm) -> Json: ...
def from_json_block_form(value: Json) -> BlockForm: ...

__all__ = [
    "Block",
    "encode_block",
    "decode_block",
    "to_json_block",
    "from_json_block",
    "BlockContext",
    "encode_block_context",
    "decode_block_context",
    "to_json_block_context",
    "from_json_block_context",
    "BlockForm",
    "encode_block_form",
    "decode_block_form",
    "to_json_block_form",
    "from_json_block_form",
]
