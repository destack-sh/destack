# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.source.file.format.formatting

@dataclass(frozen=True, slots=True)
class FormatterOptions:
    """Formatter options."""

    # line ending style (LF, CRLF, CR)
    line_ending: destack._generated.source.file.format.formatting.LineEnding
    # indent with spaces or tabs
    indent_style: destack._generated.source.file.format.formatting.IndentStyle
    # number of spaces per indent level (when using spaces)
    indent_width: int
    # target line width (best effort, not a hard limit)
    line_width: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FormatterOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FormatterOptions: ...

def encode_formatter_options(writer: BinaryWriter, value: FormatterOptions) -> None: ...
def decode_formatter_options(reader: BinaryReader) -> FormatterOptions: ...
def to_json_formatter_options(value: FormatterOptions) -> Json: ...
def from_json_formatter_options(value: Json) -> FormatterOptions: ...

__all__ = [
    "FormatterOptions",
    "encode_formatter_options",
    "decode_formatter_options",
    "to_json_formatter_options",
    "from_json_formatter_options",
]
