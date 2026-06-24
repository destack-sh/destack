# generated client target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json, json_int

"""Stable identifier for one source component."""
ComponentId: typing.TypeAlias = int


def encode_component_id(writer: BinaryWriter, value: ComponentId) -> None:
    """Encode one ComponentId."""
    writer.write_unsigned(value)


def decode_component_id(reader: BinaryReader) -> ComponentId:
    """Decode one ComponentId."""
    return reader.read_unsigned()


def to_json_component_id(value: ComponentId) -> Json:
    """Return one JSON value for one ComponentId."""
    return value


def from_json_component_id(value: Json) -> ComponentId:
    """Return one ComponentId from one JSON value."""
    return json_int(value)


__all__ = [
    "ComponentId",
    "encode_component_id",
    "decode_component_id",
    "to_json_component_id",
    "from_json_component_id",
]
