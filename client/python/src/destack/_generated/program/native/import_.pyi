# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class ImportTable:
    """Native imports required by one native code payload."""

    # native imports in linker order
    import_: Sequence[Import]

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

@dataclass(frozen=True, slots=True)
class ImportRuntime:
    """Fixed Destack runtime binding."""

    runtime: RuntimeBinding
    kind: typing.Literal["runtime"] = "runtime"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ImportSymbol:
    """External linker-visible symbol."""

    symbol: SymbolImport
    kind: typing.Literal["symbol"] = "symbol"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""One native import required by generated native code."""
Import: typing.TypeAlias = ImportRuntime | ImportSymbol

def encode_import(writer: BinaryWriter, value: Import) -> None: ...
def decode_import(reader: BinaryReader) -> Import: ...
def to_json_import(value: Import) -> Json: ...
def from_json_import(value: Json) -> Import: ...

"""Fixed Destack runtime ABI binding imported by generated native code."""
RuntimeBinding: typing.TypeAlias = (
    typing.Literal["new"]
    | typing.Literal["newSlice"]
    | typing.Literal["free"]
    | typing.Literal["pin"]
    | typing.Literal["unpin"]
    | typing.Literal["writeBarrier"]
    | typing.Literal["safepoint"]
    | typing.Literal["yield"]
    | typing.Literal["deopt"]
    | typing.Literal["trap"]
    | typing.Literal["panic"]
    | typing.Literal["unwindResume"]
    | typing.Literal["contextCurrent"]
    | typing.Literal["contextPush"]
    | typing.Literal["contextPop"]
    | typing.Literal["contextGet"]
    | typing.Literal["contextRequire"]
    | typing.Literal["contextFamily"]
    | typing.Literal["bindingCall"]
)

def encode_runtime_binding(writer: BinaryWriter, value: RuntimeBinding) -> None: ...
def decode_runtime_binding(reader: BinaryReader) -> RuntimeBinding: ...
def to_json_runtime_binding(value: RuntimeBinding) -> Json: ...
def from_json_runtime_binding(value: Json) -> RuntimeBinding: ...

@dataclass(frozen=True, slots=True)
class SymbolImport:
    """External linker-visible symbol import."""

    # the imported native symbol
    symbol: str

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SymbolImport: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SymbolImport: ...

def encode_symbol_import(writer: BinaryWriter, value: SymbolImport) -> None: ...
def decode_symbol_import(reader: BinaryReader) -> SymbolImport: ...
def to_json_symbol_import(value: SymbolImport) -> Json: ...
def from_json_symbol_import(value: Json) -> SymbolImport: ...

__all__ = [
    "ImportTable",
    "encode_import_table",
    "decode_import_table",
    "to_json_import_table",
    "from_json_import_table",
    "Import",
    "encode_import",
    "decode_import",
    "to_json_import",
    "from_json_import",
    "ImportRuntime",
    "ImportSymbol",
    "RuntimeBinding",
    "encode_runtime_binding",
    "decode_runtime_binding",
    "to_json_runtime_binding",
    "from_json_runtime_binding",
    "SymbolImport",
    "encode_symbol_import",
    "decode_symbol_import",
    "to_json_symbol_import",
    "from_json_symbol_import",
]
