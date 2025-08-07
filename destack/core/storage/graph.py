import abc
from collections.abc import Collection, Sequence
from datetime import datetime
from typing import TYPE_CHECKING, Optional, final

from ..builtin import UUID, Handle, HandleType, NodeType, declare_handle

if TYPE_CHECKING:
    from destack import Entity, Event, Snapshot

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
        raise NotImplementedError

    @abc.abstractmethod
    async def close(self) -> None:
        """Close the Graph."""
        raise NotImplementedError

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
        raise NotImplementedError

    async def insert(
        self,
        space_id: UUID,
        branch_id: UUID | None,
        snapshot_id: UUID | None,
        entities: "Sequence[Entity]",
    ) -> None:
        """Insert Entities into the Graph directly."""
        raise NotImplementedError

    @abc.abstractmethod
    def append(self, events: "Sequence[Event]") -> None:
        """Append Events to the Graph. EditEvents are reflected immediately."""
        raise NotImplementedError

    @abc.abstractmethod
    def restate(self, events: "Sequence[Event]") -> None:
        """Restate Events to the Graph. EditEvents are reflected immediately."""
        raise NotImplementedError

    @abc.abstractmethod
    async def prune(
        self,
        space_id: UUID,
        branch_id: UUID | None,
        snapshot_id: UUID | None,
    ) -> None:
        """Prune the Graph."""
        raise NotImplementedError

    @abc.abstractmethod
    async def commit(self) -> None:
        """Ensure Events/Entities are persisted in the Graph."""
        raise NotImplementedError

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
        raise NotImplementedError

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
        raise NotImplementedError

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
        raise NotImplementedError

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
        raise NotImplementedError

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
        raise NotImplementedError
