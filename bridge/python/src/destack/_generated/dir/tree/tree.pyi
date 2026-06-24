# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.source.comment
import destack._generated.dir.source.parent
import destack._generated.dir.tree.argument
import destack._generated.dir.tree.block
import destack._generated.dir.tree.declaration
import destack._generated.dir.tree.declarator
import destack._generated.dir.tree.decorator
import destack._generated.dir.tree.dependency
import destack._generated.dir.tree.expression
import destack._generated.dir.tree.index
import destack._generated.dir.tree.match
import destack._generated.dir.tree.node
import destack._generated.dir.tree.pattern
import destack._generated.dir.tree.property
import destack._generated.dir.tree.sparse
import destack._generated.dir.tree.type
import destack._generated.source.file.model.module
import destack._generated.source.tree.index

@dataclass(frozen=True, slots=True)
class Tree:
    """Mutable DIR tree across a set of related source units."""

    # the module id of the tree
    module_id: destack._generated.source.file.model.module.ModuleId
    # the first global node id stored in this tree
    first_global_id: int
    # the next id to allocate
    next_global_id: int
    # dense local id and node type metadata by global node id
    node_index_by_node_id: Sequence[destack._generated.dir.tree.index.NodeIndexEntry]
    # source ranges and anchors keyed by parsed node id
    source_index: destack._generated.source.tree.index.SourceIndexData
    expressions: Sequence[destack._generated.dir.tree.expression.Expression]
    type_expressions: Sequence[destack._generated.dir.tree.type.TypeExpression]
    blocks: Sequence[destack._generated.dir.tree.block.Block]
    catches: Sequence[destack._generated.dir.tree.expression.Catch]
    declarations: Sequence[destack._generated.dir.tree.declaration.Declaration]
    declarators: Sequence[destack._generated.dir.tree.declarator.Declarator]
    properties: Sequence[destack._generated.dir.tree.property.Property]
    type_members: Sequence[destack._generated.dir.tree.type.TypeMember]
    type_mapped_parameters: Sequence[
        destack._generated.dir.tree.type.TypeMappedParameter
    ]
    members: Sequence[destack._generated.dir.tree.property.Member]
    enum_fields: Sequence[destack._generated.dir.tree.declaration.EnumField]
    where_clauses: Sequence[destack._generated.dir.tree.expression.WhereClause]
    dependency_items: Sequence[destack._generated.dir.tree.dependency.DependencyItem]
    generic_parameters: Sequence[destack._generated.dir.tree.argument.GenericParameter]
    parameters: Sequence[destack._generated.dir.tree.argument.Parameter]
    generic_arguments: Sequence[destack._generated.dir.tree.argument.GenericArgument]
    tuple_elements: Sequence[destack._generated.dir.tree.argument.TupleElement]
    arguments: Sequence[destack._generated.dir.tree.argument.Argument]
    match_cases: Sequence[destack._generated.dir.tree.match.MatchCase]
    patterns: Sequence[destack._generated.dir.tree.pattern.Pattern]
    pattern_fields: Sequence[destack._generated.dir.tree.pattern.PatternField]
    assign_patterns: Sequence[destack._generated.dir.tree.pattern.AssignPattern]
    assign_pattern_fields: Sequence[
        destack._generated.dir.tree.pattern.AssignPatternField
    ]
    comments: Sequence[destack._generated.dir.source.comment.Comment]
    decorators: Sequence[destack._generated.dir.tree.decorator.Decorator]
    # parent of every node, rebuilt when the tree is complete and updated in place
    parents: destack._generated.dir.source.parent.NodeParentIndex
    # how each derived node came to be
    origin_by_node_id: destack._generated.dir.tree.sparse.SparseNodeMap
    # the alias node id by replaced or derived node id
    alias_node_id_by_node_id: Mapping[int, int]
    # the decorators attached to nodes
    decorators_by_node_id: Mapping[
        int, Sequence[destack._generated.dir.tree.node.LocalNodeId]
    ]
    # the normalized documentation attached to nodes
    documentation_by_node_id: Mapping[
        int, destack._generated.dir.source.comment.Documentation
    ]
    # final source span overrides by node id
    source_span_by_node_id: destack._generated.dir.tree.sparse.SparseNodeMap
    # the detached node ids
    detached_node_ids: Sequence[int]

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

__all__ = [
    "Tree",
    "encode_tree",
    "decode_tree",
    "to_json_tree",
    "from_json_tree",
]
