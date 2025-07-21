from typing import TYPE_CHECKING, cast

from ..builtin import Node
from ..common import Query
from .session import Session

if TYPE_CHECKING:
    from destack.language import Store

# nocheckin: turn QueryConnections into GraphConnections??


class GraphConnection[NodeT: "Node" = Node]:
    """
    A connection between two Graphs.
    """

    __slots__ = (
        "graph",
        "is_live",
        "nodes",
        "query",
        "result",
        "session",
        "store",
    )

    def __init__(
        self,
        *,
        query: Query,
        store: "Store",
        session: "Session",
        is_live: bool = False,
    ):
        super().__init__(
            connection=cast(GraphConnection, self),
            type=query.type,
            query=query,
            result=None,
        )

        from .graph import Graph

        self.store: Store = store
        self.session: Session = session
        self.graph: Graph = session.supergraph.create_entity_graph()
        self.is_live: bool = is_live

    def __repr__(self) -> str:
        return f"<GraphConnection query={self.query!r}>"

    async def execute(self) -> None:
        raise NotImplementedError

    def close(self) -> None:
        """Close the GraphConnection."""
        raise NotImplementedError

    async def wait_closed(self) -> None:
        """Wait for the GraphConnection to be closed."""
        raise NotImplementedError
