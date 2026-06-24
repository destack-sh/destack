# generated client target, do not edit

from __future__ import annotations

import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

"""The level of a diagnostic."""
DiagnosticSeverity: typing.TypeAlias = (
    typing.Literal["note"] | typing.Literal["warning"] | typing.Literal["error"]
)

def encode_diagnostic_severity(
    writer: BinaryWriter, value: DiagnosticSeverity
) -> None: ...
def decode_diagnostic_severity(reader: BinaryReader) -> DiagnosticSeverity: ...
def to_json_diagnostic_severity(value: DiagnosticSeverity) -> Json: ...
def from_json_diagnostic_severity(value: Json) -> DiagnosticSeverity: ...

"""Extra semantic tag for a diagnostic."""
DiagnosticTag: typing.TypeAlias = (
    typing.Literal["unnecessary"] | typing.Literal["deprecated"]
)

def encode_diagnostic_tag(writer: BinaryWriter, value: DiagnosticTag) -> None: ...
def decode_diagnostic_tag(reader: BinaryReader) -> DiagnosticTag: ...
def to_json_diagnostic_tag(value: DiagnosticTag) -> Json: ...
def from_json_diagnostic_tag(value: Json) -> DiagnosticTag: ...

__all__ = [
    "DiagnosticSeverity",
    "encode_diagnostic_severity",
    "decode_diagnostic_severity",
    "to_json_diagnostic_severity",
    "from_json_diagnostic_severity",
    "DiagnosticTag",
    "encode_diagnostic_tag",
    "decode_diagnostic_tag",
    "to_json_diagnostic_tag",
    "from_json_diagnostic_tag",
]
