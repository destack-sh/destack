import abc
from collections.abc import Collection, Sequence
from datetime import datetime
from typing import TYPE_CHECKING, Optional, final

from ..builtin import Handle, HandleType, NodeType, declare_handle
from ..utility import UUID

if TYPE_CHECKING:
    from destack.core import Entity, Event, Snapshot

type_ = type


@declare_handle(HandleType.GRAPH, is_abstract=True)
class Graph(Handle):
    """
    A Graph is a collection of Nodes from one or multiple Spaces (across time).
    """

    def __repr__(self):
        return f"<{self.__class__.__name__}>"

    #
    # Meta
    #

    @abc.abstractmethod
    async def open(self) -> None:
        """Open the Graph."""
        ...

    @abc.abstractmethod
    async def close(self) -> None:
        """Close the Graph."""
        ...

    #
    # Write
    #

    async def snapshot(
        self,
        space_id: UUID,
        branch_id: UUID | None,
        snapshot_id: UUID | None,
        epoch: int,
    ) -> "Snapshot":
        """Create a Snapshot."""
        ...

    async def insert(
        self,
        space_id: UUID,
        branch_id: UUID | None,
        snapshot_id: UUID | None,
        entities: "Sequence[Entity]",
    ) -> None:
        """Insert Entities into the Graph directly."""
        ...

    @abc.abstractmethod
    def append(self, events: "Sequence[Event]") -> None:
        """Append Events to the Graph. EditEvents are reflected immediately."""
        ...

    @abc.abstractmethod
    def restate(self, events: "Sequence[Event]") -> None:
        """Restate Events to the Graph. EditEvents are reflected immediately."""
        ...

    @abc.abstractmethod
    async def prune(
        self,
        space_id: UUID,
        branch_id: UUID | None,
        snapshot_id: UUID | None,
    ) -> None:
        """Prune the Graph."""
        ...

    @abc.abstractmethod
    async def commit(self) -> None:
        """Ensure Events/Entities are persisted in the Graph."""
        ...

    #
    # Read
    #

    @abc.abstractmethod
    def seek(
        self,
        space_id: UUID,
        branch_id: UUID | None,
        snapshot_id: UUID | None,
        type: NodeType | Collection[NodeType] | None = None,
        after: datetime | int | None = None,
        before: datetime | int | None = None,
    ) -> "Sequence[Event]":
        """Seek Events from the Graph."""
        ...

    @abc.abstractmethod
    def get(
        self,
        id: UUID,
        space_id: UUID,
        branch_id: UUID,
        snapshot_id: UUID,
        include_deleted: bool = False,
    ) -> Optional["Entity"]:
        """Gets an Entity by id."""
        ...

    @final
    def get_or_error(
        self,
        id: UUID,
        space_id: UUID,
        branch_id: UUID,
        snapshot_id: UUID,
        include_deleted: bool = False,
    ) -> "Entity":
        """Gets an Entity by id, raising an error if not found."""
        node = self.get(id, space_id, branch_id, snapshot_id, include_deleted)
        if node is None:
            raise KeyError(f"Entity {id!r} not found in {self!r}")
        return node

    @abc.abstractmethod
    def get_children(
        self,
        node: "Entity",
        space_id: UUID,
        branch_id: UUID,
        snapshot_id: UUID,
        type: NodeType | None = None,
        include_deleted: bool = False,
    ) -> Sequence["Entity"]:
        """Collect child Entities (one level down)."""
        ...

    @abc.abstractmethod
    def get_ancestors(
        self,
        node: "Entity",
        space_id: UUID,
        branch_id: UUID,
        snapshot_id: UUID,
        type: NodeType | None = None,
        include_deleted: bool = False,
    ) -> Sequence["Entity"]:
        """Gets the ancestors of this Entity (recursively up)."""
        ...

    @abc.abstractmethod
    def get_descendants(
        self,
        node: "Entity",
        space_id: UUID,
        branch_id: UUID,
        snapshot_id: UUID,
        type: NodeType | None = None,
        include_deleted: bool = False,
    ) -> Sequence["Entity"]:
        """Collect descendant Entities (recursively down)."""
        ...
