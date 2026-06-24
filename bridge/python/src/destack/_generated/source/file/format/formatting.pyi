# generated bridge target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

"""The type of line ending to apply to the printed input."""
LineEnding: typing.TypeAlias = (
    typing.Literal["lineFeed"]
    | typing.Literal["carriageReturnLineFeed"]
    | typing.Literal["carriageReturn"]
)

def encode_line_ending(writer: BinaryWriter, value: LineEnding) -> None: ...
def decode_line_ending(reader: BinaryReader) -> LineEnding: ...
def to_json_line_ending(value: LineEnding) -> Json: ...
def from_json_line_ending(value: Json) -> LineEnding: ...

"""The indent style."""
IndentStyle: typing.TypeAlias = typing.Literal["tab"] | typing.Literal["space"]

def encode_indent_style(writer: BinaryWriter, value: IndentStyle) -> None: ...
def decode_indent_style(reader: BinaryReader) -> IndentStyle: ...
def to_json_indent_style(value: IndentStyle) -> Json: ...
def from_json_indent_style(value: Json) -> IndentStyle: ...

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
