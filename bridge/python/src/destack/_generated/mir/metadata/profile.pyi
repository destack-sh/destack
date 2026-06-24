# generated bridge target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

"""Identifier for one emitted profile counter, positional within a function."""
CounterId: typing.TypeAlias = int

def encode_counter_id(writer: BinaryWriter, value: CounterId) -> None: ...
def decode_counter_id(reader: BinaryReader) -> CounterId: ...
def to_json_counter_id(value: CounterId) -> Json: ...
def from_json_counter_id(value: Json) -> CounterId: ...

__all__ = [
    "CounterId",
    "encode_counter_id",
    "decode_counter_id",
    "to_json_counter_id",
    "from_json_counter_id",
]
