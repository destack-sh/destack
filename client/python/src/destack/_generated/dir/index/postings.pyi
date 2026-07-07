# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class Postings:
    """Compact postings from one key to module ordinals."""

    # the sorted posting keys
    keys: Sequence[str]
    # offsets into the module ordinal list
    offsets: Sequence[int]
    # module ordinals by key
    modules: Sequence[int]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Postings: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Postings: ...

def encode_postings(writer: BinaryWriter, value: Postings) -> None: ...
def decode_postings(reader: BinaryReader) -> Postings: ...
def to_json_postings(value: Postings) -> Json: ...
def from_json_postings(value: Json) -> Postings: ...

__all__ = [
    "Postings",
    "encode_postings",
    "decode_postings",
    "to_json_postings",
    "from_json_postings",
]
