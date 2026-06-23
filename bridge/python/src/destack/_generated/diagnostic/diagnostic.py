# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.diagnostic.edit
import destack._generated.source.file
import destack._generated.source.span

if TYPE_CHECKING:
    from destack._generated.diagnostic.edit import (
        PatchSet,
    )

    from destack._generated.source.file import (
        ContentId,
    )

    from destack._generated.source.span import (
        Span,
    )

"""Diagnostic severity crossing bridge boundaries."""
DiagnosticSeverity: TypeAlias = Literal["note"] | Literal["warning"] | Literal["error"]


def encode_diagnostic_severity(writer: Writer, value: DiagnosticSeverity) -> None:
    if value == "note":
        writer.write_unsigned(0)
    elif value == "warning":
        writer.write_unsigned(1)
    elif value == "error":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_diagnostic_severity(reader: Reader) -> DiagnosticSeverity:
    variant = reader.read_number()

    if variant == 0:
        return "note"
    elif variant == 1:
        return "warning"
    elif variant == 2:
        return "error"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


"""Extra semantic diagnostic tag crossing bridge boundaries."""
DiagnosticTag: TypeAlias = Literal["unnecessary"] | Literal["deprecated"]


def encode_diagnostic_tag(writer: Writer, value: DiagnosticTag) -> None:
    if value == "unnecessary":
        writer.write_unsigned(0)
    elif value == "deprecated":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_diagnostic_tag(reader: Reader) -> DiagnosticTag:
    variant = reader.read_number()

    if variant == 0:
        return "unnecessary"
    elif variant == 1:
        return "deprecated"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


"""Whether a suggestion can be applied automatically."""
Applicability: TypeAlias = (
    Literal["automatic"] | Literal["unsafe"] | Literal["dangerous"]
)


def encode_applicability(writer: Writer, value: Applicability) -> None:
    if value == "automatic":
        writer.write_unsigned(0)
    elif value == "unsafe":
        writer.write_unsigned(1)
    elif value == "dangerous":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_applicability(reader: Reader) -> Applicability:
    variant = reader.read_number()

    if variant == 0:
        return "automatic"
    elif variant == 1:
        return "unsafe"
    elif variant == 2:
        return "dangerous"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class DiagnosticLabel:
    """One concrete source label in a diagnostic."""

    """Exact content containing the span."""
    content: ContentId
    """Concrete source span."""
    span: Span
    """Optional label shown on the span."""
    message: str | None


def encode_diagnostic_label(writer: Writer, value: DiagnosticLabel) -> None:
    destack._generated.source.file.encode_content_id(writer, value.content)
    destack._generated.source.span.encode_span(writer, value.span)
    if value.message is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.message)


def decode_diagnostic_label(reader: Reader) -> DiagnosticLabel:
    field_0 = destack._generated.source.file.decode_content_id(reader)
    field_1 = destack._generated.source.span.decode_span(reader)
    field_2 = reader.read_option(lambda: reader.read_string())

    return DiagnosticLabel(
        content=field_0,
        span=field_1,
        message=field_2,
    )


@dataclass(frozen=True, slots=True)
class DiagnosticNote:
    """Extra context for understanding a diagnostic."""

    """Note message."""
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

    """Help message."""
    message: str


def encode_diagnostic_help(writer: Writer, value: DiagnosticHelp) -> None:
    writer.write_string(value.message)


def decode_diagnostic_help(reader: Reader) -> DiagnosticHelp:
    field_0 = reader.read_string()

    return DiagnosticHelp(
        message=field_0,
    )


@dataclass(frozen=True, slots=True)
class DiagnosticSuggestion:
    """One suggested source change for a diagnostic."""

    """Exact source patches for machine application."""
    patches: PatchSet
    """Source labels to show with the suggestion."""
    labels: Sequence[DiagnosticLabel]
    """Suggestion message."""
    message: str
    """Suggestion applicability."""
    applicability: Applicability


def encode_diagnostic_suggestion(writer: Writer, value: DiagnosticSuggestion) -> None:
    destack._generated.diagnostic.edit.encode_patch_set(writer, value.patches)
    writer.write_unsigned(len(value.labels))
    for item_0 in value.labels:
        encode_diagnostic_label(writer, item_0)
    writer.write_string(value.message)
    encode_applicability(writer, value.applicability)


def decode_diagnostic_suggestion(reader: Reader) -> DiagnosticSuggestion:
    field_0 = destack._generated.diagnostic.edit.decode_patch_set(reader)
    field_1 = [decode_diagnostic_label(reader) for _ in range(reader.read_number())]
    field_2 = reader.read_string()
    field_3 = decode_applicability(reader)

    return DiagnosticSuggestion(
        patches=field_0,
        labels=field_1,
        message=field_2,
        applicability=field_3,
    )


@dataclass(frozen=True, slots=True)
class Diagnostic:
    """One final renderable diagnostic."""

    """Stable diagnostic code."""
    code: str
    """Diagnostic severity."""
    severity: DiagnosticSeverity
    """Diagnostic message."""
    message: str
    """Main source label."""
    primary: DiagnosticLabel
    """Additional source labels."""
    labels: Sequence[DiagnosticLabel]
    """Extra context."""
    notes: Sequence[DiagnosticNote]
    """Fixing or avoidance guidance."""
    helps: Sequence[DiagnosticHelp]
    """Suggested source changes."""
    suggestions: Sequence[DiagnosticSuggestion]
    """Extra semantic tags."""
    tags: Sequence[DiagnosticTag]


def encode_diagnostic(writer: Writer, value: Diagnostic) -> None:
    writer.write_string(value.code)
    encode_diagnostic_severity(writer, value.severity)
    writer.write_string(value.message)
    encode_diagnostic_label(writer, value.primary)
    writer.write_unsigned(len(value.labels))
    for item_0 in value.labels:
        encode_diagnostic_label(writer, item_0)
    writer.write_unsigned(len(value.notes))
    for item_0 in value.notes:
        encode_diagnostic_note(writer, item_0)
    writer.write_unsigned(len(value.helps))
    for item_0 in value.helps:
        encode_diagnostic_help(writer, item_0)
    writer.write_unsigned(len(value.suggestions))
    for item_0 in value.suggestions:
        encode_diagnostic_suggestion(writer, item_0)
    writer.write_unsigned(len(value.tags))
    for item_0 in value.tags:
        encode_diagnostic_tag(writer, item_0)


def decode_diagnostic(reader: Reader) -> Diagnostic:
    field_0 = reader.read_string()
    field_1 = decode_diagnostic_severity(reader)
    field_2 = reader.read_string()
    field_3 = decode_diagnostic_label(reader)
    field_4 = [decode_diagnostic_label(reader) for _ in range(reader.read_number())]
    field_5 = [decode_diagnostic_note(reader) for _ in range(reader.read_number())]
    field_6 = [decode_diagnostic_help(reader) for _ in range(reader.read_number())]
    field_7 = [
        decode_diagnostic_suggestion(reader) for _ in range(reader.read_number())
    ]
    field_8 = [decode_diagnostic_tag(reader) for _ in range(reader.read_number())]

    return Diagnostic(
        code=field_0,
        severity=field_1,
        message=field_2,
        primary=field_3,
        labels=field_4,
        notes=field_5,
        helps=field_6,
        suggestions=field_7,
        tags=field_8,
    )


__all__ = [
    "DiagnosticSeverity",
    "encode_diagnostic_severity",
    "decode_diagnostic_severity",
    "DiagnosticTag",
    "encode_diagnostic_tag",
    "decode_diagnostic_tag",
    "Applicability",
    "encode_applicability",
    "decode_applicability",
    "DiagnosticLabel",
    "encode_diagnostic_label",
    "decode_diagnostic_label",
    "DiagnosticNote",
    "encode_diagnostic_note",
    "decode_diagnostic_note",
    "DiagnosticHelp",
    "encode_diagnostic_help",
    "decode_diagnostic_help",
    "DiagnosticSuggestion",
    "encode_diagnostic_suggestion",
    "decode_diagnostic_suggestion",
    "Diagnostic",
    "encode_diagnostic",
    "decode_diagnostic",
]
