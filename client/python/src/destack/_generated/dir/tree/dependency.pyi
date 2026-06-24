# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

from destack._impl.dir.tree.dependency import (
    DependencyItemImpl,
)

import destack._generated.core.string
import destack._generated.dir.tree.key
import destack._generated.dir.tree.node

"""The source form of one dependency declaration."""
DependencyForm: typing.TypeAlias = typing.Literal["plain"] | typing.Literal["type"]

def encode_dependency_form(writer: BinaryWriter, value: DependencyForm) -> None: ...
def decode_dependency_form(reader: BinaryReader) -> DependencyForm: ...
def to_json_dependency_form(value: DependencyForm) -> Json: ...
def from_json_dependency_form(value: Json) -> DependencyForm: ...

"""The export kind of a declaration or binding."""
ExportKind: typing.TypeAlias = typing.Literal["named"] | typing.Literal["default"]

def encode_export_kind(writer: BinaryWriter, value: ExportKind) -> None: ...
def decode_export_kind(reader: BinaryReader) -> ExportKind: ...
def to_json_export_kind(value: ExportKind) -> Json: ...
def from_json_export_kind(value: Json) -> ExportKind: ...

@dataclass(frozen=True, slots=True)
class DependencyItemBinding(DependencyItemImpl):
    """One valid dependency binding."""

    # how the item binds into the local module
    binding: DependencyBinding
    # the source form of the item, when specified
    form: DependencyForm | None
    # the name of the item (like `foo` in `foo as bar`, None if default)
    name: destack._generated.dir.tree.key.Name | None
    # the alias to use for the item (like `bar` in `foo as bar`)
    alias: destack._generated.core.string.StringId | None
    # the value of the item (for namespace exports)
    value: destack._generated.dir.tree.node.LocalNodeId | None
    kind: typing.Literal["binding"] = "binding"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DependencyItemError(DependencyItemImpl):
    """One malformed dependency item slot."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A dependency item imports or exports one binding from a target."""
DependencyItem: typing.TypeAlias = DependencyItemBinding | DependencyItemError

def encode_dependency_item(writer: BinaryWriter, value: DependencyItem) -> None: ...
def decode_dependency_item(reader: BinaryReader) -> DependencyItem: ...
def to_json_dependency_item(value: DependencyItem) -> Json: ...
def from_json_dependency_item(value: Json) -> DependencyItem: ...

"""How one dependency item binds into the local module."""
DependencyBinding: typing.TypeAlias = (
    typing.Literal["named"] | typing.Literal["default"] | typing.Literal["namespace"]
)

def encode_dependency_binding(
    writer: BinaryWriter, value: DependencyBinding
) -> None: ...
def decode_dependency_binding(reader: BinaryReader) -> DependencyBinding: ...
def to_json_dependency_binding(value: DependencyBinding) -> Json: ...
def from_json_dependency_binding(value: Json) -> DependencyBinding: ...

__all__ = [
    "DependencyForm",
    "encode_dependency_form",
    "decode_dependency_form",
    "to_json_dependency_form",
    "from_json_dependency_form",
    "ExportKind",
    "encode_export_kind",
    "decode_export_kind",
    "to_json_export_kind",
    "from_json_export_kind",
    "DependencyItem",
    "encode_dependency_item",
    "decode_dependency_item",
    "to_json_dependency_item",
    "from_json_dependency_item",
    "DependencyItemBinding",
    "DependencyItemError",
    "DependencyBinding",
    "encode_dependency_binding",
    "decode_dependency_binding",
    "to_json_dependency_binding",
    "from_json_dependency_binding",
]
