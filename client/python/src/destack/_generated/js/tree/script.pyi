# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string
import destack._generated.js.tree.node
import destack._generated.js.tree.tree

@dataclass(frozen=True, slots=True)
class Module:
    """One lowered JavaScript or TypeScript module tree."""

    # the lowered script tree
    tree: destack._generated.js.tree.tree.Tree
    # the root nodes in the lowered tree
    roots: Sequence[destack._generated.js.tree.node.LocalNodeIdAny]
    # the string pool for the lowered tree
    strings: destack._generated.core.string.StringPoolData

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Module: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Module: ...

def encode_module(writer: BinaryWriter, value: Module) -> None: ...
def decode_module(reader: BinaryReader) -> Module: ...
def to_json_module(value: Module) -> Json: ...
def from_json_module(value: Json) -> Module: ...

__all__ = [
    "Module",
    "encode_module",
    "decode_module",
    "to_json_module",
    "from_json_module",
]
