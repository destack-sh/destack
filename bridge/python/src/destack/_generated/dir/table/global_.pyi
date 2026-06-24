# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.global_
import destack._generated.dir.symbol.key
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class GlobalTable:
    """Global names contributed by one module."""

    # the module id of the global table
    module_id: destack._generated.source.file.model.module.ModuleId
    # global entries keyed by visible global name
    entries_by_key: Mapping[
        destack._generated.dir.symbol.key.StaticKey,
        Sequence[destack._generated.dir.symbol.global_.GlobalEntry],
    ]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> GlobalTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> GlobalTable: ...

def encode_global_table(writer: BinaryWriter, value: GlobalTable) -> None: ...
def decode_global_table(reader: BinaryReader) -> GlobalTable: ...
def to_json_global_table(value: GlobalTable) -> Json: ...
def from_json_global_table(value: Json) -> GlobalTable: ...

__all__ = [
    "GlobalTable",
    "encode_global_table",
    "decode_global_table",
    "to_json_global_table",
    "from_json_global_table",
]
