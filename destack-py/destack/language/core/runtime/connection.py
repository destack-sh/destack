import asyncio
from collections.abc import Mapping, Sequence
from typing import TYPE_CHECKING, Any, Optional

from destack.utils.uuid import UUID

from ..builtin import Node, Trait
from ..common import Query, QueryResult, QueryResultBase, QueryType
from .session import Session

if TYPE_CHECKING:
    from destack.language import Store, Value


class QueryContainer[NodeT: "Trait | Node" = Node]:
    """
    A container for some (part of a) QueryResult.
    """

    __slots__ = ("connection", "discriminator", "nodes", "query", "result", "subcontainers", "type")

    def __init__(
        self,
        connection: "QueryConnection",
        type: QueryType,
        query: Query,
        result: QueryResultBase | None,
        discriminator: "Value | None" = None,
    ):
        self.connection: QueryConnection = connection
        self.type: QueryType = type
        self.query: Query = query
        self.result: QueryResultBase | None = result
        self.nodes: list[NodeT] = []  # type: ignore (no idea why pyright freaks out sometimes)
        self.discriminator: Any | None = discriminator
        self.subcontainers: list[QueryContainer] = []

    def __str__(self) -> str:
        content_parts: list[str] = [f"query={self.query!r}"]
        if self.result is not None:
            content_parts.append(f"result={self.result!r}")
        return ", ".join(content_parts)

    def __repr__(self) -> str:
        return f"<QueryContainer {self!s}>"

    def _add_result(self, result: QueryResultBase, query: Query) -> None:
        """Add a QueryResult to the connection (recursively)."""
        from ..common import unpack_value

        session = self.connection.session
        supergraph = session.supergraph
        graph = self.connection.graph

        # nodes
        for node_value in result.nodes:
            node = unpack_value(
                node_value.value,
                node_value.type,
                _session=session,
                _supergraph=supergraph,
                _graph=graph,
                _connection=self.connection,
            )
            assert isinstance(node, Node), f"expected Node, got {node!r} in {self!r}"
            self.nodes.append(node)  # type: ignore

        if isinstance(result, QueryResult):
            # subgroups
            for group in result.groups:
                if query.type == QueryType.GROUPED_NODE:
                    subtype = QueryType.NODE
                elif query.type == QueryType.GROUPED_SCALAR:
                    subtype = QueryType.SCALAR
                else:
                    raise ValueError(f"unexpected query type: {query.type!r}")
                subcontainer = QueryContainer(
                    self.connection, subtype, query, group, group.discriminator
                )
                self.subcontainers.append(subcontainer)
                subcontainer._add_result(group, query)
            # subqueries
            for i, subresult in enumerate(result.subresults):
                subquery = query.subqueries[i]
                subcontainer = QueryContainer(self.connection, subquery.type, subquery, subresult)
                self.subcontainers.append(subcontainer)
                subcontainer._add_result(subresult, subquery)

    def to_one_or_none(self) -> Optional[NodeT]:
        """Get the main Node (if any)."""
        assert self.type == QueryType.NODE, f"not a node Query: {self.query!r}"
        assert self.result is not None, f"no result for {self!r}"
        assert len(self.nodes) <= 1, (
            f"expected 0-1 root, got {len(self.nodes)} in {self!r}: {self.nodes!r}"
        )
        return self.nodes[0] if self.nodes else None

    def to_one(self) -> NodeT:
        """Get the main Node (error if none)."""
        assert self.type == QueryType.NODE, f"not a node query: {self.query!r}"
        assert self.result is not None, f"no result for {self!r}"
        assert len(self.nodes) == 1, (
            f"expected 1 root, got {len(self.nodes)} in {self!r}: {self.nodes!r}"
        )
        return self.nodes[0]

    def to_list(self) -> Sequence[NodeT]:
        """Get the list of main Nodes."""
        assert self.type == QueryType.NODE, f"not a node Query: {self.query!r}"
        assert self.result is not None, f"no result for {self!r}"
        return self.nodes

    def to_count(self) -> int:
        """Get the count."""
        assert self.type == QueryType.SCALAR, f"not a scalar Query: {self.query!r}"
        assert self.result is not None, f"no result for {self!r}"
        assert self.result.count is not None, f"no count in {self!r}"
        return self.result.count

    def to_exists(self) -> bool:
        """Get whether any results exist."""
        assert self.type == QueryType.SCALAR, f"not a scalar Query: {self.query!r}"
        assert self.result is not None, f"no result for {self!r}"
        assert self.result.exists is not None, f"no exists in {self!r}"
        return self.result.exists

    def to_scalar(self) -> Any:
        """Get the scalar value."""
        assert self.type == QueryType.SCALAR, f"not a scalar Query: {self.query!r}"
        assert self.result is not None, f"no result for {self!r}"
        assert self.result.scalar is not None, f"no scalar in {self!r}"
        return self.result.scalar.unpack()

    def to_scalar_by_group(self) -> Mapping[Any, Any]:
        """Get the scalar value by group."""
        assert self.type == QueryType.GROUPED_SCALAR, f"not a grouped scalar Query: {self.query!r}"
        assert isinstance(self.result, QueryResult), f"no group result for {self!r}"
        scalar_by_group: dict[Any, Any] = {}
        for subcontainer in self.subcontainers:
            if subcontainer.discriminator is None:
                continue
            discriminator = subcontainer.discriminator.unpack()
            scalar_by_group[discriminator] = subcontainer.to_scalar()
        return scalar_by_group

    def to_list_by_group(self) -> Mapping[Any, Sequence[Node]]:
        """Get the list of main Nodes by group."""
        assert self.type == QueryType.GROUPED_NODE, f"not a grouped node Query: {self.query!r}"
        assert isinstance(self.result, QueryResult), f"no group result for {self!r}"
        list_by_group: dict[Any, Sequence[Node]] = {}
        for subcontainer in self.subcontainers:
            if subcontainer.discriminator is None:
                continue
            discriminator = subcontainer.discriminator.unpack()
            list_by_group[discriminator] = subcontainer.to_list()
        return list_by_group

    def get(self, key: str | UUID) -> "QueryContainer":
        """Get a subresult by name or id."""
        for subcontainer in self.subcontainers:
            if subcontainer.query.name == key or subcontainer.query.id == key:
                return subcontainer
        raise KeyError(f"no subresult for {key!r} in {self!r}")


class QueryConnection[NodeT: "Trait | Node"](QueryContainer[NodeT]):  # type: ignore (pyright??)
    """
    A connection to a Query and its result.
    """

    __slots__ = (
        "graph",
        "lock",
        "nodes",
        "query",
        "result",
        "session",
        "store",
    )

    def __init__(self, query: Query, store: "Store", session: "Session"):
        super().__init__(self, query.type, query, None)

        from .graph import Graph, PolyGraph

        self.store: Store = store
        self.session: Session = session
        self.lock = asyncio.Lock()
        self.graph: Graph = PolyGraph(session.supergraph)

    def __repr__(self) -> str:
        return f"<QueryConnection {self!s}>"

    async def execute(self) -> None:
        """Execute the Query."""
        async with self.lock:
            self.result = await self.store.query(self.query)
            self._add_result(self.result, self.query)

    def close(self) -> None:
        """Close the QueryConnection."""
        raise NotImplementedError

    async def wait_closed(self) -> None:
        """Wait for the QueryConnection to be closed."""
        raise NotImplementedError
