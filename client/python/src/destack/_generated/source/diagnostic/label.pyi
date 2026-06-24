# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DiagnosticLabel: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DiagnosticLabel: ...

def encode_diagnostic_label(writer: BinaryWriter, value: DiagnosticLabel) -> None: ...
def decode_diagnostic_label(reader: BinaryReader) -> DiagnosticLabel: ...
def to_json_diagnostic_label(value: DiagnosticLabel) -> Json: ...
def from_json_diagnostic_label(value: Json) -> DiagnosticLabel: ...

@dataclass(frozen=True, slots=True)
class DiagnosticNote:
    """Extra context for understanding a diagnostic."""

    # the note message
    message: str

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DiagnosticNote: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DiagnosticNote: ...

def encode_diagnostic_note(writer: BinaryWriter, value: DiagnosticNote) -> None: ...
def decode_diagnostic_note(reader: BinaryReader) -> DiagnosticNote: ...
def to_json_diagnostic_note(value: DiagnosticNote) -> Json: ...
def from_json_diagnostic_note(value: Json) -> DiagnosticNote: ...

@dataclass(frozen=True, slots=True)
class DiagnosticHelp:
    """Guidance for fixing or avoiding a diagnostic."""

    # the help message
    message: str

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DiagnosticHelp: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DiagnosticHelp: ...

def encode_diagnostic_help(writer: BinaryWriter, value: DiagnosticHelp) -> None: ...
def decode_diagnostic_help(reader: BinaryReader) -> DiagnosticHelp: ...
def to_json_diagnostic_help(value: DiagnosticHelp) -> Json: ...
def from_json_diagnostic_help(value: Json) -> DiagnosticHelp: ...

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
