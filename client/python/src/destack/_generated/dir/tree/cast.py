# generated client target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_string,
)

"""How one cast expression entered the DIR."""
CastOrigin: typing.TypeAlias = typing.Literal["explicit"] | typing.Literal["implicit"]


def encode_cast_origin(writer: BinaryWriter, value: CastOrigin) -> None:
    """Encode one CastOrigin."""
    if value == "explicit":
        writer.write_unsigned(0)
    elif value == "implicit":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_cast_origin(reader: BinaryReader) -> CastOrigin:
    """Decode one CastOrigin."""
    variant = reader.read_number()

    if variant == 0:
        return "explicit"
    elif variant == 1:
        return "implicit"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_cast_origin(value: CastOrigin) -> Json:
    """Return one JSON value for one CastOrigin."""
    return value


def from_json_cast_origin(value: Json) -> CastOrigin:
    """Return one CastOrigin from one JSON value."""
    variant = json_string(value)

    if variant == "explicit":
        return "explicit"
    elif variant == "implicit":
        return "implicit"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "CastOrigin",
    "encode_cast_origin",
    "decode_cast_origin",
    "to_json_cast_origin",
    "from_json_cast_origin",
]
