# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.symbol
import destack._generated.js.tree.annotation
import destack._generated.js.tree.argument
import destack._generated.js.tree.block
import destack._generated.js.tree.declaration
import destack._generated.js.tree.declarator
import destack._generated.js.tree.dependency
import destack._generated.js.tree.expression
import destack._generated.js.tree.node
import destack._generated.js.tree.pattern
import destack._generated.js.tree.property
import destack._generated.js.tree.statement
import destack._generated.js.tree.type
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class Tree:
    """Mutable AST tree for a single source unit. NOT THREAD-SAFE."""

    # the next id to allocate
    next_global_id: int
    # dense local id and node type metadata by node id
    node_index_by_node_id: Sequence[NodeIndexEntry]
    # the annotations attached to nodes
    annotations_by_node_id: Mapping[
        int, Sequence[destack._generated.js.tree.node.LocalNodeId]
    ]
    # the origin node for each JS node
    origin_by_node_id: Sequence[NodeOrigin | None]
    # the alias node id by DIR node id
    alias_node_id_by_dir_id: Mapping[int, int]
    # the alias node id by JS AST node id
    alias_node_id_by_node_id: Mapping[int, int]
    # the symbol identity by JS AST node id
    symbol_id_by_node_id: Sequence[ScriptSymbolId | None]
    blocks: Sequence[destack._generated.js.tree.block.Block]
    catch_clauses: Sequence[destack._generated.js.tree.block.CatchClause]
    statements: Sequence[destack._generated.js.tree.statement.Statement]
    expressions: Sequence[destack._generated.js.tree.expression.Expression]
    array_elements: Sequence[destack._generated.js.tree.expression.ArrayElement]
    declarations: Sequence[destack._generated.js.tree.declaration.Declaration]
    declarators: Sequence[destack._generated.js.tree.declarator.Declarator]
    properties: Sequence[destack._generated.js.tree.property.Property]
    members: Sequence[destack._generated.js.tree.property.Member]
    type_expressions: Sequence[destack._generated.js.tree.type.TypeExpression]
    tuple_elements: Sequence[destack._generated.js.tree.type.TupleElement]
    type_members: Sequence[destack._generated.js.tree.type.TypeMember]
    enum_fields: Sequence[destack._generated.js.tree.declaration.EnumField]
    dependency_items: Sequence[destack._generated.js.tree.dependency.DependencyItem]
    switch_cases: Sequence[destack._generated.js.tree.block.SwitchCase]
    generic_parameters: Sequence[destack._generated.js.tree.argument.GenericParameter]
    parameters: Sequence[destack._generated.js.tree.argument.Parameter]
    arguments: Sequence[destack._generated.js.tree.argument.Argument]
    patterns: Sequence[destack._generated.js.tree.pattern.Pattern]
    pattern_fields: Sequence[destack._generated.js.tree.pattern.PatternField]
    assign_patterns: Sequence[destack._generated.js.tree.pattern.AssignPattern]
    assign_pattern_fields: Sequence[
        destack._generated.js.tree.pattern.AssignPatternField
    ]
    annotations: Sequence[destack._generated.js.tree.annotation.Annotation]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Tree: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Tree: ...

def encode_tree(writer: BinaryWriter, value: Tree) -> None: ...
def decode_tree(reader: BinaryReader) -> Tree: ...
def to_json_tree(value: Tree) -> Json: ...
def from_json_tree(value: Json) -> Tree: ...

@dataclass(frozen=True, slots=True)
class NodeIndexEntry:
    """Dense metadata for one JS node id."""

    # the packed local id and node type
    packed: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> NodeIndexEntry: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> NodeIndexEntry: ...

def encode_node_index_entry(writer: BinaryWriter, value: NodeIndexEntry) -> None: ...
def decode_node_index_entry(reader: BinaryReader) -> NodeIndexEntry: ...
def to_json_node_index_entry(value: NodeIndexEntry) -> Json: ...
def from_json_node_index_entry(value: Json) -> NodeIndexEntry: ...

@dataclass(frozen=True, slots=True)
class NodeOrigin:
    """One DIR node that produced a JS node."""

    # the origin module
    module_id: destack._generated.source.file.model.module.ModuleId
    # the origin DIR node id
    node_id: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> NodeOrigin: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> NodeOrigin: ...

def encode_node_origin(writer: BinaryWriter, value: NodeOrigin) -> None: ...
def decode_node_origin(reader: BinaryReader) -> NodeOrigin: ...
def to_json_node_origin(value: NodeOrigin) -> Json: ...
def from_json_node_origin(value: Json) -> NodeOrigin: ...

@dataclass(frozen=True, slots=True)
class ScriptSymbolIdSource:
    """One symbol lowered directly from source DIR."""

    source: destack._generated.dir.symbol.symbol.GlobalSymbolId
    kind: typing.Literal["source"] = "source"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ScriptSymbolIdModuleDefault:
    """One generated default binding for one non-code script module."""

    module_default: destack._generated.source.file.model.module.ModuleId
    kind: typing.Literal["moduleDefault"] = "moduleDefault"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""One stable symbol identity in lowered script output."""
ScriptSymbolId: typing.TypeAlias = ScriptSymbolIdSource | ScriptSymbolIdModuleDefault

def encode_script_symbol_id(writer: BinaryWriter, value: ScriptSymbolId) -> None: ...
def decode_script_symbol_id(reader: BinaryReader) -> ScriptSymbolId: ...
def to_json_script_symbol_id(value: ScriptSymbolId) -> Json: ...
def from_json_script_symbol_id(value: Json) -> ScriptSymbolId: ...

__all__ = [
    "Tree",
    "encode_tree",
    "decode_tree",
    "to_json_tree",
    "from_json_tree",
    "NodeIndexEntry",
    "encode_node_index_entry",
    "decode_node_index_entry",
    "to_json_node_index_entry",
    "from_json_node_index_entry",
    "NodeOrigin",
    "encode_node_origin",
    "decode_node_origin",
    "to_json_node_origin",
    "from_json_node_origin",
    "ScriptSymbolId",
    "encode_script_symbol_id",
    "decode_script_symbol_id",
    "to_json_script_symbol_id",
    "from_json_script_symbol_id",
    "ScriptSymbolIdSource",
    "ScriptSymbolIdModuleDefault",
]
