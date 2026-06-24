# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string

@dataclass(frozen=True, slots=True)
class Origin:
    """How one derived tree node came to be."""

    # the transform that created the node, a dotted name like `optimize.inline`
    derivation: destack._generated.core.string.StringId
    # the same-tree nodes the node derives from, interpretation per derivation
    parents: Sequence[int]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Origin: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Origin: ...

def encode_origin(writer: BinaryWriter, value: Origin) -> None: ...
def decode_origin(reader: BinaryReader) -> Origin: ...
def to_json_origin(value: Origin) -> Json: ...
def from_json_origin(value: Json) -> Origin: ...

__all__ = [
    "Origin",
    "encode_origin",
    "decode_origin",
    "to_json_origin",
    "from_json_origin",
]
