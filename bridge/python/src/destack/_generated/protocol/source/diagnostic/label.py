# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.source.file.model.file
import destack._generated.protocol.source.file.model.span

if TYPE_CHECKING:
    from destack._generated.protocol.source.file.model.file import (
        ContentId,
    )

    from destack._generated.protocol.source.file.model.span import (
        Span,
    )


@dataclass(frozen=True, slots=True)
class DiagnosticLabel:
    """One concrete source label in a diagnostic."""

    """The exact content containing the span."""
    content: ContentId
    """The concrete source span."""
    span: Span
    """The optional label shown on the span."""
    message: str | None


def encode_diagnostic_label(writer: Writer, value: DiagnosticLabel) -> None:
    destack._generated.protocol.source.file.model.file.encode_content_id(
        writer, value.content
    )
    destack._generated.protocol.source.file.model.span.encode_span(writer, value.span)
    if value.message is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.message)


def decode_diagnostic_label(reader: Reader) -> DiagnosticLabel:
    field_0 = destack._generated.protocol.source.file.model.file.decode_content_id(
        reader
    )
    field_1 = destack._generated.protocol.source.file.model.span.decode_span(reader)
    field_2 = reader.read_option(lambda: reader.read_string())

    return DiagnosticLabel(
        content=field_0,
        span=field_1,
        message=field_2,
    )


@dataclass(frozen=True, slots=True)
class DiagnosticNote:
    """Extra context for understanding a diagnostic."""

    """The note message."""
    message: str


def encode_diagnostic_note(writer: Writer, value: DiagnosticNote) -> None:
    writer.write_string(value.message)


def decode_diagnostic_note(reader: Reader) -> DiagnosticNote:
    field_0 = reader.read_string()

    return DiagnosticNote(
        message=field_0,
    )


@dataclass(frozen=True, slots=True)
class DiagnosticHelp:
    """Guidance for fixing or avoiding a diagnostic."""

    """The help message."""
    message: str


def encode_diagnostic_help(writer: Writer, value: DiagnosticHelp) -> None:
    writer.write_string(value.message)


def decode_diagnostic_help(reader: Reader) -> DiagnosticHelp:
    field_0 = reader.read_string()

    return DiagnosticHelp(
        message=field_0,
    )


__all__ = [
    "DiagnosticLabel",
    "encode_diagnostic_label",
    "decode_diagnostic_label",
    "DiagnosticNote",
    "encode_diagnostic_note",
    "decode_diagnostic_note",
    "DiagnosticHelp",
    "encode_diagnostic_help",
    "decode_diagnostic_help",
]
