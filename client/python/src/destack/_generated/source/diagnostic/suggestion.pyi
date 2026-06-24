# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.source.diagnostic.label
import destack._generated.source.edit.edit

"""Whether a suggestion can be applied automatically."""
Applicability: typing.TypeAlias = (
    typing.Literal["automatic"] | typing.Literal["unsafe"] | typing.Literal["dangerous"]
)

def encode_applicability(writer: BinaryWriter, value: Applicability) -> None: ...
def decode_applicability(reader: BinaryReader) -> Applicability: ...
def to_json_applicability(value: Applicability) -> Json: ...
def from_json_applicability(value: Json) -> Applicability: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DiagnosticSuggestion: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DiagnosticSuggestion: ...

def encode_diagnostic_suggestion(
    writer: BinaryWriter, value: DiagnosticSuggestion
) -> None: ...
def decode_diagnostic_suggestion(reader: BinaryReader) -> DiagnosticSuggestion: ...
def to_json_diagnostic_suggestion(value: DiagnosticSuggestion) -> Json: ...
def from_json_diagnostic_suggestion(value: Json) -> DiagnosticSuggestion: ...

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
