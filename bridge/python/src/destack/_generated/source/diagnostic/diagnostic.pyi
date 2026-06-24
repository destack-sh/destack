# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Diagnostic: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Diagnostic: ...

def encode_diagnostic(writer: BinaryWriter, value: Diagnostic) -> None: ...
def decode_diagnostic(reader: BinaryReader) -> Diagnostic: ...
def to_json_diagnostic(value: Diagnostic) -> Json: ...
def from_json_diagnostic(value: Json) -> Diagnostic: ...

__all__ = [
    "Diagnostic",
    "encode_diagnostic",
    "decode_diagnostic",
    "to_json_diagnostic",
    "from_json_diagnostic",
]
