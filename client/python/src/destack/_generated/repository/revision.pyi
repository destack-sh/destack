# generated client target, do not edit

from __future__ import annotations

import builtins

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class Revision:
    """Content identity for one repository source and environment state."""

    value: builtins.bytes | bytearray | Sequence[int]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Revision: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Revision: ...

def encode_revision(writer: BinaryWriter, value: Revision) -> None: ...
def decode_revision(reader: BinaryReader) -> Revision: ...
def to_json_revision(value: Revision) -> Json: ...
def from_json_revision(value: Json) -> Revision: ...

__all__ = [
    "Revision",
    "encode_revision",
    "decode_revision",
    "to_json_revision",
    "from_json_revision",
]
