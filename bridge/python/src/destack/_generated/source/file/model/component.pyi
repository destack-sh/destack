# generated bridge target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

"""Stable identifier for one source component."""
ComponentId: typing.TypeAlias = int

def encode_component_id(writer: BinaryWriter, value: ComponentId) -> None: ...
def decode_component_id(reader: BinaryReader) -> ComponentId: ...
def to_json_component_id(value: ComponentId) -> Json: ...
def from_json_component_id(value: Json) -> ComponentId: ...

__all__ = [
    "ComponentId",
    "encode_component_id",
    "decode_component_id",
    "to_json_component_id",
    "from_json_component_id",
]
