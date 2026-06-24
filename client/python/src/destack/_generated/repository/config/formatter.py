# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_int,
    json_object,
)

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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_formatter_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FormatterOptions:
        """Decode one FormatterOptions."""
        return decode_formatter_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_formatter_options(self)

    @classmethod
    def from_json(cls, value: Json) -> FormatterOptions:
        """Return one FormatterOptions from one JSON value."""
        return from_json_formatter_options(value)


def encode_formatter_options(writer: BinaryWriter, value: FormatterOptions) -> None:
    """Encode one FormatterOptions."""
    destack._generated.source.file.format.formatting.encode_line_ending(
        writer, value.line_ending
    )
    destack._generated.source.file.format.formatting.encode_indent_style(
        writer, value.indent_style
    )
    writer.write_byte(value.indent_width)
    writer.write_unsigned(value.line_width)


def decode_formatter_options(reader: BinaryReader) -> FormatterOptions:
    """Decode one FormatterOptions."""
    line_ending = destack._generated.source.file.format.formatting.decode_line_ending(
        reader
    )
    indent_style = destack._generated.source.file.format.formatting.decode_indent_style(
        reader
    )
    indent_width = reader.read_byte()
    line_width = reader.read_number()

    return FormatterOptions(
        line_ending=line_ending,
        indent_style=indent_style,
        indent_width=indent_width,
        line_width=line_width,
    )


def to_json_formatter_options(value: FormatterOptions) -> Json:
    """Return one JSON value for one FormatterOptions."""
    return {
        "lineEnding": destack._generated.source.file.format.formatting.to_json_line_ending(
            value.line_ending
        ),
        "indentStyle": destack._generated.source.file.format.formatting.to_json_indent_style(
            value.indent_style
        ),
        "indentWidth": value.indent_width,
        "lineWidth": value.line_width,
    }


def from_json_formatter_options(value: Json) -> FormatterOptions:
    """Return one FormatterOptions from one JSON value."""
    object_ = json_object(value)

    return FormatterOptions(
        line_ending=destack._generated.source.file.format.formatting.from_json_line_ending(
            json_field(object_, "lineEnding")
        ),
        indent_style=destack._generated.source.file.format.formatting.from_json_indent_style(
            json_field(object_, "indentStyle")
        ),
        indent_width=json_int(json_field(object_, "indentWidth")),
        line_width=json_int(json_field(object_, "lineWidth")),
    )


__all__ = [
    "FormatterOptions",
    "encode_formatter_options",
    "decode_formatter_options",
    "to_json_formatter_options",
    "from_json_formatter_options",
]
