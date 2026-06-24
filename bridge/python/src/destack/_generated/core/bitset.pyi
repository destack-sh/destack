# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

from destack._impl.core.bitset import (
    BitSetImpl,
)

@dataclass(frozen=True, slots=True)
class BitSet(BitSetImpl):
    """A fixed-length set of bits packed into 64-bit words."""

    # backing words, each holding 64 bits from low to high
    words: Sequence[int]
    # the number of valid bits; positions at or beyond this stay clear
    length: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> BitSet: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> BitSet: ...

def encode_bit_set(writer: BinaryWriter, value: BitSet) -> None: ...
def decode_bit_set(reader: BinaryReader) -> BitSet: ...
def to_json_bit_set(value: BitSet) -> Json: ...
def from_json_bit_set(value: Json) -> BitSet: ...

__all__ = [
    "BitSet",
    "encode_bit_set",
    "decode_bit_set",
    "to_json_bit_set",
    "from_json_bit_set",
]
