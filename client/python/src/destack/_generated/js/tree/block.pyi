# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.js.tree.node

@dataclass(frozen=True, slots=True)
class Block:
    """Block of statements."""

    # the statements in the block
    statements: Sequence[destack._generated.js.tree.node.LocalNodeId]

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

@dataclass(frozen=True, slots=True)
class CatchClause:
    """A catch clause."""

    # the optional catch pattern
    pattern: destack._generated.js.tree.node.LocalNodeId | None
    # the catch body
    body: destack._generated.js.tree.node.LocalNodeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CatchClause: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CatchClause: ...

def encode_catch_clause(writer: BinaryWriter, value: CatchClause) -> None: ...
def decode_catch_clause(reader: BinaryReader) -> CatchClause: ...
def to_json_catch_clause(value: CatchClause) -> Json: ...
def from_json_catch_clause(value: Json) -> CatchClause: ...

@dataclass(frozen=True, slots=True)
class SwitchCase:
    """A switch case."""

    # the optional case selector
    value: destack._generated.js.tree.node.LocalNodeId | None
    # the body of the case
    body: destack._generated.js.tree.node.LocalNodeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SwitchCase: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SwitchCase: ...

def encode_switch_case(writer: BinaryWriter, value: SwitchCase) -> None: ...
def decode_switch_case(reader: BinaryReader) -> SwitchCase: ...
def to_json_switch_case(value: SwitchCase) -> Json: ...
def from_json_switch_case(value: Json) -> SwitchCase: ...

__all__ = [
    "Block",
    "encode_block",
    "decode_block",
    "to_json_block",
    "from_json_block",
    "CatchClause",
    "encode_catch_clause",
    "decode_catch_clause",
    "to_json_catch_clause",
    "from_json_catch_clause",
    "SwitchCase",
    "encode_switch_case",
    "decode_switch_case",
    "to_json_switch_case",
    "from_json_switch_case",
]
