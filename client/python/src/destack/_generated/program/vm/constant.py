# generated client target, do not edit

from __future__ import annotations

import builtins

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    bytes_from_json,
    bytes_to_json,
    json_field,
    json_object,
    json_string,
)


@dataclass(frozen=True, slots=True)
class ConstValueAggregate:
    """Constant payload for an aggregate value."""

    aggregate: builtins.bytes | bytearray | Sequence[int]
    kind: typing.Literal["aggregate"] = "aggregate"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_const_value(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_const_value(self)


"""Constant value stored in lowered instructions."""
ConstValue: typing.TypeAlias = ConstValueAggregate


def encode_const_value(writer: BinaryWriter, value: ConstValue) -> None:
    """Encode one ConstValue."""
    if value.kind == "aggregate":
        writer.write_unsigned(0)
        writer.write_byte_slice(value.aggregate)
    else:
        raise SerdeError("unknown enum variant")


def decode_const_value(reader: BinaryReader) -> ConstValue:
    """Decode one ConstValue."""
    variant = reader.read_number()

    if variant == 0:
        aggregate = reader.read_byte_slice()

        return ConstValueAggregate(aggregate=aggregate)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_const_value(value: ConstValue) -> Json:
    """Return one JSON value for one ConstValue."""
    if value.kind == "aggregate":
        return {
            "kind": "aggregate",
            "aggregate": bytes_to_json(value.aggregate),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_const_value(value: Json) -> ConstValue:
    """Return one ConstValue from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "aggregate":
        return ConstValueAggregate(
            aggregate=bytes_from_json(json_field(object_, "aggregate"))
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "ConstValue",
    "encode_const_value",
    "decode_const_value",
    "to_json_const_value",
    "from_json_const_value",
    "ConstValueAggregate",
]
