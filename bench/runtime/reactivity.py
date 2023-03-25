from __future__ import annotations

import enum
from dataclasses import dataclass, field
from itertools import chain
from typing import Iterator, Optional
from uuid import UUID

from bench import language
from bench.language import GeneratedMapping, ModuleIndex
from bench.language.type import (
    Build,
    Code,
    Dataset,
    Expectation,
    GeneratedMappingType,
    InterpSymbol,
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


@dataclass(repr=False, slots=True)
class RawNode:
    """Un-versioned tracked node."""

    type: TrackedNodeType
    id: UUID


@dataclass(repr=False, slots=True)
class RawMapping:
    """Un-versioned tracked mapping."""

    type: TrackedNodeType
    source_id: UUID
    target_id: Optional[UUID]


@dataclass(repr=False, slots=True)
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

    def map_mapping(self, raw_mapping: RawMapping) -> GeneratedMapping:
        return GeneratedMapping(
            type=GeneratedMappingType(raw_mapping.type.value),
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


@dataclass(slots=True)
class TrackedNode:
    type: TrackedNodeType
    id: UUID
    revision: int
    parent_id: Optional[UUID] = None
    reference_id: Optional[UUID] = None
    order_key: Optional[str] = None

    def copy(self) -> TrackedNode:
        return TrackedNode(
            type=self.type,
            id=self.id,
            revision=self.revision,
            parent_id=self.parent_id,
            reference_id=self.reference_id,
            order_key=self.order_key,
        )


@dataclass(slots=True)
class TrackedTree:
    nodes: dict[UUID, TrackedNode] = field(default_factory=dict)

    def copy(self) -> TrackedTree:
        return TrackedTree(nodes={k: v.copy() for k, v in self.nodes.items()})

    def merge(self, other: TrackedTree):
        self.nodes.update(other.nodes)

    def __str__(self):
        return str(len(self.nodes))

    def __repr__(self):
        return f"<TrackedTree {self}>"

    def __iter__(self) -> Iterator[TrackedNode]:
        return iter(self.nodes.values())


def tree_from_mappings(mappings: list[GeneratedMapping]) -> TrackedTree:
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


def tree_from_module(revmap: RevisionMap, idx: ModuleIndex, exclude_generated: bool) -> TrackedTree:
    tree = TrackedTree(nodes={})
    for symbol in idx.symbols.values():
        if exclude_generated and symbol.source.generated:
            continue
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
        # we can overwrite the node if it's not a statement
        return
    tree.nodes[symbol.id] = TrackedNode(
        type=TrackedNodeType.STATEMENT,
        id=symbol.id,
        revision=1,
        parent_id=symbol.source.parent_id,
    )

    # track build dependencies
    if isinstance(symbol, Build):
        for symbol in chain(symbol.tasks, symbol.models):
            track_interp_symbol(tree, symbol)

    # track record subsymbols
    if isinstance(symbol, Dataset):
        for record in symbol.records:
            # this will have to change later, see :NaiveTreeTracking
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
                # this will have to change later, see :NaiveTreeTracking
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


def get_stale_symbols(revmap: RevisionMap, idx: language.ModuleIndex) -> list[language.Statement]:
    """Gets the stale generated or generator symbols in the given module"""

    # A generated/generator symbol is stale if
    #  1) one of its dependencies has changed
    #  2) one of its dependencies is affected by another change
    # These are because 1) checks for changes in known dependencies,
    # while 2) checks for new symbols that affect the dependencies.

    # (we exclude generated symbols here because we only need to consider source symbols)
    # TODO @Robustness: tree_from_module reactivity does not work with imports/redefs (incl. generated)
    #  TrackedTree assumes that each nodes dependencies are its revisioned children, and
    #  and any transient dependencies are tracked by walking descendants and adding them
    #  to the overall dependencies. The tree stores nodes by their source id, so with
    #  imports and redefs only the first instance of each descendant is tracked.
    #  This is not a problem with regular refs since you can't refer to refs.
    #  :NaiveTreeTracking
    new_tree = tree_from_module(revmap, idx, exclude_generated=True)

    stale_symbols = []
    for symbol in idx.symbols.values():
        if not symbol.is_generator:
            continue
        if isinstance(symbol, Build):
            source_mappings = symbol.source_mappings
        else:
            raise ValueError(f"unexpected generator symbol: {symbol}")

        # rebuild old tree for this generator
        old_tree = tree_from_mappings(source_mappings)
        diff_nodes = list(diff_trees(old_tree, new_tree))
        if not diff_nodes:
            # nothing relevant changed
            continue

        # mark generated statements as stale
        # also mark generator and the directly mapped source of the generated symbol
        # ideally we would also track which generator the symbol is stale in
        for source_mapping in source_mappings:
            if source_mapping.target_id is None:
                continue
            generated_target = idx.get_symbol_by_id(source_mapping.target_id)
            generated_source = idx.get_symbol_by_id(source_mapping.source_id)
            if generated_target is not None:
                # ignore if no target (was deleted or undirected dependency)
                stale_symbols.append(generated_target.source)
                if generated_source is not None:  # may have been deleted
                    stale_symbols.append(generated_source.source)
        stale_symbols.append(symbol.source)

    return stale_symbols
