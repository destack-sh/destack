from __future__ import annotations

import enum
import hashlib
from dataclasses import dataclass, field
from itertools import chain
from typing import Callable, Iterator, Optional
from uuid import UUID

from bench.language import ModuleIndex
from bench.language.type import InterpSymbol, Record, TypeNode
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

    def __getitem__(self, id: UUID) -> int:
        return self.revisions[id]

    def map_node(self, raw_node: RawNode) -> TrackedNode:
        return TrackedNode(
            type=raw_node.type,
            id=raw_node.id,
            revision=self.get(raw_node.id),
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

    def stable_hash(self) -> str:
        """Stable hash of nodes (sorted)"""
        sorted_nodes = sorted(self.nodes.values(), key=lambda n: n.id)
        sorted_nodes_str = ",".join(
            f"{n.type}:{n.id}:{n.revision}:{n.parent_id}" for n in sorted_nodes
        )
        stable_hash = hashlib.sha256(sorted_nodes_str.encode("utf-8")).hexdigest()
        return stable_hash

    def __str__(self):
        return str(len(self.nodes))

    def __repr__(self):
        return f"<TrackedTree {self}>"

    def __iter__(self) -> Iterator[TrackedNode]:
        return iter(self.nodes.values())


def tree_from_module(
    revmap: RevisionMap, idx: ModuleIndex, filter: Callable[[InterpSymbol], bool] = None
) -> TrackedTree:
    tree = TrackedTree(nodes={})
    for symbol in idx.symbols.values():
        if filter and not filter(symbol):
            continue
        track_interp_symbol(tree, symbol, filter=filter)
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


def track_interp_symbol(
    tree: TrackedTree,
    symbol: InterpSymbol,
    filter: Callable[[InterpSymbol | TypeNode], bool] = None,
) -> None:
    """
    Tracks the instruction tree for the symbol into the given tree.
    """
    if symbol.id in tree.nodes:
        return
    tree._instruction_tree = tree._instruction_tree or InstructionTree()
    map_instruction(symbol, tree._instruction_tree, filter=filter)

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


def tracked_tree_from_symbol(
    symbol: InterpSymbol,
    filter: Callable[[InterpSymbol | TypeNode], bool] = None,
) -> TrackedTree:
    tree = TrackedTree()
    track_interp_symbol(tree, symbol, filter=filter)
    return tree
