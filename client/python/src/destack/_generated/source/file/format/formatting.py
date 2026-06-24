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

"""The type of line ending to apply to the printed input."""
LineEnding: typing.TypeAlias = (
    typing.Literal["lineFeed"]
    | typing.Literal["carriageReturnLineFeed"]
    | typing.Literal["carriageReturn"]
)


def encode_line_ending(writer: BinaryWriter, value: LineEnding) -> None:
    """Encode one LineEnding."""
    if value == "lineFeed":
        writer.write_unsigned(0)
    elif value == "carriageReturnLineFeed":
        writer.write_unsigned(1)
    elif value == "carriageReturn":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_line_ending(reader: BinaryReader) -> LineEnding:
    """Decode one LineEnding."""
    variant = reader.read_number()

    if variant == 0:
        return "lineFeed"
    elif variant == 1:
        return "carriageReturnLineFeed"
    elif variant == 2:
        return "carriageReturn"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_line_ending(value: LineEnding) -> Json:
    """Return one JSON value for one LineEnding."""
    return value


def from_json_line_ending(value: Json) -> LineEnding:
    """Return one LineEnding from one JSON value."""
    variant = json_string(value)

    if variant == "lineFeed":
        return "lineFeed"
    elif variant == "carriageReturnLineFeed":
        return "carriageReturnLineFeed"
    elif variant == "carriageReturn":
        return "carriageReturn"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""The indent style."""
IndentStyle: typing.TypeAlias = typing.Literal["tab"] | typing.Literal["space"]


def encode_indent_style(writer: BinaryWriter, value: IndentStyle) -> None:
    """Encode one IndentStyle."""
    if value == "tab":
        writer.write_unsigned(0)
    elif value == "space":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_indent_style(reader: BinaryReader) -> IndentStyle:
    """Decode one IndentStyle."""
    variant = reader.read_number()

    if variant == 0:
        return "tab"
    elif variant == 1:
        return "space"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_indent_style(value: IndentStyle) -> Json:
    """Return one JSON value for one IndentStyle."""
    return value


def from_json_indent_style(value: Json) -> IndentStyle:
    """Return one IndentStyle from one JSON value."""
    variant = json_string(value)

    if variant == "tab":
        return "tab"
    elif variant == "space":
        return "space"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "LineEnding",
    "encode_line_ending",
    "decode_line_ending",
    "to_json_line_ending",
    "from_json_line_ending",
    "IndentStyle",
    "encode_indent_style",
    "decode_indent_style",
    "to_json_indent_style",
    "from_json_indent_style",
]
