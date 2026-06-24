# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string
import destack._generated.js.tree.key
import destack._generated.js.tree.node

"""The source form of one dependency item."""
DependencyForm: typing.TypeAlias = typing.Literal["type"] | typing.Literal["plain"]

def encode_dependency_form(writer: BinaryWriter, value: DependencyForm) -> None: ...
def decode_dependency_form(reader: BinaryReader) -> DependencyForm: ...
def to_json_dependency_form(value: DependencyForm) -> Json: ...
def from_json_dependency_form(value: Json) -> DependencyForm: ...

"""How one dependency item binds into the local module or export surface."""
DependencyBinding: typing.TypeAlias = (
    typing.Literal["named"] | typing.Literal["default"] | typing.Literal["namespace"]
)

def encode_dependency_binding(
    writer: BinaryWriter, value: DependencyBinding
) -> None: ...
def decode_dependency_binding(reader: BinaryReader) -> DependencyBinding: ...
def to_json_dependency_binding(value: DependencyBinding) -> Json: ...
def from_json_dependency_binding(value: Json) -> DependencyBinding: ...

@dataclass(frozen=True, slots=True)
class DependencyItem:
    """One dependency binding in an import or export clause."""

    # how the item binds
    binding: DependencyBinding
    # the source form of the item, when specified
    form: DependencyForm | None
    # the name of the item (like `foo` in `foo as bar`)
    name: destack._generated.js.tree.key.Name | None
    # the alias to use for the item (like `bar` in `foo as bar`)
    alias: destack._generated.core.string.StringId | None
    # the value of the item (for `export = foo` style exports)
    value: destack._generated.js.tree.node.LocalNodeId | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DependencyItem: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DependencyItem: ...

def encode_dependency_item(writer: BinaryWriter, value: DependencyItem) -> None: ...
def decode_dependency_item(reader: BinaryReader) -> DependencyItem: ...
def to_json_dependency_item(value: DependencyItem) -> Json: ...
def from_json_dependency_item(value: Json) -> DependencyItem: ...

__all__ = [
    "DependencyForm",
    "encode_dependency_form",
    "decode_dependency_form",
    "to_json_dependency_form",
    "from_json_dependency_form",
    "DependencyBinding",
    "encode_dependency_binding",
    "decode_dependency_binding",
    "to_json_dependency_binding",
    "from_json_dependency_binding",
    "DependencyItem",
    "encode_dependency_item",
    "decode_dependency_item",
    "to_json_dependency_item",
    "from_json_dependency_item",
]
