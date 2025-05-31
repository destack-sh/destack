from collections.abc import Collection
from dataclasses import dataclass
from typing import (
    TYPE_CHECKING,
    Callable,
)

import structlog
from fastuuid import UUID
from opentelemetry import trace

from bench.language import (
    Node,
    NodeType,
    bittuple,
)
from bench.proto import EditData

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass(slots=True)
class Commit[T: Node]:
    """
    A simplified diff of edited Nodes from a committed transaction for the Host system.
    In this view, archive/soft-delete => remove (and unarchive/restore => add).
    """

    edits: Collection[EditData]
    cascaded_edits: Collection[EditData]
    edited_types: bittuple[NodeType]
    added: tuple[T, ...]
    updated: tuple[T, ...]
    removed: tuple[T, ...]
    edited: tuple[T, ...]
    edited_by_id: dict[UUID, T]
    epoch: int

    def __str__(self):
        return f"added={self.added!r}, updated={self.updated!r}, removed={self.removed!r}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self!s}>"

    @property
    def is_empty(self) -> bool:
        return not self.added and not self.updated and not self.removed

    def has(
        self, filter: NodeType | tuple[NodeType, ...] | bittuple[NodeType] | Callable[[T], bool]
    ) -> bool:
        """Check if the diff contains any nodes matching the filter."""
        if callable(filter):
            return any(filter(node) for node in self.edited)
        elif isinstance(filter, NodeType):
            return filter in self.edited_types
        elif isinstance(filter, tuple):
            return any(t in self.edited_types for t in filter)
        else:
            return (self.edited_types.bits & filter.bits).any()

    def trim_to(
        self, filter: NodeType | tuple[NodeType, ...] | bittuple[NodeType] | Callable[[T], bool]
    ) -> "Commit[T]":
        """Trims the diff to only include nodes matching the filter."""
        if callable(filter):
            added = tuple(node for node in self.added if filter(node))
            updated = tuple(node for node in self.updated if filter(node))
            removed = tuple(node for node in self.removed if filter(node))
            edited = tuple(node for node in self.edited if filter(node))
            edited_by_id = {id: node for id, node in self.edited_by_id.items() if filter(node)}
            return Commit(
                edits=[e for e in self.edits if any(filter(n) for n in self.edited)],
                cascaded_edits=[
                    e for e in self.cascaded_edits if any(filter(n) for n in self.edited)
                ],
                edited_types=self.edited_types,  # can't trim bits since we don't know types
                added=added,
                updated=updated,
                removed=removed,
                edited=edited,
                edited_by_id=edited_by_id,
                epoch=self.epoch,
            )
        else:
            if isinstance(filter, NodeType):
                filter = bittuple(filter)
            elif isinstance(filter, tuple):
                filter = bittuple(*filter)
            added = tuple(node for node in self.added if node.metatype in filter)
            updated = tuple(node for node in self.updated if node.metatype in filter)
            removed = tuple(node for node in self.removed if node.metatype in filter)
            edited = tuple(node for node in self.edited if node.metatype in filter)
            edited_by_id = {
                id: node for id, node in self.edited_by_id.items() if node.metatype in filter
            }
            return Commit(
                edits=[e for e in self.edits if NodeType(e.node_ptr.node_type) in filter],
                cascaded_edits=[
                    e for e in self.cascaded_edits if NodeType(e.node_ptr.node_type) in filter
                ],
                edited_types=self.edited_types & filter,
                added=added,
                updated=updated,
                removed=removed,
                edited=edited,
                edited_by_id=edited_by_id,
                epoch=self.epoch,
            )
