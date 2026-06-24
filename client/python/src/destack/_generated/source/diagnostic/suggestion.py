# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_array,
    json_field,
    json_object,
    json_string,
)

import destack._generated.source.diagnostic.label
import destack._generated.source.edit.edit

"""Whether a suggestion can be applied automatically."""
Applicability: typing.TypeAlias = (
    typing.Literal["automatic"] | typing.Literal["unsafe"] | typing.Literal["dangerous"]
)


def encode_applicability(writer: BinaryWriter, value: Applicability) -> None:
    """Encode one Applicability."""
    if value == "automatic":
        writer.write_unsigned(0)
    elif value == "unsafe":
        writer.write_unsigned(1)
    elif value == "dangerous":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_applicability(reader: BinaryReader) -> Applicability:
    """Decode one Applicability."""
    variant = reader.read_number()

    if variant == 0:
        return "automatic"
    elif variant == 1:
        return "unsafe"
    elif variant == 2:
        return "dangerous"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_applicability(value: Applicability) -> Json:
    """Return one JSON value for one Applicability."""
    return value


def from_json_applicability(value: Json) -> Applicability:
    """Return one Applicability from one JSON value."""
    variant = json_string(value)

    if variant == "automatic":
        return "automatic"
    elif variant == "unsafe":
        return "unsafe"
    elif variant == "dangerous":
        return "dangerous"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class DiagnosticSuggestion:
    """One suggested source change for a diagnostic."""

    # exact source patches for machine application
    patches: destack._generated.source.edit.edit.PatchSet
    # source labels to show with the suggestion
    labels: Sequence[destack._generated.source.diagnostic.label.DiagnosticLabel]
    # the message of the suggestion
    message: str
    # the applicability of the suggestion
    applicability: Applicability

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_diagnostic_suggestion(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DiagnosticSuggestion:
        """Decode one DiagnosticSuggestion."""
        return decode_diagnostic_suggestion(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_diagnostic_suggestion(self)

    @classmethod
    def from_json(cls, value: Json) -> DiagnosticSuggestion:
        """Return one DiagnosticSuggestion from one JSON value."""
        return from_json_diagnostic_suggestion(value)


def encode_diagnostic_suggestion(
    writer: BinaryWriter, value: DiagnosticSuggestion
) -> None:
    """Encode one DiagnosticSuggestion."""
    destack._generated.source.edit.edit.encode_patch_set(writer, value.patches)
    writer.write_unsigned(len(value.labels))
    for item_value_labels_0 in value.labels:
        destack._generated.source.diagnostic.label.encode_diagnostic_label(
            writer, item_value_labels_0
        )
    writer.write_string(value.message)
    encode_applicability(writer, value.applicability)


def decode_diagnostic_suggestion(reader: BinaryReader) -> DiagnosticSuggestion:
    """Decode one DiagnosticSuggestion."""
    patches = destack._generated.source.edit.edit.decode_patch_set(reader)
    labels = [
        destack._generated.source.diagnostic.label.decode_diagnostic_label(reader)
        for _ in range(reader.read_number())
    ]
    message = reader.read_string()
    applicability = decode_applicability(reader)

    return DiagnosticSuggestion(
        patches=patches,
        labels=labels,
        message=message,
        applicability=applicability,
    )


def to_json_diagnostic_suggestion(value: DiagnosticSuggestion) -> Json:
    """Return one JSON value for one DiagnosticSuggestion."""
    return {
        "patches": destack._generated.source.edit.edit.to_json_patch_set(value.patches),
        "labels": [
            destack._generated.source.diagnostic.label.to_json_diagnostic_label(item_0)
            for item_0 in value.labels
        ],
        "message": value.message,
        "applicability": to_json_applicability(value.applicability),
    }


def from_json_diagnostic_suggestion(value: Json) -> DiagnosticSuggestion:
    """Return one DiagnosticSuggestion from one JSON value."""
    object_ = json_object(value)

    return DiagnosticSuggestion(
        patches=destack._generated.source.edit.edit.from_json_patch_set(
            json_field(object_, "patches")
        ),
        labels=[
            destack._generated.source.diagnostic.label.from_json_diagnostic_label(
                item_0
            )
            for item_0 in json_array(json_field(object_, "labels"))
        ],
        message=json_string(json_field(object_, "message")),
        applicability=from_json_applicability(json_field(object_, "applicability")),
    )


__all__ = [
    "Applicability",
    "encode_applicability",
    "decode_applicability",
    "to_json_applicability",
    "from_json_applicability",
    "DiagnosticSuggestion",
    "encode_diagnostic_suggestion",
    "decode_diagnostic_suggestion",
    "to_json_diagnostic_suggestion",
    "from_json_diagnostic_suggestion",
]
