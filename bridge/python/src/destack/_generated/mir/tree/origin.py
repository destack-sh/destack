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
    json_optional,
)

import destack._generated.core.string


@dataclass(frozen=True, slots=True)
class OriginTable:
    """Origin records for derived nodes: dense per-node slots into a packed arena."""

    # the arena slot for each node index, present for derived nodes
    slot_by_index: Sequence[int | None]
    # the packed origin records, emptied when an origin moves to another node
    origins: Sequence[Origin | None]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_origin_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> OriginTable:
        """Decode one OriginTable."""
        return decode_origin_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_origin_table(self)

    @classmethod
    def from_json(cls, value: Json) -> OriginTable:
        """Return one OriginTable from one JSON value."""
        return from_json_origin_table(value)


def encode_origin_table(writer: BinaryWriter, value: OriginTable) -> None:
    """Encode one OriginTable."""
    writer.write_unsigned(len(value.slot_by_index))
    for item_value_slot_by_index_0 in value.slot_by_index:
        if item_value_slot_by_index_0 is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            writer.write_unsigned(item_value_slot_by_index_0)
    writer.write_unsigned(len(value.origins))
    for item_value_origins_0 in value.origins:
        if item_value_origins_0 is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_origin(writer, item_value_origins_0)


def decode_origin_table(reader: BinaryReader) -> OriginTable:
    """Decode one OriginTable."""
    slot_by_index = [
        reader.read_option(lambda: reader.read_number())
        for _ in range(reader.read_number())
    ]
    origins = [
        reader.read_option(lambda: decode_origin(reader))
        for _ in range(reader.read_number())
    ]

    return OriginTable(
        slot_by_index=slot_by_index,
        origins=origins,
    )


def to_json_origin_table(value: OriginTable) -> Json:
    """Return one JSON value for one OriginTable."""
    return {
        "slotByIndex": [
            None if item_0 is None else item_0 for item_0 in value.slot_by_index
        ],
        "origins": [
            None if item_0 is None else to_json_origin(item_0)
            for item_0 in value.origins
        ],
    }


def from_json_origin_table(value: Json) -> OriginTable:
    """Return one OriginTable from one JSON value."""
    object_ = json_object(value)

    return OriginTable(
        slot_by_index=[
            None if item_0 is None else json_int(item_0)
            for item_0 in json_array(json_field(object_, "slotByIndex"))
        ],
        origins=[
            None if item_0 is None else from_json_origin(item_0)
            for item_0 in json_array(json_field(object_, "origins"))
        ],
    )


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
    "OriginTable",
    "encode_origin_table",
    "decode_origin_table",
    "to_json_origin_table",
    "from_json_origin_table",
    "Origin",
    "encode_origin",
    "decode_origin",
    "to_json_origin",
    "from_json_origin",
]
