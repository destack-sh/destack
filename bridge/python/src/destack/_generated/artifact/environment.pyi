# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string
import destack._generated.dir.symbol.key
import destack._generated.dir.symbol.language
import destack._generated.dir.symbol.symbol
import destack._generated.dir.table.import_
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class LanguageEnvironment:
    """Compiler-known language environment for one profile."""

    # language item symbols by item id
    symbol_by_item: Mapping[
        destack._generated.dir.symbol.language.LanguageItem,
        destack._generated.dir.symbol.symbol.GlobalSymbolId,
    ]
    # language items by symbol id
    items_by_symbol: Mapping[
        destack._generated.dir.symbol.symbol.GlobalSymbolId,
        destack._generated.dir.symbol.language.LanguageItem,
    ]
    # builtin symbols by export name
    symbols: Mapping[
        destack._generated.core.string.StringId,
        destack._generated.dir.symbol.symbol.GlobalSymbolId,
    ]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LanguageEnvironment: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LanguageEnvironment: ...

def encode_language_environment(
    writer: BinaryWriter, value: LanguageEnvironment
) -> None: ...
def decode_language_environment(reader: BinaryReader) -> LanguageEnvironment: ...
def to_json_language_environment(value: LanguageEnvironment) -> Json: ...
def from_json_language_environment(value: Json) -> LanguageEnvironment: ...

@dataclass(frozen=True, slots=True)
class GlobalEnvironment:
    """Explicit global environment selected for one profile."""

    # compiler-known language environment
    language: LanguageEnvironment
    # explicit global modules in load order
    globals: Sequence[destack._generated.source.file.model.module.ModuleId]
    # resolved global bindings by key across the global modules
    global_targets_by_key: Mapping[
        destack._generated.dir.symbol.key.StaticKey,
        Sequence[destack._generated.dir.table.import_.ImportTarget],
    ]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> GlobalEnvironment: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> GlobalEnvironment: ...

def encode_global_environment(
    writer: BinaryWriter, value: GlobalEnvironment
) -> None: ...
def decode_global_environment(reader: BinaryReader) -> GlobalEnvironment: ...
def to_json_global_environment(value: GlobalEnvironment) -> Json: ...
def from_json_global_environment(value: Json) -> GlobalEnvironment: ...

@dataclass(frozen=True, slots=True)
class LanguageIntrinsics:
    """Resolved compiler-known intrinsic bindings for a profile."""

    # intrinsic names keyed by symbol id
    names_by_symbol: Mapping[destack._generated.dir.symbol.symbol.GlobalSymbolId, str]
    # intrinsic symbols keyed by name
    symbols_by_name: Mapping[str, destack._generated.dir.symbol.symbol.GlobalSymbolId]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LanguageIntrinsics: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LanguageIntrinsics: ...

def encode_language_intrinsics(
    writer: BinaryWriter, value: LanguageIntrinsics
) -> None: ...
def decode_language_intrinsics(reader: BinaryReader) -> LanguageIntrinsics: ...
def to_json_language_intrinsics(value: LanguageIntrinsics) -> Json: ...
def from_json_language_intrinsics(value: Json) -> LanguageIntrinsics: ...

__all__ = [
    "LanguageEnvironment",
    "encode_language_environment",
    "decode_language_environment",
    "to_json_language_environment",
    "from_json_language_environment",
    "GlobalEnvironment",
    "encode_global_environment",
    "decode_global_environment",
    "to_json_global_environment",
    "from_json_global_environment",
    "LanguageIntrinsics",
    "encode_language_intrinsics",
    "decode_language_intrinsics",
    "to_json_language_intrinsics",
    "from_json_language_intrinsics",
]
