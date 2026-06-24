# generated bridge target, do not edit

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
)

import destack._generated.source.diagnostic.diagnostic


@dataclass(frozen=True, slots=True)
class DiagnosticCollection:
    """A collection of diagnostics."""

    # the diagnostics
    diagnostics: Sequence[destack._generated.source.diagnostic.diagnostic.Diagnostic]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_diagnostic_collection(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DiagnosticCollection:
        """Decode one DiagnosticCollection."""
        return decode_diagnostic_collection(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_diagnostic_collection(self)

    @classmethod
    def from_json(cls, value: Json) -> DiagnosticCollection:
        """Return one DiagnosticCollection from one JSON value."""
        return from_json_diagnostic_collection(value)


def encode_diagnostic_collection(
    writer: BinaryWriter, value: DiagnosticCollection
) -> None:
    """Encode one DiagnosticCollection."""
    writer.write_unsigned(len(value.diagnostics))
    for item_value_diagnostics_0 in value.diagnostics:
        destack._generated.source.diagnostic.diagnostic.encode_diagnostic(
            writer, item_value_diagnostics_0
        )


def decode_diagnostic_collection(reader: BinaryReader) -> DiagnosticCollection:
    """Decode one DiagnosticCollection."""
    diagnostics = [
        destack._generated.source.diagnostic.diagnostic.decode_diagnostic(reader)
        for _ in range(reader.read_number())
    ]

    return DiagnosticCollection(
        diagnostics=diagnostics,
    )


def to_json_diagnostic_collection(value: DiagnosticCollection) -> Json:
    """Return one JSON value for one DiagnosticCollection."""
    return {
        "diagnostics": [
            destack._generated.source.diagnostic.diagnostic.to_json_diagnostic(item_0)
            for item_0 in value.diagnostics
        ],
    }


def from_json_diagnostic_collection(value: Json) -> DiagnosticCollection:
    """Return one DiagnosticCollection from one JSON value."""
    object_ = json_object(value)

    return DiagnosticCollection(
        diagnostics=[
            destack._generated.source.diagnostic.diagnostic.from_json_diagnostic(item_0)
            for item_0 in json_array(json_field(object_, "diagnostics"))
        ],
    )


__all__ = [
    "DiagnosticCollection",
    "encode_diagnostic_collection",
    "decode_diagnostic_collection",
    "to_json_diagnostic_collection",
    "from_json_diagnostic_collection",
]
