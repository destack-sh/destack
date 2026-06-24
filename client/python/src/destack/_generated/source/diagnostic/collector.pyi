# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.source.diagnostic.diagnostic

@dataclass(frozen=True, slots=True)
class DiagnosticCollection:
    """A collection of diagnostics."""

    # the diagnostics
    diagnostics: Sequence[destack._generated.source.diagnostic.diagnostic.Diagnostic]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DiagnosticCollection: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DiagnosticCollection: ...

def encode_diagnostic_collection(
    writer: BinaryWriter, value: DiagnosticCollection
) -> None: ...
def decode_diagnostic_collection(reader: BinaryReader) -> DiagnosticCollection: ...
def to_json_diagnostic_collection(value: DiagnosticCollection) -> Json: ...
def from_json_diagnostic_collection(value: Json) -> DiagnosticCollection: ...

__all__ = [
    "DiagnosticCollection",
    "encode_diagnostic_collection",
    "decode_diagnostic_collection",
    "to_json_diagnostic_collection",
    "from_json_diagnostic_collection",
]
