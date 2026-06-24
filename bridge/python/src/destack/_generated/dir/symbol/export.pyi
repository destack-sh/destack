# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.key
import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class ExportKeyDefault:
    """The ECMAScript default export name."""

    kind: typing.Literal["default"] = "default"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExportKeyNamed:
    """A named export key."""

    named: destack._generated.dir.symbol.key.StaticKey
    kind: typing.Literal["named"] = "named"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""The exported name in one module record."""
ExportKey: typing.TypeAlias = ExportKeyDefault | ExportKeyNamed

def encode_export_key(writer: BinaryWriter, value: ExportKey) -> None: ...
def decode_export_key(reader: BinaryReader) -> ExportKey: ...
def to_json_export_key(value: ExportKey) -> Json: ...
def from_json_export_key(value: Json) -> ExportKey: ...

@dataclass(frozen=True, slots=True)
class ExportEntryLocal:
    """A local export."""

    local: LocalExportEntry
    kind: typing.Literal["local"] = "local"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExportEntryIndirect:
    """A re-export from another module."""

    indirect: IndirectExportEntry
    kind: typing.Literal["indirect"] = "indirect"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""One named export entry."""
ExportEntry: typing.TypeAlias = ExportEntryLocal | ExportEntryIndirect

def encode_export_entry(writer: BinaryWriter, value: ExportEntry) -> None: ...
def decode_export_entry(reader: BinaryReader) -> ExportEntry: ...
def to_json_export_entry(value: ExportEntry) -> Json: ...
def from_json_export_entry(value: Json) -> ExportEntry: ...

@dataclass(frozen=True, slots=True)
class LocalExportEntry:
    """One local export from a symbol declared in the current module."""

    # the exported name
    key: ExportKey
    # the local symbol exposed by the export
    source: destack._generated.dir.symbol.symbol.LocalSymbolId
    # the export clause item that declared this export
    item: destack._generated.dir.tree.node.LocalNodeId | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LocalExportEntry: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LocalExportEntry: ...

def encode_local_export_entry(
    writer: BinaryWriter, value: LocalExportEntry
) -> None: ...
def decode_local_export_entry(reader: BinaryReader) -> LocalExportEntry: ...
def to_json_local_export_entry(value: LocalExportEntry) -> Json: ...
def from_json_local_export_entry(value: Json) -> LocalExportEntry: ...

@dataclass(frozen=True, slots=True)
class IndirectExportEntry:
    """One named re-export from another module."""

    # the exported name in the current module
    key: ExportKey
    # the dependency item that declared the export
    item: destack._generated.dir.tree.node.LocalNodeId
    # the target module selected by the export
    target: destack._generated.source.file.model.module.ModuleId | None
    # the export selected from the target module
    imported: ExportSelector

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> IndirectExportEntry: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> IndirectExportEntry: ...

def encode_indirect_export_entry(
    writer: BinaryWriter, value: IndirectExportEntry
) -> None: ...
def decode_indirect_export_entry(reader: BinaryReader) -> IndirectExportEntry: ...
def to_json_indirect_export_entry(value: IndirectExportEntry) -> Json: ...
def from_json_indirect_export_entry(value: Json) -> IndirectExportEntry: ...

@dataclass(frozen=True, slots=True)
class ExportSelectorNamed:
    """A named target export."""

    named: destack._generated.dir.symbol.key.StaticKey
    kind: typing.Literal["named"] = "named"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExportSelectorDefault:
    """The target module default export."""

    kind: typing.Literal["default"] = "default"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExportSelectorNamespace:
    """The target module namespace object."""

    kind: typing.Literal["namespace"] = "namespace"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Which binding a re-export selects from the target module."""
ExportSelector: typing.TypeAlias = (
    ExportSelectorNamed | ExportSelectorDefault | ExportSelectorNamespace
)

def encode_export_selector(writer: BinaryWriter, value: ExportSelector) -> None: ...
def decode_export_selector(reader: BinaryReader) -> ExportSelector: ...
def to_json_export_selector(value: ExportSelector) -> Json: ...
def from_json_export_selector(value: Json) -> ExportSelector: ...

@dataclass(frozen=True, slots=True)
class StarExportEntry:
    """One `export * from` edge."""

    # the dependency item that declared the star export
    item: destack._generated.dir.tree.node.LocalNodeId
    # the target module selected by the export
    target: destack._generated.source.file.model.module.ModuleId | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> StarExportEntry: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> StarExportEntry: ...

def encode_star_export_entry(writer: BinaryWriter, value: StarExportEntry) -> None: ...
def decode_star_export_entry(reader: BinaryReader) -> StarExportEntry: ...
def to_json_star_export_entry(value: StarExportEntry) -> Json: ...
def from_json_star_export_entry(value: Json) -> StarExportEntry: ...

__all__ = [
    "ExportKey",
    "encode_export_key",
    "decode_export_key",
    "to_json_export_key",
    "from_json_export_key",
    "ExportKeyDefault",
    "ExportKeyNamed",
    "ExportEntry",
    "encode_export_entry",
    "decode_export_entry",
    "to_json_export_entry",
    "from_json_export_entry",
    "ExportEntryLocal",
    "ExportEntryIndirect",
    "LocalExportEntry",
    "encode_local_export_entry",
    "decode_local_export_entry",
    "to_json_local_export_entry",
    "from_json_local_export_entry",
    "IndirectExportEntry",
    "encode_indirect_export_entry",
    "decode_indirect_export_entry",
    "to_json_indirect_export_entry",
    "from_json_indirect_export_entry",
    "ExportSelector",
    "encode_export_selector",
    "decode_export_selector",
    "to_json_export_selector",
    "from_json_export_selector",
    "ExportSelectorNamed",
    "ExportSelectorDefault",
    "ExportSelectorNamespace",
    "StarExportEntry",
    "encode_star_export_entry",
    "decode_star_export_entry",
    "to_json_star_export_entry",
    "from_json_star_export_entry",
]
