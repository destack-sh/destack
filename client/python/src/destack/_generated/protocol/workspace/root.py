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

"""Reason for reloading host source state."""
ReloadReason: typing.TypeAlias = (
    typing.Literal["manual"] | typing.Literal["overflow"] | typing.Literal["watch"]
)


def encode_reload_reason(writer: BinaryWriter, value: ReloadReason) -> None:
    """Encode one ReloadReason."""
    if value == "manual":
        writer.write_unsigned(0)
    elif value == "overflow":
        writer.write_unsigned(1)
    elif value == "watch":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_reload_reason(reader: BinaryReader) -> ReloadReason:
    """Decode one ReloadReason."""
    variant = reader.read_number()

    if variant == 0:
        return "manual"
    elif variant == 1:
        return "overflow"
    elif variant == 2:
        return "watch"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_reload_reason(value: ReloadReason) -> Json:
    """Return one JSON value for one ReloadReason."""
    return value


def from_json_reload_reason(value: Json) -> ReloadReason:
    """Return one ReloadReason from one JSON value."""
    variant = json_string(value)

    if variant == "manual":
        return "manual"
    elif variant == "overflow":
        return "overflow"
    elif variant == "watch":
        return "watch"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "ReloadReason",
    "encode_reload_reason",
    "decode_reload_reason",
    "to_json_reload_reason",
    "from_json_reload_reason",
]
