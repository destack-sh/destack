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


@dataclass
class TrackedNode:
    id: UUID
    revision: int
    parent_id: UUID
    reference_id: UUID
    order_key: Optional[str]
    type: TrackedNodeType  # opaque?


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


def react_to_diff(diff: list[TrackedNode], barriers: list[ReactivityBarrier]) -> list[Reaction]:
    raise NotImplementedError


@dataclass
class ReactivityBarrier:
    blocked_id: UUID
