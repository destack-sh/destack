from __future__ import annotations

import enum
from dataclasses import dataclass, field
from itertools import chain
from typing import Iterator, Optional
from uuid import UUID

from bench.language import ModuleIndex, SourceMapping
from bench.language.type import (
    Code,
    Dataset,
    Expectation,
    InterpSymbol,
    SourceMappingType,
    Task,
    Type,
)
from bench.language.wire import ModuleData


class TrackedNodeType(enum.Enum):
    MODULE = "module"
    FILE = "file"
    STATEMENT = "statement"
    RECORD = "record"
    TYPE_NODE = "type_node"


@dataclass
class RawNode:
    """Un-versioned tracked node."""

    type: TrackedNodeType
    id: UUID


@dataclass
class RawMapping:
    """Un-versioned tracked mapping."""

    type: TrackedNodeType
    source_id: UUID
    target_id: Optional[UUID]


@dataclass
class RevisionMap:
    """A simple lookup for the revisions used by node id in a module."""

    module_id: UUID
    revisions: dict[UUID, int]

    def get(self, id: UUID) -> int:
        return self.revisions.get(id, 0)

    def map_node(self, raw_node: RawNode) -> TrackedNode:
        return TrackedNode(
            type=raw_node.type,
            id=raw_node.id,
            revision=self.get(raw_node.id),
        )

    def map_mapping(self, raw_mapping: RawMapping) -> SourceMapping:
        return SourceMapping(
            type=SourceMappingType(raw_mapping.type.value),
            source_id=raw_mapping.source_id,
            source_revision=self.get(raw_mapping.source_id),
            target_id=raw_mapping.target_id,
            target_revision=self.get(raw_mapping.target_id) if raw_mapping.target_id else None,
        )

    @staticmethod
    def from_module(module: ModuleData) -> RevisionMap:
        revisions = {}
        for file in module.files:
            revisions[file.id] = file.revision
            for statement in file.statements:
                revisions[statement.id] = statement.revision
                for subsymbol in chain(statement.records or [], statement.type_nodes or []):
                    revisions[subsymbol.id] = subsymbol.revision
        return RevisionMap(module_id=module.id, revisions=revisions)


@dataclass
class TrackedNode:
    type: TrackedNodeType
    id: UUID
    revision: int
    parent_id: Optional[UUID] = None
    reference_id: Optional[UUID] = None
    order_key: Optional[str] = None


@dataclass
class TrackedTree:
    nodes: dict[UUID, TrackedNode] = field(default_factory=dict)

    def __str__(self):
        return str(len(self.nodes))

    def __repr__(self):
        return f"<TrackedTree {self}>"

    def __iter__(self) -> Iterator[TrackedNode]:
        return iter(self.nodes.values())


def tree_from_mappings(mappings: list[SourceMapping]) -> TrackedTree:
    tree = TrackedTree(nodes={})
    for mapping in mappings:
        # we only care about source nodes since they are the dependencies
        if mapping.source_id not in tree.nodes:
            tree.nodes[mapping.source_id] = TrackedNode(
                type=TrackedNodeType(mapping.type),
                id=mapping.source_id,
                revision=mapping.source_revision,
            )
    return tree


def tree_from_module(revmap: RevisionMap, idx: ModuleIndex) -> TrackedTree:
    tree = TrackedTree(nodes={})
    for symbol in idx.symbols.values():
        track_interp_symbol(tree, symbol)
    # assign revisions from revmap
    for node in tree.nodes.values():
        node.revision = revmap.get(node.id)
    return tree


def diff_trees(old: TrackedTree, new: TrackedTree) -> list[TrackedNode]:
    """
    Semantic difference between an old dependency tree and a new one (not symmetric).

    A node is considered different if
     1) it is present in the old tree and its revision changed
     2) if it was present in the old tree and is no longer present in the new tree
     3) if it is new and has an old tree node as a parent.

    Reference & order keys are not considered here because
     they factor into the tree & its revisions (respectively).
    """
    for node in old.nodes.values():
        if node.id not in new.nodes:
            yield node
        elif node.revision != new.nodes[node.id].revision:
            yield node
    for node in new.nodes.values():
        if node.id not in old.nodes and node.parent_id is not None and node.parent_id in old.nodes:
            yield node


def track_interp_symbol(tree: TrackedTree, symbol: InterpSymbol) -> None:
    """
    Walks all referenced symbols and subsymbols and adds them to the tree.
    If nodes are already present, they are skipped.
    """
    if symbol.id in tree.nodes and tree.nodes[symbol.id].type == TrackedNodeType.STATEMENT:
        # we can overwrite the none if it's not a statement
        return
    tree.nodes[symbol.id] = TrackedNode(
        type=TrackedNodeType.STATEMENT,
        id=symbol.id,
        revision=1,
        parent_id=symbol.source.parent_id,
    )

    # track record subsymbols
    if isinstance(symbol, Dataset):
        for record in symbol.records:
            tree.nodes[record.id] = TrackedNode(
                type=TrackedNodeType.RECORD,
                revision=1,
                id=record.id,
                parent_id=symbol.id,
                order_key=record.order_key,
            )

    # track types and their subsymbols
    if isinstance(symbol, (Task, Code, Type, Dataset)):
        if isinstance(symbol, Type):
            type = symbol
        else:
            type = symbol.type
        for type_node in type.walk():
            if isinstance(type_node, InterpSymbol):
                track_interp_symbol(tree, type_node)
            elif type_node.id not in tree.nodes:  # id may be re-used, prefer symbol node
                tree.nodes[type_node.id] = TrackedNode(
                    type=TrackedNodeType.TYPE_NODE, revision=1, id=type_node.id, parent_id=symbol.id
                )

    # context symbols
    for symbol in symbol.context.values():
        track_interp_symbol(tree, symbol)

    # if this is a reference, walk the referenced symbol (can only be definition for now)
    if symbol.reference is not None and symbol.reference != symbol:
        track_interp_symbol(tree, symbol.reference)

    # walk expectations
    if isinstance(symbol, (Task, Expectation, Type)):
        for expectation in symbol.expectations:
            track_interp_symbol(tree, expectation)

    # walk task steps & implementation
    if isinstance(symbol, Task):
        for step in symbol.steps:
            track_interp_symbol(tree, step)
        if symbol.implementation is not None:
            track_interp_symbol(tree, symbol.implementation)
