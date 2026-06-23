# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.source.diagnostic.label
import destack._generated.protocol.source.diagnostic.severity
import destack._generated.protocol.source.diagnostic.suggestion

if TYPE_CHECKING:
    from destack._generated.protocol.source.diagnostic.label import (
        DiagnosticHelp,
        DiagnosticLabel,
        DiagnosticNote,
    )

    from destack._generated.protocol.source.diagnostic.severity import (
        DiagnosticSeverity,
        DiagnosticTag,
    )

    from destack._generated.protocol.source.diagnostic.suggestion import (
        DiagnosticSuggestion,
    )


@dataclass(frozen=True, slots=True)
class Diagnostic:
    """A final renderable diagnostic."""

    """The stable identifier of the diagnostic (like `E001` or `W017`)."""
    code: str
    """The DiagnosticSeverity of the diagnostic."""
    severity: DiagnosticSeverity
    """The message of the diagnostic."""
    message: str
    """The main source label."""
    primary: DiagnosticLabel
    """Additional source labels."""
    labels: Sequence[DiagnosticLabel]
    """Extra context for understanding the diagnostic."""
    notes: Sequence[DiagnosticNote]
    """Guidance for fixing or avoiding the diagnostic."""
    helps: Sequence[DiagnosticHelp]
    """The suggestions for the diagnostic."""
    suggestions: Sequence[DiagnosticSuggestion]
    """Extra semantic tags."""
    tags: Sequence[DiagnosticTag]


def encode_diagnostic(writer: Writer, value: Diagnostic) -> None:
    writer.write_string(value.code)
    destack._generated.protocol.source.diagnostic.severity.encode_diagnostic_severity(
        writer, value.severity
    )
    writer.write_string(value.message)
    destack._generated.protocol.source.diagnostic.label.encode_diagnostic_label(
        writer, value.primary
    )
    writer.write_unsigned(len(value.labels))
    for item_0 in value.labels:
        destack._generated.protocol.source.diagnostic.label.encode_diagnostic_label(
            writer, item_0
        )
    writer.write_unsigned(len(value.notes))
    for item_0 in value.notes:
        destack._generated.protocol.source.diagnostic.label.encode_diagnostic_note(
            writer, item_0
        )
    writer.write_unsigned(len(value.helps))
    for item_0 in value.helps:
        destack._generated.protocol.source.diagnostic.label.encode_diagnostic_help(
            writer, item_0
        )
    writer.write_unsigned(len(value.suggestions))
    for item_0 in value.suggestions:
        destack._generated.protocol.source.diagnostic.suggestion.encode_diagnostic_suggestion(
            writer, item_0
        )
    writer.write_unsigned(len(value.tags))
    for item_0 in value.tags:
        destack._generated.protocol.source.diagnostic.severity.encode_diagnostic_tag(
            writer, item_0
        )


def decode_diagnostic(reader: Reader) -> Diagnostic:
    field_0 = reader.read_string()
    field_1 = destack._generated.protocol.source.diagnostic.severity.decode_diagnostic_severity(
        reader
    )
    field_2 = reader.read_string()
    field_3 = (
        destack._generated.protocol.source.diagnostic.label.decode_diagnostic_label(
            reader
        )
    )
    field_4 = [
        destack._generated.protocol.source.diagnostic.label.decode_diagnostic_label(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_5 = [
        destack._generated.protocol.source.diagnostic.label.decode_diagnostic_note(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_6 = [
        destack._generated.protocol.source.diagnostic.label.decode_diagnostic_help(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_7 = [
        destack._generated.protocol.source.diagnostic.suggestion.decode_diagnostic_suggestion(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_8 = [
        destack._generated.protocol.source.diagnostic.severity.decode_diagnostic_tag(
            reader
        )
        for _ in range(reader.read_number())
    ]

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
    "Diagnostic",
    "encode_diagnostic",
    "decode_diagnostic",
]
