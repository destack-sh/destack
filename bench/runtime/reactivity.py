from __future__ import annotations

import enum
from dataclasses import dataclass
from typing import Optional
from uuid import UUID

from bench.language import ModuleIndex, SourceMapping


class TrackedObjectType(enum.Enum):
    MODULE = "module"
    FILE = "file"
    STATEMENT = "statement"


@dataclass
class TrackedObject:
    id: UUID
    revision: int
    parent_id: UUID
    order_key: Optional[str]
    type: str  # opaque?


@dataclass
class TrackedTree:
    root: TrackedObject
    objects_by_id: dict[UUID, list[TrackedObject]]


def tree_from_mappings(mappings: list[SourceMapping]) -> TrackedTree:
    raise NotImplementedError


def tree_from_module(module: ModuleIndex) -> TrackedTree:
    raise NotImplementedError


def diff_trees(old: TrackedTree, new: TrackedTree) -> list[TrackedObject]:
    raise NotImplementedError


@dataclass
class Reaction:
    pass


def react_to_diff(diff: list[TrackedObject], barriers: list[ReactivityBarrier]) -> list[Reaction]:
    raise NotImplementedError


@dataclass
class ReactivityBarrier:
    blocked_id: UUID
