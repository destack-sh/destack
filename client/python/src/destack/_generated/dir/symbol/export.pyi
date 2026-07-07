# generated client target, do not edit

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
class NamedExportLocal:
    """A local export."""

    local: LocalExport
    kind: typing.Literal["local"] = "local"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class NamedExportIndirect:
    """A re-export from another module."""

    indirect: IndirectExport
    kind: typing.Literal["indirect"] = "indirect"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""One named export entry."""
NamedExport: typing.TypeAlias = NamedExportLocal | NamedExportIndirect

def encode_named_export(writer: BinaryWriter, value: NamedExport) -> None: ...
def decode_named_export(reader: BinaryReader) -> NamedExport: ...
def to_json_named_export(value: NamedExport) -> Json: ...
def from_json_named_export(value: Json) -> NamedExport: ...

@dataclass(frozen=True, slots=True)
class LocalExport:
    """One local export from a symbol declared in the current module."""

    # the exported name
    key: ExportKey
    # the local symbol exposed by the export
    source: destack._generated.dir.symbol.symbol.LocalSymbolId
    # the export clause item that declared this export
    item: destack._generated.dir.tree.node.LocalNodeId | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LocalExport: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LocalExport: ...

def encode_local_export(writer: BinaryWriter, value: LocalExport) -> None: ...
def decode_local_export(reader: BinaryReader) -> LocalExport: ...
def to_json_local_export(value: LocalExport) -> Json: ...
def from_json_local_export(value: Json) -> LocalExport: ...

@dataclass(frozen=True, slots=True)
class IndirectExport:
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
    def decode(cls, reader: BinaryReader) -> IndirectExport: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> IndirectExport: ...

def encode_indirect_export(writer: BinaryWriter, value: IndirectExport) -> None: ...
def decode_indirect_export(reader: BinaryReader) -> IndirectExport: ...
def to_json_indirect_export(value: IndirectExport) -> Json: ...
def from_json_indirect_export(value: Json) -> IndirectExport: ...

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
class StarExport:
    """One `export * from` edge."""

    # the dependency item that declared the star export
    item: destack._generated.dir.tree.node.LocalNodeId
    # the target module selected by the export
    target: destack._generated.source.file.model.module.ModuleId | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> StarExport: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> StarExport: ...

def encode_star_export(writer: BinaryWriter, value: StarExport) -> None: ...
def decode_star_export(reader: BinaryReader) -> StarExport: ...
def to_json_star_export(value: StarExport) -> Json: ...
def from_json_star_export(value: Json) -> StarExport: ...

__all__ = [
    "ExportKey",
    "encode_export_key",
    "decode_export_key",
    "to_json_export_key",
    "from_json_export_key",
    "ExportKeyDefault",
    "ExportKeyNamed",
    "NamedExport",
    "encode_named_export",
    "decode_named_export",
    "to_json_named_export",
    "from_json_named_export",
    "NamedExportLocal",
    "NamedExportIndirect",
    "LocalExport",
    "encode_local_export",
    "decode_local_export",
    "to_json_local_export",
    "from_json_local_export",
    "IndirectExport",
    "encode_indirect_export",
    "decode_indirect_export",
    "to_json_indirect_export",
    "from_json_indirect_export",
    "ExportSelector",
    "encode_export_selector",
    "decode_export_selector",
    "to_json_export_selector",
    "from_json_export_selector",
    "ExportSelectorNamed",
    "ExportSelectorDefault",
    "ExportSelectorNamespace",
    "StarExport",
    "encode_star_export",
    "decode_star_export",
    "to_json_star_export",
    "from_json_star_export",
]
