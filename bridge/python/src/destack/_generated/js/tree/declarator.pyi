# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.js.tree.node

@dataclass(frozen=True, slots=True)
class Declarator:
    """A Declarator is an individual variable declaration within a let/const/var statement."""

    # the pattern to bind (can be a simple identifier or destructuring pattern)
    pattern: destack._generated.js.tree.node.LocalNodeId
    # optional type annotation
    ty: destack._generated.js.tree.node.LocalNodeId | None
    # optional value expression
    value: destack._generated.js.tree.node.LocalNodeId | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Declarator: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Declarator: ...

def encode_declarator(writer: BinaryWriter, value: Declarator) -> None: ...
def decode_declarator(reader: BinaryReader) -> Declarator: ...
def to_json_declarator(value: Declarator) -> Json: ...
def from_json_declarator(value: Json) -> Declarator: ...

__all__ = [
    "Declarator",
    "encode_declarator",
    "decode_declarator",
    "to_json_declarator",
    "from_json_declarator",
]
