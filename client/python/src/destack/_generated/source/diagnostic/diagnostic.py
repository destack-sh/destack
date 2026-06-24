# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_field,
    json_object,
    json_string,
)

import destack._generated.source.diagnostic.label
import destack._generated.source.diagnostic.severity
import destack._generated.source.diagnostic.suggestion


@dataclass(frozen=True, slots=True)
class Diagnostic:
    """A final renderable diagnostic."""

    # the stable identifier of the diagnostic (like `E001` or `W017`)
    code: str
    # the DiagnosticSeverity of the diagnostic
    severity: destack._generated.source.diagnostic.severity.DiagnosticSeverity
    # the message of the diagnostic
    message: str
    # the main source label
    primary: destack._generated.source.diagnostic.label.DiagnosticLabel
    # additional source labels
    labels: Sequence[destack._generated.source.diagnostic.label.DiagnosticLabel]
    # extra context for understanding the diagnostic
    notes: Sequence[destack._generated.source.diagnostic.label.DiagnosticNote]
    # guidance for fixing or avoiding the diagnostic
    helps: Sequence[destack._generated.source.diagnostic.label.DiagnosticHelp]
    # the suggestions for the diagnostic
    suggestions: Sequence[
        destack._generated.source.diagnostic.suggestion.DiagnosticSuggestion
    ]
    # extra semantic tags
    tags: Sequence[destack._generated.source.diagnostic.severity.DiagnosticTag]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_diagnostic(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Diagnostic:
        """Decode one Diagnostic."""
        return decode_diagnostic(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_diagnostic(self)

    @classmethod
    def from_json(cls, value: Json) -> Diagnostic:
        """Return one Diagnostic from one JSON value."""
        return from_json_diagnostic(value)


def encode_diagnostic(writer: BinaryWriter, value: Diagnostic) -> None:
    """Encode one Diagnostic."""
    writer.write_string(value.code)
    destack._generated.source.diagnostic.severity.encode_diagnostic_severity(
        writer, value.severity
    )
    writer.write_string(value.message)
    destack._generated.source.diagnostic.label.encode_diagnostic_label(
        writer, value.primary
    )
    writer.write_unsigned(len(value.labels))
    for item_value_labels_0 in value.labels:
        destack._generated.source.diagnostic.label.encode_diagnostic_label(
            writer, item_value_labels_0
        )
    writer.write_unsigned(len(value.notes))
    for item_value_notes_0 in value.notes:
        destack._generated.source.diagnostic.label.encode_diagnostic_note(
            writer, item_value_notes_0
        )
    writer.write_unsigned(len(value.helps))
    for item_value_helps_0 in value.helps:
        destack._generated.source.diagnostic.label.encode_diagnostic_help(
            writer, item_value_helps_0
        )
    writer.write_unsigned(len(value.suggestions))
    for item_value_suggestions_0 in value.suggestions:
        destack._generated.source.diagnostic.suggestion.encode_diagnostic_suggestion(
            writer, item_value_suggestions_0
        )
    writer.write_unsigned(len(value.tags))
    for item_value_tags_0 in value.tags:
        destack._generated.source.diagnostic.severity.encode_diagnostic_tag(
            writer, item_value_tags_0
        )


def decode_diagnostic(reader: BinaryReader) -> Diagnostic:
    """Decode one Diagnostic."""
    code = reader.read_string()
    severity = destack._generated.source.diagnostic.severity.decode_diagnostic_severity(
        reader
    )
    message = reader.read_string()
    primary = destack._generated.source.diagnostic.label.decode_diagnostic_label(reader)
    labels = [
        destack._generated.source.diagnostic.label.decode_diagnostic_label(reader)
        for _ in range(reader.read_number())
    ]
    notes = [
        destack._generated.source.diagnostic.label.decode_diagnostic_note(reader)
        for _ in range(reader.read_number())
    ]
    helps = [
        destack._generated.source.diagnostic.label.decode_diagnostic_help(reader)
        for _ in range(reader.read_number())
    ]
    suggestions = [
        destack._generated.source.diagnostic.suggestion.decode_diagnostic_suggestion(
            reader
        )
        for _ in range(reader.read_number())
    ]
    tags = [
        destack._generated.source.diagnostic.severity.decode_diagnostic_tag(reader)
        for _ in range(reader.read_number())
    ]

    return Diagnostic(
        code=code,
        severity=severity,
        message=message,
        primary=primary,
        labels=labels,
        notes=notes,
        helps=helps,
        suggestions=suggestions,
        tags=tags,
    )


def to_json_diagnostic(value: Diagnostic) -> Json:
    """Return one JSON value for one Diagnostic."""
    return {
        "code": value.code,
        "severity": destack._generated.source.diagnostic.severity.to_json_diagnostic_severity(
            value.severity
        ),
        "message": value.message,
        "primary": destack._generated.source.diagnostic.label.to_json_diagnostic_label(
            value.primary
        ),
        "labels": [
            destack._generated.source.diagnostic.label.to_json_diagnostic_label(item_0)
            for item_0 in value.labels
        ],
        "notes": [
            destack._generated.source.diagnostic.label.to_json_diagnostic_note(item_0)
            for item_0 in value.notes
        ],
        "helps": [
            destack._generated.source.diagnostic.label.to_json_diagnostic_help(item_0)
            for item_0 in value.helps
        ],
        "suggestions": [
            destack._generated.source.diagnostic.suggestion.to_json_diagnostic_suggestion(
                item_0
            )
            for item_0 in value.suggestions
        ],
        "tags": [
            destack._generated.source.diagnostic.severity.to_json_diagnostic_tag(item_0)
            for item_0 in value.tags
        ],
    }


def from_json_diagnostic(value: Json) -> Diagnostic:
    """Return one Diagnostic from one JSON value."""
    object_ = json_object(value)

    return Diagnostic(
        code=json_string(json_field(object_, "code")),
        severity=destack._generated.source.diagnostic.severity.from_json_diagnostic_severity(
            json_field(object_, "severity")
        ),
        message=json_string(json_field(object_, "message")),
        primary=destack._generated.source.diagnostic.label.from_json_diagnostic_label(
            json_field(object_, "primary")
        ),
        labels=[
            destack._generated.source.diagnostic.label.from_json_diagnostic_label(
                item_0
            )
            for item_0 in json_array(json_field(object_, "labels"))
        ],
        notes=[
            destack._generated.source.diagnostic.label.from_json_diagnostic_note(item_0)
            for item_0 in json_array(json_field(object_, "notes"))
        ],
        helps=[
            destack._generated.source.diagnostic.label.from_json_diagnostic_help(item_0)
            for item_0 in json_array(json_field(object_, "helps"))
        ],
        suggestions=[
            destack._generated.source.diagnostic.suggestion.from_json_diagnostic_suggestion(
                item_0
            )
            for item_0 in json_array(json_field(object_, "suggestions"))
        ],
        tags=[
            destack._generated.source.diagnostic.severity.from_json_diagnostic_tag(
                item_0
            )
            for item_0 in json_array(json_field(object_, "tags"))
        ],
    )


__all__ = [
    "Diagnostic",
    "encode_diagnostic",
    "decode_diagnostic",
    "to_json_diagnostic",
    "from_json_diagnostic",
]
