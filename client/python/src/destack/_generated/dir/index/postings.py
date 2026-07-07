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
    json_string,
)


@dataclass(frozen=True, slots=True)
class Postings:
    """Compact postings from one key to module ordinals."""

    # the sorted posting keys
    keys: Sequence[str]
    # offsets into the module ordinal list
    offsets: Sequence[int]
    # module ordinals by key
    modules: Sequence[int]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_postings(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Postings:
        """Decode one Postings."""
        return decode_postings(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_postings(self)

    @classmethod
    def from_json(cls, value: Json) -> Postings:
        """Return one Postings from one JSON value."""
        return from_json_postings(value)


def encode_postings(writer: BinaryWriter, value: Postings) -> None:
    """Encode one Postings."""
    writer.write_unsigned(len(value.keys))
    for item_value_keys_0 in value.keys:
        writer.write_string(item_value_keys_0)
    writer.write_unsigned(len(value.offsets))
    for item_value_offsets_0 in value.offsets:
        writer.write_unsigned(item_value_offsets_0)
    writer.write_unsigned(len(value.modules))
    for item_value_modules_0 in value.modules:
        writer.write_unsigned(item_value_modules_0)


def decode_postings(reader: BinaryReader) -> Postings:
    """Decode one Postings."""
    keys = [reader.read_string() for _ in range(reader.read_number())]
    offsets = [reader.read_number() for _ in range(reader.read_number())]
    modules = [reader.read_number() for _ in range(reader.read_number())]

    return Postings(
        keys=keys,
        offsets=offsets,
        modules=modules,
    )


def to_json_postings(value: Postings) -> Json:
    """Return one JSON value for one Postings."""
    return {
        "keys": [item_0 for item_0 in value.keys],
        "offsets": [item_0 for item_0 in value.offsets],
        "modules": [item_0 for item_0 in value.modules],
    }


def from_json_postings(value: Json) -> Postings:
    """Return one Postings from one JSON value."""
    object_ = json_object(value)

    return Postings(
        keys=[
            json_string(item_0) for item_0 in json_array(json_field(object_, "keys"))
        ],
        offsets=[
            json_int(item_0) for item_0 in json_array(json_field(object_, "offsets"))
        ],
        modules=[
            json_int(item_0) for item_0 in json_array(json_field(object_, "modules"))
        ],
    )


__all__ = [
    "Postings",
    "encode_postings",
    "decode_postings",
    "to_json_postings",
    "from_json_postings",
]
