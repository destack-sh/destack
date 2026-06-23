# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.source.file.format.formatting

if TYPE_CHECKING:
    from destack._generated.protocol.source.file.format.formatting import (
        IndentStyle,
        LineEnding,
    )


@dataclass(frozen=True, slots=True)
class FormatterOptions:
    """Formatter options."""

    """Line ending style (LF, CRLF, CR)."""
    line_ending: LineEnding
    """Indent with spaces or tabs."""
    indent_style: IndentStyle
    """Number of spaces per indent level (when using spaces)."""
    indent_width: int
    """Target line width (best effort, not a hard limit)."""
    line_width: int


def encode_formatter_options(writer: Writer, value: FormatterOptions) -> None:
    destack._generated.protocol.source.file.format.formatting.encode_line_ending(
        writer, value.line_ending
    )
    destack._generated.protocol.source.file.format.formatting.encode_indent_style(
        writer, value.indent_style
    )
    writer.write_byte(value.indent_width)
    writer.write_unsigned(value.line_width)


def decode_formatter_options(reader: Reader) -> FormatterOptions:
    field_0 = (
        destack._generated.protocol.source.file.format.formatting.decode_line_ending(
            reader
        )
    )
    field_1 = (
        destack._generated.protocol.source.file.format.formatting.decode_indent_style(
            reader
        )
    )
    field_2 = reader.read_byte()
    field_3 = reader.read_number()

    return FormatterOptions(
        line_ending=field_0,
        indent_style=field_1,
        indent_width=field_2,
        line_width=field_3,
    )


__all__ = [
    "FormatterOptions",
    "encode_formatter_options",
    "decode_formatter_options",
]
