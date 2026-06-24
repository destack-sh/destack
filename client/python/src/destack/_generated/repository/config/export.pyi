# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.repository.config.condition

@dataclass(frozen=True, slots=True)
class Export:
    """Public package material declaration."""

    # exported material kind
    kind: ExportKind
    # package relative material path
    path: str
    # condition predicate required for this export
    when: destack._generated.repository.config.condition.ConditionRef | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Export: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Export: ...

def encode_export(writer: BinaryWriter, value: Export) -> None: ...
def decode_export(reader: BinaryReader) -> Export: ...
def to_json_export(value: Export) -> Json: ...
def from_json_export(value: Json) -> Export: ...

"""Public package material kind."""
ExportKind: typing.TypeAlias = (
    typing.Literal["module"]
    | typing.Literal["asset"]
    | typing.Literal["template"]
    | typing.Literal["reflect"]
    | typing.Literal["simulation"]
    | typing.Literal["service"]
    | typing.Literal["app"]
)

def encode_export_kind(writer: BinaryWriter, value: ExportKind) -> None: ...
def decode_export_kind(reader: BinaryReader) -> ExportKind: ...
def to_json_export_kind(value: ExportKind) -> Json: ...
def from_json_export_kind(value: Json) -> ExportKind: ...

__all__ = [
    "Export",
    "encode_export",
    "decode_export",
    "to_json_export",
    "from_json_export",
    "ExportKind",
    "encode_export_kind",
    "decode_export_kind",
    "to_json_export_kind",
    "from_json_export_kind",
]
