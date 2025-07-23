from collections.abc import Sequence
from typing import TYPE_CHECKING

from destack.utils.uuid import UUID

from ..builtin import Event, Node
from .session import Session

if TYPE_CHECKING:
    from destack.language import Graph, NodeReference


class GraphConnection[NodeT: "Node" = Node]:
    """
    A connection between a local and a remote Graph.
    """

    __slots__ = ("graph", "session", "space_ptr")

    def __init__(
        self,
        *,
        space_ptr: "NodeReference",
        graph: "Graph",
        session: "Session",
    ):
        self.space_ptr = space_ptr
        self.session = session
        self.graph = graph

    def __repr__(self) -> str:
        return f"<GraphConnection space={self.space_ptr!r}>"

    async def open(self) -> None:
        """Open the GraphConnection."""
        raise NotImplementedError

    async def pull(
        self,
        space_id: UUID,
        branch_id: UUID | None,
        snapshot_id: UUID | None,
    ) -> None:
        """Pull the relevant Entities and Events from the remote Graph into this Graph."""
        raise NotImplementedError

    async def push(self, events: Sequence["Event"]) -> Sequence["Event"]:
        """Push the Events to the remote Graph."""
        raise NotImplementedError

    async def close(self) -> None:
        """Close the GraphConnection."""
        raise NotImplementedError
