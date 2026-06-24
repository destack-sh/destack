# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.source.file.model.file
import destack._generated.source.file.model.span


@dataclass(frozen=True, slots=True)
class DiagnosticLabel:
    """One concrete source label in a diagnostic."""

    # the exact content containing the span
    content: destack._generated.source.file.model.file.ContentId
    # the concrete source span
    span: destack._generated.source.file.model.span.Span
    # the optional label shown on the span
    message: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_diagnostic_label(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DiagnosticLabel:
        """Decode one DiagnosticLabel."""
        return decode_diagnostic_label(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_diagnostic_label(self)

    @classmethod
    def from_json(cls, value: Json) -> DiagnosticLabel:
        """Return one DiagnosticLabel from one JSON value."""
        return from_json_diagnostic_label(value)


def encode_diagnostic_label(writer: BinaryWriter, value: DiagnosticLabel) -> None:
    """Encode one DiagnosticLabel."""
    destack._generated.source.file.model.file.encode_content_id(writer, value.content)
    destack._generated.source.file.model.span.encode_span(writer, value.span)
    if value.message is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.message)


def decode_diagnostic_label(reader: BinaryReader) -> DiagnosticLabel:
    """Decode one DiagnosticLabel."""
    content = destack._generated.source.file.model.file.decode_content_id(reader)
    span = destack._generated.source.file.model.span.decode_span(reader)
    message = reader.read_option(lambda: reader.read_string())

    return DiagnosticLabel(
        content=content,
        span=span,
        message=message,
    )


def to_json_diagnostic_label(value: DiagnosticLabel) -> Json:
    """Return one JSON value for one DiagnosticLabel."""
    return {
        "content": destack._generated.source.file.model.file.to_json_content_id(
            value.content
        ),
        "span": destack._generated.source.file.model.span.to_json_span(value.span),
        **({} if value.message is None else {"message": value.message}),
    }


def from_json_diagnostic_label(value: Json) -> DiagnosticLabel:
    """Return one DiagnosticLabel from one JSON value."""
    object_ = json_object(value)

    return DiagnosticLabel(
        content=destack._generated.source.file.model.file.from_json_content_id(
            json_field(object_, "content")
        ),
        span=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "span")
        ),
        message=json_optional(object_, "message", lambda value: json_string(value)),
    )


@dataclass(frozen=True, slots=True)
class DiagnosticNote:
    """Extra context for understanding a diagnostic."""

    # the note message
    message: str

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_diagnostic_note(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DiagnosticNote:
        """Decode one DiagnosticNote."""
        return decode_diagnostic_note(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_diagnostic_note(self)

    @classmethod
    def from_json(cls, value: Json) -> DiagnosticNote:
        """Return one DiagnosticNote from one JSON value."""
        return from_json_diagnostic_note(value)


def encode_diagnostic_note(writer: BinaryWriter, value: DiagnosticNote) -> None:
    """Encode one DiagnosticNote."""
    writer.write_string(value.message)


def decode_diagnostic_note(reader: BinaryReader) -> DiagnosticNote:
    """Decode one DiagnosticNote."""
    message = reader.read_string()

    return DiagnosticNote(
        message=message,
    )


def to_json_diagnostic_note(value: DiagnosticNote) -> Json:
    """Return one JSON value for one DiagnosticNote."""
    return {
        "message": value.message,
    }


def from_json_diagnostic_note(value: Json) -> DiagnosticNote:
    """Return one DiagnosticNote from one JSON value."""
    object_ = json_object(value)

    return DiagnosticNote(
        message=json_string(json_field(object_, "message")),
    )


@dataclass(frozen=True, slots=True)
class DiagnosticHelp:
    """Guidance for fixing or avoiding a diagnostic."""

    # the help message
    message: str

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_diagnostic_help(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DiagnosticHelp:
        """Decode one DiagnosticHelp."""
        return decode_diagnostic_help(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_diagnostic_help(self)

    @classmethod
    def from_json(cls, value: Json) -> DiagnosticHelp:
        """Return one DiagnosticHelp from one JSON value."""
        return from_json_diagnostic_help(value)


def encode_diagnostic_help(writer: BinaryWriter, value: DiagnosticHelp) -> None:
    """Encode one DiagnosticHelp."""
    writer.write_string(value.message)


def decode_diagnostic_help(reader: BinaryReader) -> DiagnosticHelp:
    """Decode one DiagnosticHelp."""
    message = reader.read_string()

    return DiagnosticHelp(
        message=message,
    )


def to_json_diagnostic_help(value: DiagnosticHelp) -> Json:
    """Return one JSON value for one DiagnosticHelp."""
    return {
        "message": value.message,
    }


def from_json_diagnostic_help(value: Json) -> DiagnosticHelp:
    """Return one DiagnosticHelp from one JSON value."""
    object_ = json_object(value)

    return DiagnosticHelp(
        message=json_string(json_field(object_, "message")),
    )


__all__ = [
    "DiagnosticLabel",
    "encode_diagnostic_label",
    "decode_diagnostic_label",
    "to_json_diagnostic_label",
    "from_json_diagnostic_label",
    "DiagnosticNote",
    "encode_diagnostic_note",
    "decode_diagnostic_note",
    "to_json_diagnostic_note",
    "from_json_diagnostic_note",
    "DiagnosticHelp",
    "encode_diagnostic_help",
    "decode_diagnostic_help",
    "to_json_diagnostic_help",
    "from_json_diagnostic_help",
]
