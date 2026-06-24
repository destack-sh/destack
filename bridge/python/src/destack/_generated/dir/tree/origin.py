# generated bridge target, do not edit

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

import destack._generated.core.string


@dataclass(frozen=True, slots=True)
class Origin:
    """How one derived tree node came to be."""

    # the transform that created the node, a dotted name like `optimize.inline`
    derivation: destack._generated.core.string.StringId
    # the same-tree nodes the node derives from, interpretation per derivation
    parents: Sequence[int]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_origin(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Origin:
        """Decode one Origin."""
        return decode_origin(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_origin(self)

    @classmethod
    def from_json(cls, value: Json) -> Origin:
        """Return one Origin from one JSON value."""
        return from_json_origin(value)


def encode_origin(writer: BinaryWriter, value: Origin) -> None:
    """Encode one Origin."""
    destack._generated.core.string.encode_string_id(writer, value.derivation)
    writer.write_unsigned(len(value.parents))
    for item_value_parents_0 in value.parents:
        writer.write_unsigned(item_value_parents_0)


def decode_origin(reader: BinaryReader) -> Origin:
    """Decode one Origin."""
    derivation = destack._generated.core.string.decode_string_id(reader)
    parents = [reader.read_number() for _ in range(reader.read_number())]

    return Origin(
        derivation=derivation,
        parents=parents,
    )


def to_json_origin(value: Origin) -> Json:
    """Return one JSON value for one Origin."""
    return {
        "derivation": destack._generated.core.string.to_json_string_id(
            value.derivation
        ),
        "parents": [item_0 for item_0 in value.parents],
    }


def from_json_origin(value: Json) -> Origin:
    """Return one Origin from one JSON value."""
    object_ = json_object(value)

    return Origin(
        derivation=destack._generated.core.string.from_json_string_id(
            json_field(object_, "derivation")
        ),
        parents=[
            json_int(item_0) for item_0 in json_array(json_field(object_, "parents"))
        ],
    )


__all__ = [
    "Origin",
    "encode_origin",
    "decode_origin",
    "to_json_origin",
    "from_json_origin",
]
