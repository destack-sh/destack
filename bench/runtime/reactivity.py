from __future__ import annotations

import enum
from dataclasses import dataclass, field
from itertools import chain
from typing import Callable, Iterator, Optional
from uuid import UUID

from bench import language
from bench.language import GeneratedMapping, ModuleIndex
from bench.language.type import Build, GeneratedMappingType, InterpSymbol, Record, TypeNode
from bench.language.wire import ModuleData
from bench.runtime.instruct import InstructionTree, map_instruction


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

    def copy(self) -> TrackedNode:
        return TrackedNode(
            type=self.type,
            id=self.id,
            revision=self.revision,
            parent_id=self.parent_id,
        )


@dataclass(slots=True)
class TrackedTree:
    nodes: dict[UUID, TrackedNode] = field(default_factory=dict)

    # private instruction tree for mapping instruction trees more efficiently
    # if it's mapped over multiple passes (e.g. different symbols)
    _instruction_tree: Optional[InstructionTree] = None

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


def tree_from_module(
    revmap: RevisionMap, idx: ModuleIndex, filter: Callable[[InterpSymbol], bool]
) -> TrackedTree:
    tree = TrackedTree(nodes={})
    for symbol in idx.symbols.values():
        if filter and not filter(symbol):
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
    Tracks the instruction tree for the symbol into the given tree.
    """
    if symbol.id in tree.nodes:
        return
    tree._instruction_tree = tree._instruction_tree or InstructionTree()
    map_instruction(symbol, tree._instruction_tree)

    for node, parent in tree._instruction_tree.walk_with_parent():
        if node.id not in tree.nodes:
            if isinstance(node.node, InterpSymbol):
                type = TrackedNodeType.STATEMENT
            elif isinstance(node.node, Record):
                type = TrackedNodeType.RECORD
            elif isinstance(node.node, TypeNode):
                type = TrackedNodeType.TYPE_NODE
            else:
                raise ValueError(f"unexpected instruction node {node}")
            tree.nodes[node.id] = TrackedNode(
                type=type,
                id=node.id,
                revision=1,
                parent_id=parent.id if parent else None,
            )


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
    #  To solve this, we'll probably invert nodes to track children (instead of parents),
    #  enabling multiple dependencies per trigger.
    #  :NaiveTreeTracking
    new_tree = tree_from_module(
        revmap, idx, filter=lambda s: not s.is_generated or isinstance(s, Build)
    )
    # (include builds since we need generated implicit builds to diff as well)

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
