from __future__ import annotations

import enum
from dataclasses import dataclass
from typing import Optional
from uuid import UUID

from bench.language import ModuleIndex, SourceMapping


class TrackedNodeType(enum.Enum):
    MODULE = "module"
    FILE = "file"
    STATEMENT = "statement"
    # not tracking TypeNode / Record level
    # (yet, since they're folded into Statement revisions :SubSymbolRevisions)


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


@dataclass
class RevisionMap:
    revisions: dict[UUID, int]

    def get(self, id: UUID) -> int:
        return self.revisions.get(id, 0)
