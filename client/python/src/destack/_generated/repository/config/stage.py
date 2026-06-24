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

"""Package release stage."""
Stage: typing.TypeAlias = (
    typing.Literal["experimental"]
    | typing.Literal["alpha"]
    | typing.Literal["beta"]
    | typing.Literal["stable"]
)


def encode_stage(writer: BinaryWriter, value: Stage) -> None:
    """Encode one Stage."""
    if value == "experimental":
        writer.write_unsigned(0)
    elif value == "alpha":
        writer.write_unsigned(1)
    elif value == "beta":
        writer.write_unsigned(2)
    elif value == "stable":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_stage(reader: BinaryReader) -> Stage:
    """Decode one Stage."""
    variant = reader.read_number()

    if variant == 0:
        return "experimental"
    elif variant == 1:
        return "alpha"
    elif variant == 2:
        return "beta"
    elif variant == 3:
        return "stable"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_stage(value: Stage) -> Json:
    """Return one JSON value for one Stage."""
    return value


def from_json_stage(value: Json) -> Stage:
    """Return one Stage from one JSON value."""
    variant = json_string(value)

    if variant == "experimental":
        return "experimental"
    elif variant == "alpha":
        return "alpha"
    elif variant == "beta":
        return "beta"
    elif variant == "stable":
        return "stable"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "Stage",
    "encode_stage",
    "decode_stage",
    "to_json_stage",
    "from_json_stage",
]
