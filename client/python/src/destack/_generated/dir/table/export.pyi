# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.export
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class ExportTable:
    """Resolved module exports for one module."""

    # the module id of the export table
    module_id: destack._generated.source.file.model.module.ModuleId
    # local and indirect exports keyed by exported name
    export_by_key: Mapping[
        destack._generated.dir.symbol.export.ExportKey,
        destack._generated.dir.symbol.export.ExportEntry,
    ]
    # star exports declared by the module
    star_exports: Sequence[destack._generated.dir.symbol.export.StarExportEntry]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ExportTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ExportTable: ...

def encode_export_table(writer: BinaryWriter, value: ExportTable) -> None: ...
def decode_export_table(reader: BinaryReader) -> ExportTable: ...
def to_json_export_table(value: ExportTable) -> Json: ...
def from_json_export_table(value: Json) -> ExportTable: ...

__all__ = [
    "ExportTable",
    "encode_export_table",
    "decode_export_table",
    "to_json_export_table",
    "from_json_export_table",
]
