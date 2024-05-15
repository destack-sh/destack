#
# Queries
#
import abc
from typing import (
    TYPE_CHECKING,
    Any,
    ClassVar,
    Generic,
    NamedTuple,
    Optional,
    TypeVar,
    Union,
    cast,
)
from uuid import UUID

import psycopg
import structlog

from bench.language.const import AccessKind, AggregationOp, BenchError, NodeType, StoreEngineType
from bench.language.node import Node
from bench.proto.wire import (
    AggregationData,
    AnyNodeData,
    EditData,
    ExpressionData,
    GraphIoStub,
    GraphScope,
    HostStub,
    NodeReferenceData,
    ReadOptionsData,
    SupervisorStub,
)
from bench.utils.func import bytetuple
from bench.utils.tenacity import RetryOptions, retry

if TYPE_CHECKING:
    from bench.language import Expression, Field, Property, Session, Store
    from bench.language.query import QueryBuilder
    from bench.sql.client import _PgStoreConnection

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false

logger = structlog.get_logger(__name__)

NodeT = TypeVar("NodeT", bound=Node)
NodeDataT = TypeVar("NodeDataT", bound=AnyNodeData)
FieldOrProperty = Union[
    Field if TYPE_CHECKING else "Field", Property if TYPE_CHECKING else "Property", Any
]
NodeTypeOrClass = Union[NodeType, type[Node]]


class StoreEngineError(BenchError, ValueError):
    def __init__(
        self,
        engine: Union["StoreEngine", "StoreConnection", StoreEngineType],
        query: Optional["QueryBuilder"] = None,
        expression: Union["Expression", list["Expression"], None] = None,
        reason: str | None = None,
    ):
        if query is not None:
            action = repr(query)
        elif expression is not None:
            action = repr(expression)
        else:
            action = "<unknown action>"
        super().__init__(f"{engine} cannot {action}: {reason or '<unknown error>'}")
        self.engine = engine
        self.query = query
        self.expression = expression
        self.reason = reason


class ConnectionIncapableError(StoreEngineError):
    pass


class FetchOptions(NamedTuple):
    count: bool = False  # type: ignore
    lock_for_update: bool = False
    skip_locked: bool = False


class FetchResult(NamedTuple):
    nodes: list[AnyNodeData] | tuple[AnyNodeData, ...]
    roots: list[NodeReferenceData] | tuple[NodeReferenceData, ...]
    cursors: list[str] | tuple[str, ...]
    start_cursor: str | None
    total: int | None = None
    epoch: int | None = None


class AggregateResult(NamedTuple):
    aggregation: AggregationData


class StoreEngine(abc.ABC, Generic[NodeT, NodeDataT]):
    """A store engine provides Connections to query some Store-like thing."""

    type: ClassVar[StoreEngineType]

    def __init__(
        self,
        node_types: tuple[NodeType, ...] | bytetuple[NodeType],
    ):
        self.node_types = node_types

    def __repr__(self):
        self_str = str(self)
        if self_str:
            return f"<{self.__class__.__name__} {self_str} ({self.type.bench_name})>"
        else:
            return f"<{self.__class__.__name__} ({self.type.bench_name})>"

    @property
    def id(self) -> int | str | UUID:
        return hash(self)

    def supports(self, scope: GraphScope, node_type: NodeType, access_kind: AccessKind) -> bool:
        if node_type not in self.node_types:
            return False
        return True

    async def connect(self, session: "Session") -> "StoreConnection":
        """Opens the store engine for a session."""
        raise NotImplementedError


StoreEngineT = TypeVar("StoreEngineT", bound=StoreEngine)


class StoreConnection(abc.ABC, Generic[StoreEngineT, NodeT, NodeDataT]):
    def __init__(self, engine: "StoreEngineT", session: "Session"):
        self.engine = engine
        self.session = session

    @property
    def type(self) -> StoreEngineType:
        return self.engine.type

    def __str__(self):
        return f"session={self.session}"

    def __repr__(self):
        self_str = str(self)
        if self_str:
            return f"<{self.__class__.__name__} {self}>"
        else:
            return f"<{self.__class__.__name__}>"

    #
    # Read
    #

    async def fetch(
        self, query: "QueryBuilder[NodeT, NodeDataT]", options: FetchOptions
    ) -> FetchResult:
        """Read the nodes given the fetch query in the current transaction context (if any)."""
        raise ConnectionIncapableError(self, query, reason="fetch unsupported")

    async def aggregate(self, query: "QueryBuilder[NodeT, NodeDataT]") -> AggregateResult:
        """Read the nodes given the aggregate query in the current transaction context (if any)."""
        raise ConnectionIncapableError(self, query, reason="exists unsupported")

    #
    # Transaction management
    # The methods closely mirror :GraphIO service methods for universal 2PCs.
    #

    async def flush(
        self, edits: list[EditData] | tuple[EditData, ...]
    ) -> list[int] | tuple[int, ...] | None:
        """
        Flushes edits in the current transaction context. If not in a transaction, begins one.
        If this is a primary store, must return the accepted revisions for every edit (in order).
        """
        raise ConnectionIncapableError(self, reason="flush unsupported")

    async def complete(self) -> None:
        """Completes the current transaction context. No further operations are allowed."""
        raise ConnectionIncapableError(self, reason="complete unsupported")

    async def cancel(self) -> None:
        """Cancels the current transaction context. No further operations are allowed."""
        raise ConnectionIncapableError(self, reason="cancel unsupported")

    async def commit(
        self, edits: list[EditData] | tuple[EditData, ...]
    ) -> list[int] | tuple[int, ...] | None:
        """
        Commits the flushed pending and given edits in the current transaction context.
        If this is a primary store, must return the accepted revisions for every edit (in order).
        """
        raise ConnectionIncapableError(self, reason="commit unsupported")

    async def close(self):
        """Closes this connection to all further operations."""
        pass


class RemoteEngine(StoreEngine[NodeT, NodeDataT]):
    """An engine that proxies to a remote graph store."""

    type = StoreEngineType.REMOTE

    def __init__(
        self,
        default_scope: GraphScope,
        node_types: tuple[NodeType, ...] | bytetuple[NodeType],
        remote: GraphIoStub | HostStub | SupervisorStub,
        retry: RetryOptions = RetryOptions(max_attempts=1),
    ):
        super().__init__(node_types)
        self.default_scope = default_scope
        self.remote = remote
        self.retry = retry

    def __str__(self):
        return f"remote={self.remote}"

    async def connect(self, session: "Session") -> "RemoteConnection":
        return RemoteConnection(self, session)


class RemoteConnection(StoreConnection[RemoteEngine, NodeT, NodeDataT]):
    @retry(
        lambda self, *args, **kwargs: self.engine.retry,
        on_failure=lambda self, query, options, e: logger.error(
            "remote.fetch.error", connection=self, query=query, options=options, exc_info=e
        ),
    )
    async def fetch(
        self, query: "QueryBuilder[NodeT, NodeDataT]", options: FetchOptions
    ) -> FetchResult:
        from bench.proto import wire, wiring

        request = wire.SearchNodesRequest(
            node_type=wiring.pack_enum(NodeType, query._node_type),
            filter=wiring.pack_struct_maybe(query._filter, ExpressionData),
            sort=(
                [wiring.pack_struct(s, ExpressionData) for s in query._sort] if query._sort else []
            ),
            first=query._first,
            options=wiring.pack_struct_maybe(query._options, ReadOptionsData),
            count=options.count,
            scope=self.engine.default_scope,
        )
        response = await self.engine.remote.search_nodes(request)
        return FetchResult(
            nodes=[wiring.unwrap_some_node(n) for n in response.nodes],
            roots=response.roots,
            cursors=response.cursors,
            start_cursor=response.start_cursor,
            total=response.total,
            epoch=response.epoch,
        )

    @retry(
        lambda self, *args, **kwargs: self.engine.retry,
        on_failure=lambda self, query, e: logger.error(
            "remote.aggregate.error", connection=self, query=query, exc_info=e
        ),
    )
    async def aggregate(self, query: "QueryBuilder[NodeT, NodeDataT]") -> AggregateResult:
        from bench.proto import wire, wiring

        assert query._aggregation is not None, f"{query!r} has no aggregation"
        request = wire.AggregateNodesRequest(
            node_type=wiring.pack_enum(NodeType, query._node_type),
            filter=wiring.pack_struct_maybe(query._filter, expect=ExpressionData),
            aggregation=cast(ExpressionData, query._aggregation._to_data()),
            scope=self.engine.default_scope,
        )
        response = await self.engine.remote.aggregate_nodes(request)
        return AggregateResult(response.aggregation)

    @retry(
        lambda self, *args, **kwargs: self.engine.retry,
        on_failure=lambda self, edits, e: logger.error(
            "remote.commit.error", connection=self, edits=edits, exc_info=e
        ),
    )
    async def commit(self, edits: list[EditData] | tuple[EditData, ...]) -> list[int]:
        from bench.proto import wire

        edits = list(edits)
        request = wire.CommitTransactionRequest(
            id=str(self.session.tx.id), edits=edits, scope=self.engine.default_scope
        )
        response = await self.engine.remote.commit_transaction(request)
        return response.revisions


class PostgresEngine(StoreEngine[NodeT, NodeDataT], Generic[NodeT, NodeDataT]):
    type = StoreEngineType.POSTGRES

    def __init__(
        self,
        store: "Store",
        node_types: tuple[NodeType, ...] | bytetuple[NodeType],
    ):
        super().__init__(node_types)
        self.store = store

    def __str__(self):
        return f"store={self.store!r}"

    async def connect(self, session: "Session") -> "PostgresConnection":
        from bench.sql.client import get_pg_store_connection

        conn = await get_pg_store_connection(self.store)
        cur = await conn.open()
        return PostgresConnection(self, session, conn, cur)


class PostgresConnection(
    StoreConnection[PostgresEngine, NodeT, NodeDataT], Generic[NodeT, NodeDataT]
):
    def __init__(
        self,
        engine: "PostgresEngine",
        session: "Session",
        conn: "_PgStoreConnection",
        cur: psycopg.AsyncCursor,
    ):
        super().__init__(engine, session)
        self.conn = conn
        self.cur = cur

    async def close(self):
        await self.cur.connection.rollback()
        await self.conn.close()

    async def fetch(
        self, query: "QueryBuilder[NodeT, NodeDataT]", options: FetchOptions
    ) -> FetchResult:
        from bench.language import NodeReference, ReadOptions
        from bench.sql.engine import (
            compile_pg_conditional_maybe,
            pg_count,
            pg_search_nodes_data_graph,
        )

        assert query._node_cls.__table__ is not None, f"{query._node_cls} has no table"
        roots, graph = await pg_search_nodes_data_graph(
            cur=self.cur,
            node_type=query._node_type,
            options=query._options or ReadOptions(),
            filter=query._filter,
            sort=query._sort,
            first=query._first,
            skip=query._skip,
        )
        if options.count:
            total = await pg_count(
                cur=self.cur,
                table=query._node_cls.__table__,
                where=compile_pg_conditional_maybe(query._node_cls, query._filter),
            )
        else:
            total = None
        return FetchResult(
            roots=[NodeReference.from_node_data(r) for r in roots.nodes],
            nodes=list(graph.nodes),
            cursors=roots.cursors,
            start_cursor=roots.start_cursor,
            total=total,
        )

    async def aggregate(self, query: "QueryBuilder[NodeT, NodeDataT]") -> AggregateResult:
        from bench.sql.engine import compile_pg_conditional_maybe, pg_count, pg_exists

        assert query._node_cls.__table__ is not None, f"{query._node_cls} has no table"
        assert query._aggregation is not None
        where = compile_pg_conditional_maybe(query._node_cls, query._filter)
        if query._aggregation.op == AggregationOp.EXISTS:
            exists = await pg_exists(self.cur, query._node_cls.__table__, where=where)
            return AggregateResult(AggregationData(exists=exists))
        elif query._aggregation.op == AggregationOp.COUNT:
            count = await pg_count(self.cur, query._node_cls.__table__, where=where)
            return AggregateResult(AggregationData(count=count))
        else:
            raise ConnectionIncapableError(
                self, query, expression=query._aggregation, reason="unsupported"
            )

    async def flush(
        self, edits: list[EditData] | tuple[EditData, ...]
    ) -> list[int] | tuple[int, ...]:
        from bench.sql.engine import pg_write_edits

        new_revisions = await pg_write_edits(self.cur, edits)
        return new_revisions

    async def commit(
        self, edits: list[EditData] | tuple[EditData, ...]
    ) -> list[int] | tuple[int, ...]:
        from bench.sql.engine import pg_write_edits

        new_revisions = await pg_write_edits(self.cur, edits)
        await self.cur.connection.commit()
        return new_revisions

    async def cancel(self) -> None:
        await self.cur.connection.rollback()
