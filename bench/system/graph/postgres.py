from functools import wraps
from typing import override

import psycopg
import structlog
from opentelemetry import trace

from bench.language import (
    Bench,
    CommitResultData,
    Connection,
    ConnectionOptions,
    Engine,
    EngineUnavailableError,
    FlushResultData,
    GetConnection,
    GetResultData,
    LegacyQuery,
    Node,
    NodeDataGraph,
    NodeReference,
    NodeType,
    QueryType,
    SearchConnection,
    SearchResultData,
    Session,
    Store,
    WritableConnector,
    bittuple,
    repr_enums,
    repr_scope,
)
from bench.proto import (
    EditData,
    GraphScopeData,
)
from bench.sql import (
    PostgresConnection,
    SqlContext,
    get_pg_pool,
    pg_graph_edit,
    pg_graph_get,
    pg_graph_search,
)

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class PostgresEngine(Engine):
    """An engine that talks directly to a Postgres store."""

    def __init__(
        self,
        name: str,
        store: "Store",
        bench: "Bench",
        scope: GraphScopeData,
        node_types: bittuple[NodeType],
        context: SqlContext,
    ):
        super().__init__(name, scope, node_types)
        self.store = store
        self.bench = bench
        self.context = context

    def __str__(self):
        return f"{self.name!r}, [scope={repr_scope(self.scope)}, node_types={repr_enums(self.node_types)}, store={self.store!r}]"

    @override
    async def connector(self, session: "Session") -> "PostgresConnector":
        pool = get_pg_pool(self.store)
        channel = PostgresConnector(self, session, connection=None)
        conn = await pool.acquire(owner=channel)
        channel._connection = conn
        logger.trace("postgres.channel.open", channel=channel, conn=conn)
        return channel

    @property
    def include_removed(self) -> bool:
        return True

    @property
    def is_readonly(self) -> bool:
        return False


def _pg_method(func):
    """Wraps a Postgres function with tracing & error wrapping."""
    method_name = func.__name__

    @wraps(func)
    @tracer.start_as_current_span(f"postgres.{method_name}")
    async def wrapper(self: "PostgresConnector", *args, **kwargs):
        from bench.sql import SqlConnectionError

        try:
            return await func(self, *args, **kwargs)
        except (SqlConnectionError, psycopg.OperationalError) as e:
            raise EngineUnavailableError(self, args[0] if args else None, reason=str(e)) from e

    return wrapper


class PostgresConnector(WritableConnector[PostgresEngine]):
    """A channel to a Postgres store (usually maps to a postgres connection)."""

    def __init__(
        self,
        engine: "PostgresEngine",
        session: "Session",
        connection: "PostgresConnection | None",
    ):
        super().__init__(engine, session)
        self._connection = connection

    def __str__(self):
        return f"engine={self.engine!r}, session={self.session}"

    @property
    def cur(self) -> psycopg.AsyncCursor:
        assert self._connection is not None, f"{self!r} has no connection"
        return self._connection.cursor

    @property
    def connection(self) -> "PostgresConnection":
        assert self._connection is not None, f"{self!r} has no connection"
        return self._connection

    @override
    def _get_connection_cls(
        self, query: "LegacyQuery", scope: GraphScopeData, options: ConnectionOptions
    ) -> type[Connection]:
        if query._type == QueryType.GET:
            return PostgresGetConnection
        elif query._type == QueryType.SEARCH:
            return PostgresSearchConnection
        else:
            raise RuntimeError(f"unsupported read type {query._type}")

    @override
    @_pg_method
    async def flush(self, edits: list[EditData] | tuple[EditData, ...]) -> FlushResultData:
        async with self.connection.lock:
            cascaded_edits = await pg_graph_edit(cur=self.cur, ctx=self.engine.context, edits=edits)
        return FlushResultData(cascaded_edits=cascaded_edits)

    @override
    @_pg_method
    async def commit(self, edits: list[EditData] | tuple[EditData, ...]) -> CommitResultData:
        cascaded_edits = await pg_graph_edit(cur=self.cur, ctx=self.engine.context, edits=edits)
        async with self.connection.lock:
            await self.connection.commit()
        return CommitResultData(cascaded_edits=cascaded_edits)

    @override
    async def reset(self):
        async with self.connection.lock:
            pool = self.connection.pool
            await pool.close()
            self._connection = await pool.acquire(owner=self)

    @override
    @_pg_method
    async def close(self):
        async with self.connection.lock:
            if not self.cur.connection.broken:
                await self.cur.connection.rollback()
            await self.connection.close()
        logger.trace("postgres.channel.close", channel=self, conn=self.connection)


class PostgresGetConnection[T: Node](GetConnection[PostgresConnector, T]):
    """Get from a Postgres channel."""

    @override
    async def _do_read(self, query: "LegacyQuery") -> GetResultData:
        assert query._roots, f"{query!r} has no roots"
        graph = NodeDataGraph(scope=self.scope, node_types=self.node_types)
        roots_ptr = [r._to_data() for r in query._roots]
        async with self.connector.connection.lock:
            _ = await pg_graph_get(
                cur=self.connector.cur,
                ctx=self.connector.engine.context,
                query=query,
                visited_graph=graph,
            )
        return GetResultData(graph=graph, roots_ptr=roots_ptr, epoch=None, connection_token=None)


class PostgresSearchConnection[T: Node](SearchConnection[PostgresConnector, T]):
    """Search a Postgres channel."""

    @override
    @_pg_method
    async def _do_read(self, query: "LegacyQuery") -> SearchResultData:
        async with self.connector.connection.lock:
            roots, graph, total = await pg_graph_search(
                cur=self.connector.cur,
                ctx=self.connector.engine.context,
                scope=self.connector.engine.scope,
                query=query,
                count=self.options.count,
            )
        return SearchResultData(
            graph=graph,
            roots=roots,
            roots_ptr=[NodeReference._ref_data_from_node_data(r) for r in roots],
            total=total,
            epoch=None,
            connection_token=None,
        )
