# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.export
import destack._generated.dir.symbol.key
import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class GlobalEntryLocal:
    """A local global declaration."""

    local: LocalGlobalEntry
    kind: typing.Literal["local"] = "local"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class GlobalEntryIndirect:
    """A re-exported global declaration."""

    indirect: IndirectGlobalEntry
    kind: typing.Literal["indirect"] = "indirect"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""One global table entry."""
GlobalEntry: typing.TypeAlias = GlobalEntryLocal | GlobalEntryIndirect

def encode_global_entry(writer: BinaryWriter, value: GlobalEntry) -> None: ...
def decode_global_entry(reader: BinaryReader) -> GlobalEntry: ...
def to_json_global_entry(value: GlobalEntry) -> Json: ...
def from_json_global_entry(value: Json) -> GlobalEntry: ...

@dataclass(frozen=True, slots=True)
class LocalGlobalEntry:
    """One local global declaration from a symbol declared in the current module."""

    # the global name
    key: destack._generated.dir.symbol.key.StaticKey
    # the local symbol exposed as a global
    source: destack._generated.dir.symbol.symbol.LocalSymbolId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LocalGlobalEntry: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LocalGlobalEntry: ...

def encode_local_global_entry(
    writer: BinaryWriter, value: LocalGlobalEntry
) -> None: ...
def decode_local_global_entry(reader: BinaryReader) -> LocalGlobalEntry: ...
def to_json_local_global_entry(value: LocalGlobalEntry) -> Json: ...
def from_json_local_global_entry(value: Json) -> LocalGlobalEntry: ...

@dataclass(frozen=True, slots=True)
class IndirectGlobalEntry:
    """One named global re-export from another module."""

    # the global name
    key: destack._generated.dir.symbol.key.StaticKey
    # the dependency item that declared the global export
    item: destack._generated.dir.tree.node.LocalNodeId
    # the target module selected by the export
    target: destack._generated.source.file.model.module.ModuleId | None
    # the export selected from the target module
    imported: destack._generated.dir.symbol.export.ExportSelector

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> IndirectGlobalEntry: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> IndirectGlobalEntry: ...

def encode_indirect_global_entry(
    writer: BinaryWriter, value: IndirectGlobalEntry
) -> None: ...
def decode_indirect_global_entry(reader: BinaryReader) -> IndirectGlobalEntry: ...
def to_json_indirect_global_entry(value: IndirectGlobalEntry) -> Json: ...
def from_json_indirect_global_entry(value: Json) -> IndirectGlobalEntry: ...

__all__ = [
    "GlobalEntry",
    "encode_global_entry",
    "decode_global_entry",
    "to_json_global_entry",
    "from_json_global_entry",
    "GlobalEntryLocal",
    "GlobalEntryIndirect",
    "LocalGlobalEntry",
    "encode_local_global_entry",
    "decode_local_global_entry",
    "to_json_local_global_entry",
    "from_json_local_global_entry",
    "IndirectGlobalEntry",
    "encode_indirect_global_entry",
    "decode_indirect_global_entry",
    "to_json_indirect_global_entry",
    "from_json_indirect_global_entry",
]
