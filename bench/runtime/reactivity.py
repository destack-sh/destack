from __future__ import annotations

import enum
from dataclasses import dataclass
from typing import Iterable, Optional
from uuid import UUID

from bench.language import ModuleIndex, SourceMapping
from bench.language.type import Code, Dataset, InterpSymbol, Task, Type
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
    root: TrackedNode
    objects_by_id: dict[UUID, list[TrackedNode]]


def tree_from_mappings(mappings: list[SourceMapping]) -> TrackedTree:
    raise NotImplementedError


def tree_from_module(module: ModuleIndex) -> TrackedTree:
    raise NotImplementedError


def diff_trees(old: TrackedTree, new: TrackedTree) -> list[TrackedNode]:
    raise NotImplementedError


@dataclass
class Reaction:
    pass


def react_to_diff(
    new: TrackedTree, diff: list[TrackedNode], barriers: list[ReactivityBarrier]
) -> list[Reaction]:
    raise NotImplementedError


@dataclass
class ReactivityBarrier:
    blocked_id: UUID


def walk_interp_symbol(
    symbol: InterpSymbol, path: list[InterpSymbol] = None
) -> Iterable[InterpSymbol]:
    """
    Walks all referenced symbols in an InterpSymbol.

    Note that this is not the same as walking parent/child relationships, since not all
    child statements are meaningful symbols.
    Note also that doesn't touch type_nodes/records individually because of missing :SubSymbolRevisions
    """
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
