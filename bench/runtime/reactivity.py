from __future__ import annotations

import enum
from dataclasses import dataclass
from typing import Iterable, Optional
from uuid import UUID

from bench.language import ModuleIndex, SourceMapping
from bench.language.type import Code, Dataset, Expectation, InterpSymbol, Task, Type
from bench.language.wire import ModuleData


class TrackedNodeType(enum.Enum):
    MODULE = "module"
    FILE = "file"
    STATEMENT = "statement"
    # not tracking TypeNode / Record level
    # (yet, since they're folded into Statement revisions :SubSymbolRevisions)


@dataclass
class RawNode:
    """Un-versioned tracked node."""

    type: TrackedNodeType
    id: UUID


@dataclass
class RawMapping:
    """Un-versioned tracked mapping."""

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
            # only up to statement-level for now since there are no :SubSymbolRevisions
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
    nodes: dict[UUID, TrackedNode]


def tree_from_mappings(mappings: list[SourceMapping]) -> TrackedTree:
    tree = TrackedTree(nodes={})
    for mapping in mappings:
        # we only care about source nodes since they are the dependencies
        if mapping.source_id not in tree.nodes:
            tree.nodes[mapping.source_id] = TrackedNode(
                type=TrackedNodeType.STATEMENT,
                id=mapping.source_id,
                revision=mapping.source_revision,
            )
    return tree


def tree_from_module(revmap: RevisionMap, idx: ModuleIndex) -> TrackedTree:
    tree = TrackedTree(nodes={})
    for symbol in idx.symbols.values():
        tree.nodes[symbol.id] = TrackedNode(
            type=TrackedNodeType.STATEMENT,
            id=symbol.id,
            revision=revmap.get(symbol.id),
            parent_id=symbol.source.parent_id,
            reference_id=symbol.source.reference_id,
            order_key=symbol.source.order_key,
        )
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


def walk_interp_symbol(
    symbol: InterpSymbol, path: list[InterpSymbol] = None
) -> Iterable[InterpSymbol]:
    """
    Walks all referenced symbols in an InterpSymbol.

    Note that this is not the same as walking parent/child relationships, since not all
    child statements are meaningful 'child' symbols.
    Note also that doesn't touch type_nodes/records individually because of missing :SubSymbolRevisions

    TODO @Cleanup: it seems easy to forget adding new symbol references here
    """

    # path breaks cycles (which are allowed)
    if path is not None and symbol in path:
        return
    elif path is None:
        path = []
    path = path + [symbol]

    yield symbol

    # context symbols
    for symbol in symbol.context.values():
        yield from walk_interp_symbol(symbol, path)

    # if this is a reference, walk the referenced symbol (can only be definition for now)
    if symbol.reference is not None and symbol.reference != symbol:
        yield from walk_interp_symbol(symbol.reference, path)

    # walk the type if it's a typed symbol
    if isinstance(symbol, (Task, Code, Dataset)):
        yield from walk_interp_symbol(symbol.type, path)

    # walk type nodes for Types
    if isinstance(symbol, Type):
        for type_node in symbol.walk():
            if isinstance(type_node, InterpSymbol):
                yield from walk_interp_symbol(type_node, path)

    # walk expectations
    if isinstance(symbol, (Task, Expectation, Type)):
        for expectation in symbol.expectations:
            yield from walk_interp_symbol(expectation, path)

    # walk task steps & implementation
    if isinstance(symbol, Task):
        for step in symbol.steps:
            yield from walk_interp_symbol(step, path)
        if symbol.implementation is not None:
            yield from walk_interp_symbol(symbol.implementation, path)
