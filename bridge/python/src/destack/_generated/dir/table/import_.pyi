# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.key
import destack._generated.dir.symbol.language
import destack._generated.dir.symbol.symbol
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class ImportTargetSymbol:
    """A symbol exported by a target module."""

    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    kind: typing.Literal["symbol"] = "symbol"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ImportTargetNamespace:
    """A namespace object for a target module."""

    namespace: destack._generated.source.file.model.module.ModuleId
    kind: typing.Literal["namespace"] = "namespace"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Target selected by one import binding."""
ImportTarget: typing.TypeAlias = ImportTargetSymbol | ImportTargetNamespace

def encode_import_target(writer: BinaryWriter, value: ImportTarget) -> None: ...
def decode_import_target(reader: BinaryReader) -> ImportTarget: ...
def to_json_import_target(value: ImportTarget) -> Json: ...
def from_json_import_target(value: Json) -> ImportTarget: ...

@dataclass(frozen=True, slots=True)
class ImportTable:
    """Resolved import targets for one module."""

    # the module id of the import table
    module_id: destack._generated.source.file.model.module.ModuleId
    # modules reached by resolved imports and active globals
    modules: Sequence[destack._generated.source.file.model.module.ModuleId]
    # imported local symbols keyed to their resolved target
    target_by_symbol: Mapping[
        destack._generated.dir.symbol.symbol.LocalSymbolId, ImportTarget
    ]
    # global targets made visible by the active profile
    global_target_by_key: Mapping[
        destack._generated.dir.symbol.key.StaticKey, Sequence[ImportTarget]
    ]
    # language item symbols required by compiler syntax
    language_symbol_by_item: Mapping[
        destack._generated.dir.symbol.language.LanguageItem,
        destack._generated.dir.symbol.symbol.GlobalSymbolId,
    ]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ImportTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ImportTable: ...

def encode_import_table(writer: BinaryWriter, value: ImportTable) -> None: ...
def decode_import_table(reader: BinaryReader) -> ImportTable: ...
def to_json_import_table(value: ImportTable) -> Json: ...
def from_json_import_table(value: Json) -> ImportTable: ...

__all__ = [
    "ImportTarget",
    "encode_import_target",
    "decode_import_target",
    "to_json_import_target",
    "from_json_import_target",
    "ImportTargetSymbol",
    "ImportTargetNamespace",
    "ImportTable",
    "encode_import_table",
    "decode_import_table",
    "to_json_import_table",
    "from_json_import_table",
]
