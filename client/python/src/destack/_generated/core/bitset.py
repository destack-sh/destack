# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_field,
    json_int,
    json_object,
)

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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_bit_set(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> BitSet:
        """Decode one BitSet."""
        return decode_bit_set(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_bit_set(self)

    @classmethod
    def from_json(cls, value: Json) -> BitSet:
        """Return one BitSet from one JSON value."""
        return from_json_bit_set(value)


def encode_bit_set(writer: BinaryWriter, value: BitSet) -> None:
    """Encode one BitSet."""
    writer.write_unsigned(len(value.words))
    for item_value_words_0 in value.words:
        writer.write_unsigned(item_value_words_0)
    writer.write_unsigned(value.length)


def decode_bit_set(reader: BinaryReader) -> BitSet:
    """Decode one BitSet."""
    words = [reader.read_number() for _ in range(reader.read_number())]
    length = reader.read_number()

    return BitSet(
        words=words,
        length=length,
    )


def to_json_bit_set(value: BitSet) -> Json:
    """Return one JSON value for one BitSet."""
    return {
        "words": [item_0 for item_0 in value.words],
        "length": value.length,
    }


def from_json_bit_set(value: Json) -> BitSet:
    """Return one BitSet from one JSON value."""
    object_ = json_object(value)

    return BitSet(
        words=[json_int(item_0) for item_0 in json_array(json_field(object_, "words"))],
        length=json_int(json_field(object_, "length")),
    )


__all__ = [
    "BitSet",
    "encode_bit_set",
    "decode_bit_set",
    "to_json_bit_set",
    "from_json_bit_set",
]
