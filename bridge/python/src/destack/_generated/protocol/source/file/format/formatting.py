# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

"""The type of line ending to apply to the printed input."""
LineEnding: TypeAlias = (
    Literal["lineFeed"] | Literal["carriageReturnLineFeed"] | Literal["carriageReturn"]
)


def encode_line_ending(writer: Writer, value: LineEnding) -> None:
    if value == "lineFeed":
        writer.write_unsigned(0)
    elif value == "carriageReturnLineFeed":
        writer.write_unsigned(1)
    elif value == "carriageReturn":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_line_ending(reader: Reader) -> LineEnding:
    variant = reader.read_number()

    if variant == 0:
        return "lineFeed"
    elif variant == 1:
        return "carriageReturnLineFeed"
    elif variant == 2:
        return "carriageReturn"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


"""The indent style."""
IndentStyle: TypeAlias = Literal["tab"] | Literal["space"]


def encode_indent_style(writer: Writer, value: IndentStyle) -> None:
    if value == "tab":
        writer.write_unsigned(0)
    elif value == "space":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_indent_style(reader: Reader) -> IndentStyle:
    variant = reader.read_number()

    if variant == 0:
        return "tab"
    elif variant == 1:
        return "space"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


__all__ = [
    "LineEnding",
    "encode_line_ending",
    "decode_line_ending",
    "IndentStyle",
    "encode_indent_style",
    "decode_indent_style",
]
