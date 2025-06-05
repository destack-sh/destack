import asyncio
from typing import TYPE_CHECKING, Optional, cast

from fastuuid import UUID

from .node import Node
from .query import Query, QueryResult
from .session import Session
from .trait import Trait

if TYPE_CHECKING:
    from bench.language import Store, Value


class QueryConnection[RootT: "Trait | Node"]:
    """
    A connection to a Query and its result.
    NOTE: 'root' refers to the root Query, not necessarily the root of the result Graph.
    """

    __slots__ = (
        "graph",
        "id",
        "is_live",
        "lock",
        "queries_by_id",
        "query",
        "result",
        "results_by_id",
        "roots",
        "session",
        "store",
    )

    def __init__(self, query: Query, store: "Store", session: "Session"):
        from .graph import Graph

        # meta
        self.id: UUID = query.id
        self.query: Query = query
        self.is_live = query.is_live
        self.store: Store = store
        self.session: Session = session
        self.lock = asyncio.Lock()

        # result
        self.result: QueryResult | None = None
        self.queries_by_id: dict[UUID, Query] = {}
        self.results_by_id: dict[UUID, QueryResult] = {}
        self.graph: Graph = Graph(supergraph=session.supergraph, connection=self)
        self.roots: list[RootT] = []

    def __str__(self) -> str:
        content_parts: list[str] = [
            f"id={self.id}",
            f"query={self.query!r}",
            f"is_live={self.is_live}",
        ]
        if self.result:
            content_parts.append(f"result={self.result!r}")
        return ", ".join(content_parts)

    def __repr__(self) -> str:
        return f"<QueryConnection {self!s}>"

    async def execute(self) -> None:
        """Execute the Query."""
        async with self.lock:
            self.result = await self.store.query(self.query)
            self._add_result(self.result, is_root=True)

    def _add_result(self, result: QueryResult, is_root: bool) -> None:
        """Add a QueryResult to the connection (recursively)."""
        from .value import unpack_value

        for node_value in result.nodes:
            node = unpack_value(
                node_value.value,
                node_value.type,
                _session=self.session,
                _supergraph=self.session.supergraph,
                _graph=self.graph,
                _connection=self,
            )
            assert isinstance(node, Node), f"expected Node, got {node!r} in {self!r}"
            if is_root:
                self.roots.append(cast(RootT, node))
        for subresult in result.subresults:
            self._add_result(subresult, is_root=False)

    def to_one_or_none(self) -> Optional[RootT]:
        """Get the root Node (if any)."""
        assert self.result is not None, f"no result for {self!r}"
        assert len(self.roots) <= 1, (
            f"expected 0-1 root, got {len(self.roots)} in {self!r}: {self.roots!r}"
        )
        return self.roots[0] if self.roots else None

    def to_one(self) -> RootT:
        """Get the root Node (error if none)."""
        assert self.result is not None, f"no result for {self!r}"
        assert len(self.roots) == 1, (
            f"expected 1 root, got {len(self.roots)} in {self!r}: {self.roots!r}"
        )
        return self.roots[0]

    def to_list(self) -> list[RootT]:
        """Get the list of roots."""
        assert self.result is not None, f"no result for {self!r}"
        assert len(self.roots) > 0, f"no roots in {self!r}"
        return self.roots

    def to_count(self) -> int:
        """Get the count."""
        assert self.result is not None, f"no result for {self!r}"
        assert self.result.count is not None, f"no count in {self!r}"
        return self.result.count

    def to_exists(self) -> bool:
        """Get whether any results exist."""
        assert self.result is not None, f"no result for {self!r}"
        assert self.result.exists is not None, f"no exists in {self!r}"
        return self.result.exists

    def to_scalar(self) -> "Value":
        """Get the scalar value."""
        assert self.result is not None, f"no result for {self!r}"
        assert self.result.scalar is not None, f"no scalar in {self!r}"
        return self.result.scalar

    def close(self) -> None:
        """Close the QueryConnection."""
        raise NotImplementedError

    async def wait_closed(self) -> None:
        """Wait for the QueryConnection to be closed."""
        raise NotImplementedError
