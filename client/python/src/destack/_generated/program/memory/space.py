# generated client target, do not edit

from __future__ import annotations

import builtins

from collections.abc import Mapping, Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    bytes_from_json,
    bytes_to_json,
    json_array,
    json_field,
    json_int,
    json_object,
    nested_bytes,
)

import destack._generated.program.memory.region


@dataclass(frozen=True, slots=True)
class StaticSpace:
    """Static memory."""

    # the static bytes
    bytes: builtins.bytes | bytearray | Sequence[int]
    # static regions
    regions: Sequence[destack._generated.program.memory.region.StaticRegion]
    # region index by id
    region_by_id: Mapping[destack._generated.program.memory.region.StaticId, int]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_static_space(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> StaticSpace:
        """Decode one StaticSpace."""
        return decode_static_space(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_static_space(self)

    @classmethod
    def from_json(cls, value: Json) -> StaticSpace:
        """Return one StaticSpace from one JSON value."""
        return from_json_static_space(value)


def encode_static_space(writer: BinaryWriter, value: StaticSpace) -> None:
    """Encode one StaticSpace."""
    writer.write_byte_slice(value.bytes)
    writer.write_unsigned(len(value.regions))
    for item_value_regions_0 in value.regions:
        destack._generated.program.memory.region.encode_static_region(
            writer, item_value_regions_0
        )
    entries_value_region_by_id_0 = []
    for (
        key_value_region_by_id_0,
        item_value_region_by_id_0,
    ) in value.region_by_id.items():

        def write_key_value_region_by_id_0(writer: BinaryWriter) -> None:
            destack._generated.program.memory.region.encode_static_id(
                writer, key_value_region_by_id_0
            )

        key_bytes = nested_bytes(write_key_value_region_by_id_0)
        entries_value_region_by_id_0.append(
            (key_value_region_by_id_0, item_value_region_by_id_0, key_bytes)
        )
    entries_value_region_by_id_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_region_by_id_0))
    for entry_value_region_by_id_0 in entries_value_region_by_id_0:
        destack._generated.program.memory.region.encode_static_id(
            writer, entry_value_region_by_id_0[0]
        )
        writer.write_unsigned(entry_value_region_by_id_0[1])


def decode_static_space(reader: BinaryReader) -> StaticSpace:
    """Decode one StaticSpace."""
    bytes = reader.read_byte_slice()
    regions = [
        destack._generated.program.memory.region.decode_static_region(reader)
        for _ in range(reader.read_number())
    ]
    region_by_id = {
        destack._generated.program.memory.region.decode_static_id(
            reader
        ): reader.read_number()
        for _ in range(reader.read_number())
    }

    return StaticSpace(
        bytes=bytes,
        regions=regions,
        region_by_id=region_by_id,
    )


def to_json_static_space(value: StaticSpace) -> Json:
    """Return one JSON value for one StaticSpace."""
    return {
        "bytes": bytes_to_json(value.bytes),
        "regions": [
            destack._generated.program.memory.region.to_json_static_region(item_0)
            for item_0 in value.regions
        ],
        "regionById": [
            [destack._generated.program.memory.region.to_json_static_id(key_0), item_0]
            for key_0, item_0 in value.region_by_id.items()
        ],
    }


def from_json_static_space(value: Json) -> StaticSpace:
    """Return one StaticSpace from one JSON value."""
    object_ = json_object(value)

    return StaticSpace(
        bytes=bytes_from_json(json_field(object_, "bytes")),
        regions=[
            destack._generated.program.memory.region.from_json_static_region(item_0)
            for item_0 in json_array(json_field(object_, "regions"))
        ],
        region_by_id={
            destack._generated.program.memory.region.from_json_static_id(
                key_0
            ): json_int(item_0)
            for key_0, item_0 in json_array(json_field(object_, "regionById"))
        },
    )


__all__ = [
    "StaticSpace",
    "encode_static_space",
    "decode_static_space",
    "to_json_static_space",
    "from_json_static_space",
]
