# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string
import destack._generated.dir.tree.node
import destack._generated.source.file.model.loader
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class ModuleEdge:
    """One resolved module import edge."""

    # the DIR node that declared the dependency
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the static import specifier
    specifier: destack._generated.core.string.StringId
    # the module import relation
    relation: ModuleRelation
    # the loader override selected for the import
    loader: destack._generated.source.file.model.loader.Loader | None
    # the resolved module
    target: destack._generated.source.file.model.module.ModuleId | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ModuleEdge: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ModuleEdge: ...

def encode_module_edge(writer: BinaryWriter, value: ModuleEdge) -> None: ...
def decode_module_edge(reader: BinaryReader) -> ModuleEdge: ...
def to_json_module_edge(value: ModuleEdge) -> Json: ...
def from_json_module_edge(value: Json) -> ModuleEdge: ...

"""The relation declared by a resolved module import edge."""
ModuleRelation: typing.TypeAlias = typing.Literal["import"] | typing.Literal["reExport"]

def encode_module_relation(writer: BinaryWriter, value: ModuleRelation) -> None: ...
def decode_module_relation(reader: BinaryReader) -> ModuleRelation: ...
def to_json_module_relation(value: ModuleRelation) -> Json: ...
def from_json_module_relation(value: Json) -> ModuleRelation: ...

__all__ = [
    "ModuleEdge",
    "encode_module_edge",
    "decode_module_edge",
    "to_json_module_edge",
    "from_json_module_edge",
    "ModuleRelation",
    "encode_module_relation",
    "decode_module_relation",
    "to_json_module_relation",
    "from_json_module_relation",
]
