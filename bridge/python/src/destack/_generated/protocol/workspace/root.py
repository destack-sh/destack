# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

"""Reason for reloading host source state."""
ReloadReason: TypeAlias = Literal["manual"] | Literal["overflow"] | Literal["watch"]


def encode_reload_reason(writer: Writer, value: ReloadReason) -> None:
    if value == "manual":
        writer.write_unsigned(0)
    elif value == "overflow":
        writer.write_unsigned(1)
    elif value == "watch":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_reload_reason(reader: Reader) -> ReloadReason:
    variant = reader.read_number()

    if variant == 0:
        return "manual"
    elif variant == 1:
        return "overflow"
    elif variant == 2:
        return "watch"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


__all__ = [
    "ReloadReason",
    "encode_reload_reason",
    "decode_reload_reason",
]
