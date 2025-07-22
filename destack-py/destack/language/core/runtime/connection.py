from collections.abc import Sequence
from typing import TYPE_CHECKING

from ..builtin import Event, Node
from .session import Session

if TYPE_CHECKING:
    from destack.language import Graph, NodeReference

# nocheckin: turn QueryConnections into GraphConnections??


class GraphConnection[NodeT: "Node" = Node]:
    """
    A connection between two Graphs.
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
        raise NotImplementedError

    async def commit(self, events: Sequence["Event"]) -> Sequence["Event"]:
        raise NotImplementedError

    async def close(self) -> None:
        raise NotImplementedError
