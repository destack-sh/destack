#
# Queries
#
import abc
import asyncio
from functools import wraps
from typing import (
    TYPE_CHECKING,
    Any,
    Collection,
    Generic,
    NamedTuple,
    Optional,
    TypeVar,
    Union,
    cast,
    override,
)
from uuid import UUID

import psycopg
import structlog
from opentelemetry import trace

from bench.language.const import (
    AggregationOp,
    BenchError,
    ConditionalOp,
    NodeType,
    ReadType,
    StoreConnectionType,
)
from bench.language.node import Node
from bench.language.setup import CHILD_NODE_TYPES
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
    RpcMetadata,
    SupervisorStub,
)
from bench.utils.func import bittuple, group_by
from bench.utils.tenacity import RETRY_GRPC, RetryOptions

if TYPE_CHECKING:
    from bench.language import Bench, Expression, Field, Property, Session, Store
    from bench.language.graph import NodeDataGraph
    from bench.language.query import QueryBuilder
    from bench.proto.monkey import _PatchedRpcMetadata
    from bench.sql.client import PgStoreConnection

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

NodeT = TypeVar("NodeT", bound=Node)
NodeDataT = TypeVar("NodeDataT", bound=AnyNodeData)
FieldOrProperty = Union[
    Field if TYPE_CHECKING else "Field", Property if TYPE_CHECKING else "Property", Any
]
NodeTypeOrClass = Union[NodeType, type[Node]]


class ConnectionError(BenchError):
    def __init__(
        self,
        engine: Union["StoreEngine", "StoreConnection", StoreConnectionType],
        query: Optional["QueryBuilder"] | Collection[EditData] = None,
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


class ConnectionFailedError(ConnectionError):
    """The connection is temporarily unavailable."""

    pass


class ConnectionIncapableError(ConnectionError):
    """The connection can't do this thing."""

    pass


class FetchOptions(NamedTuple):
    count: bool = False  # type: ignore
    lock_for_update: bool = False
    skip_locked: bool = False


class FetchResult(NamedTuple):
    nodes: Collection[AnyNodeData]
    roots: Collection[NodeReferenceData]
    total: int | None = None
    epoch: int | None = None
    connection_token: str | None = None


class FlushResult(NamedTuple):
    revisions: list[int]
    cascaded_edits: list[EditData]


class AggregateResult(NamedTuple):
    aggregation: AggregationData


def scope_includes(scope: GraphScope, other: GraphScope) -> bool:
    return (scope.bench_id is None or scope.bench_id == other.bench_id) and (
        scope.package_id is None or scope.package_id == other.package_id
    )


class StoreEngine(abc.ABC, Generic[NodeT, NodeDataT]):
    """A store engine provides Connections to query some Store-like thing."""

    def __init__(
        self,
        scope: GraphScope,
        node_types: bittuple[NodeType],
    ):
        self.scope = scope
        self.node_types = node_types

    def __repr__(self):
        self_str = str(self)
        if self_str:
            return f"<{self.__class__.__name__} {self_str}>"
        else:
            return f"<{self.__class__.__name__}>"

    @property
    def is_readonly(self) -> bool:
        return False

    @property
    def id(self) -> int | str | UUID:
        return hash(self)

    async def connect(self, session: "Session") -> "StoreConnection":
        """Opens the store engine for a session."""
        raise NotImplementedError


class StoreConnection(abc.ABC, Generic[NodeT, NodeDataT]):
    def __init__(self, session: "Session"):
        self.session = session

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
        raise ConnectionIncapableError(self, query, reason="aggregate unsupported")

    #
    # Transaction management
    # The methods closely mirror :GraphIO service methods for universal 2PCs.
    #

    async def flush(self, edits: list[EditData] | tuple[EditData, ...]) -> FlushResult:
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

    async def commit(self, edits: list[EditData] | tuple[EditData, ...]) -> FlushResult:
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

    def __init__(
        self,
        scope: GraphScope,
        node_types: bittuple[NodeType],
        remote: GraphIoStub | HostStub | SupervisorStub,
        rpc_metadata: RpcMetadata,
        retry: RetryOptions = RETRY_GRPC,
    ):
        super().__init__(scope, node_types)
        self.remote = remote
        self.rpc_metadata = rpc_metadata
        self.rpc_headers = cast("_PatchedRpcMetadata", rpc_metadata).to_headers()
        self.retry = retry

    def __str__(self):
        return f"scope={self.scope!r}, node_types=[{', '.join(t.bench_name for t in self.node_types)}], remote={self.remote.__class__.__name__}"

    @override
    async def connect(self, session: "Session") -> "RemoteConnection":
        return RemoteConnection(self, session)


class RemoteConnection(StoreConnection[NodeT, NodeDataT]):
    """A connection to a remote graph."""

    def __init__(self, engine: "RemoteEngine", session: "Session"):
        super().__init__(session)
        self.engine = engine

    def __str__(self):
        return f"engine={self.engine!r}, session={self.session}"

    @staticmethod
    def _remote_method(func):
        """Wraps an RPC connection function with tracing & retries."""
        method_name = func.__name__

        @wraps(func)
        @tracer.start_as_current_span(f"remote.{method_name}")
        async def wrapper(self: "RemoteConnection", *args, **kwargs):
            retry = self.engine.retry.new()
            while retry.should_retry:
                retry.on_attempt()
                try:
                    return await func(self, *args, **kwargs)
                except self.engine.retry.retry_on as e:
                    retry.on_error(e)
                    logger.error(f"remote.{method_name}.error", connection=self, exc_info=True)
                    if retry.should_retry:
                        await asyncio.sleep(retry.interval)
            error = retry.to_error()
            if isinstance(error, (OSError,)):
                raise ConnectionFailedError(
                    self, args[0] if args else None, reason=str(error)
                ) from error
            else:
                raise error

        return wrapper

    @override
    @_remote_method
    async def fetch(
        self, query: "QueryBuilder[NodeT, NodeDataT]", options: FetchOptions
    ) -> FetchResult:
        from bench.proto import wire, wiring

        trace.get_current_span().set_attribute("connection", repr(self))
        request = wire.SearchNodesRequest(
            node_type=wiring.pack_enum(NodeType, query._node_type),
            filter=wiring.pack_object_maybe(query._filter, ExpressionData),
            sort=(
                [wiring.pack_object(s, ExpressionData) for s in query._sort] if query._sort else []
            ),
            first=query._first,
            options=wiring.pack_object_maybe(query._options, ReadOptionsData),
            count=options.count,
            scope=self.engine.scope,
        )
        response = await self.engine.remote.search_nodes(request, metadata=self.engine.rpc_headers)
        return FetchResult(
            nodes=[wiring.unwrap_some_node(n) for n in response.nodes],
            roots=response.roots,
            total=response.total,
            epoch=response.epoch,
        )

    @override
    @_remote_method
    async def aggregate(self, query: "QueryBuilder[NodeT, NodeDataT]") -> AggregateResult:
        from bench.proto import wire, wiring

        trace.get_current_span().set_attribute("connection", repr(self))
        assert query._aggregation is not None, f"{query!r} has no aggregation"
        request = wire.AggregateNodesRequest(
            node_type=wiring.pack_enum(NodeType, query._node_type),
            filter=wiring.pack_object_maybe(query._filter, expect=ExpressionData),
            aggregation=cast(ExpressionData, query._aggregation._to_data()),
            scope=self.engine.scope,
        )
        response = await self.engine.remote.aggregate_nodes(
            request, metadata=self.engine.rpc_headers
        )
        return AggregateResult(response.aggregation)

    @override
    @_remote_method
    async def commit(self, edits: list[EditData] | tuple[EditData, ...]) -> FlushResult:
        from bench.proto import wire

        trace.get_current_span().set_attribute("connection", repr(self))
        edits = list(edits)
        request = wire.CommitTransactionRequest(
            id=str(self.session.tx.id),
            edits=edits,
            scope=self.engine.scope,
            context=self.session._get_session_context(),
        )
        response = await self.engine.remote.commit_transaction(
            request, metadata=self.engine.rpc_headers
        )
        return FlushResult(revisions=response.revisions, cascaded_edits=response.cascaded_edits)


class PostgresEngine(StoreEngine[NodeT, NodeDataT], Generic[NodeT, NodeDataT]):
    """An engine that uses postgres connections."""

    def __init__(
        self,
        store: "Store",
        bench: "Bench",
        scope: GraphScope,
        node_types: bittuple[NodeType],
    ):
        super().__init__(scope, node_types)
        self.store = store
        self.bench = bench

    def __str__(self):
        return f"scope={self.scope!r}, node_types=[{', '.join(t.bench_name for t in self.node_types)}], store={self.store!r}"

    @override
    async def connect(self, session: "Session") -> "PostgresConnection":
        from bench.sql.client import pg_store_connection

        conn = pg_store_connection(self.store)
        cur = await conn.open()
        return PostgresConnection(self, session, conn, cur)


class PostgresConnection(StoreConnection[NodeT, NodeDataT], Generic[NodeT, NodeDataT]):
    """A specific connection to a Postgres store."""

    def __init__(
        self,
        engine: "PostgresEngine",
        session: "Session",
        conn: "PgStoreConnection",
        cur: psycopg.AsyncCursor,
    ):
        super().__init__(session)
        self.engine = engine
        self.conn = conn
        self.cur = cur

    def __str__(self):
        return f"engine={self.engine!r}, session={self.session}"

    @staticmethod
    def _pg_method(func):
        """Wraps a Postgres connection function with tracing & error wrapping."""
        method_name = func.__name__

        @wraps(func)
        @tracer.start_as_current_span(f"pg.{method_name}")
        async def wrapper(self: "PostgresConnection", *args, **kwargs):
            from bench.sql.engine import SqlConnectionError

            try:
                return await func(self, *args, **kwargs)
            except SqlConnectionError as e:
                raise ConnectionFailedError(self, args[0] if args else None, reason=str(e)) from e

        return wrapper

    @override
    @_pg_method
    async def fetch(
        self, query: "QueryBuilder[NodeT, NodeDataT]", options: FetchOptions
    ) -> FetchResult:
        from bench.language import NodeReference, ReadOptions
        from bench.sql.engine import _pg_compile_conditional_maybe, pg_count, pg_search_node_graph

        assert query._node_cls.__table__ is not None, f"{query._node_cls} has no table"
        roots, graph = await pg_search_node_graph(
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
                where=_pg_compile_conditional_maybe(query._node_cls, query._filter),
            )
        else:
            total = None
        return FetchResult(
            roots=[NodeReference.from_node_data(r) for r in roots.nodes],
            nodes=list(graph.nodes),
            total=total,
        )

    @override
    @_pg_method
    async def aggregate(self, query: "QueryBuilder[NodeT, NodeDataT]") -> AggregateResult:
        from bench.sql.engine import _pg_compile_conditional_maybe, pg_count, pg_exists

        assert query._node_cls.__table__ is not None, f"{query._node_cls} has no table"
        assert query._aggregation is not None
        where = _pg_compile_conditional_maybe(query._node_cls, query._filter)
        if query._aggregation.op == AggregationOp.EXISTS:
            exists = await pg_exists(cur=self.cur, table=query._node_cls.__table__, where=where)
            return AggregateResult(AggregationData(exists=exists))
        elif query._aggregation.op == AggregationOp.COUNT:
            count = await pg_count(cur=self.cur, table=query._node_cls.__table__, where=where)
            return AggregateResult(AggregationData(count=count))
        else:
            raise ConnectionIncapableError(
                self, query, expression=query._aggregation, reason="unsupported"
            )

    @override
    @_pg_method
    async def flush(self, edits: list[EditData] | tuple[EditData, ...]) -> FlushResult:
        from bench.sql.engine import pg_edit

        new_revisions, cascaded_edits = await pg_edit(cur=self.cur, edits=edits)
        return FlushResult(revisions=new_revisions, cascaded_edits=cascaded_edits)

    @override
    @_pg_method
    async def commit(self, edits: list[EditData] | tuple[EditData, ...]) -> FlushResult:
        from bench.sql.engine import pg_edit

        new_revisions, cascaded_edits = await pg_edit(cur=self.cur, edits=edits)
        await self.cur.connection.commit()
        return FlushResult(revisions=new_revisions, cascaded_edits=cascaded_edits)

    @override
    @_pg_method
    async def cancel(self) -> None:
        if not self.cur.connection.broken:
            await self.cur.connection.rollback()

    @override
    @_pg_method
    async def close(self):
        if not self.cur.connection.broken:
            await self.cur.connection.rollback()
        await self.conn.close()


class InMemoryEngine(StoreEngine[NodeT, NodeDataT], Generic[NodeT, NodeDataT]):
    """A read-only engine that reads from an in-memory graph."""

    def __init__(
        self, scope: GraphScope, node_types: bittuple[NodeType], graph: "NodeDataGraph"
    ):
        super().__init__(scope, node_types)
        self.graph = graph

    def __str__(self):
        return f"scope={self.scope!r}, node_types=[{', '.join(t.bench_name for t in self.node_types)}], graph={self.graph!r}"

    @property
    def is_readonly(self) -> bool:
        return True

    async def connect(self, session: "Session"):
        return InMemoryConnection(self, session)


class InMemoryConnection(StoreConnection[NodeT, NodeDataT], Generic[NodeT, NodeDataT]):
    """A read-only connection to an in-memory graph."""

    def __init__(self, engine: "InMemoryEngine", session: "Session"):
        super().__init__(session)
        self.engine = engine

    def __str__(self):
        return f"engine={self.engine!r}, session={self.session}"

    @override
    @tracer.start_as_current_span("memory.fetch")
    async def fetch(
        self, query: "QueryBuilder[NodeT, NodeDataT]", options: FetchOptions
    ) -> FetchResult:
        from bench.language.expression import NodeReference
        from bench.language.graph import NodeDataGraph
        from bench.proto import wire

        loaded_graph = self.engine.graph
        visited_graph = NodeDataGraph()

        # get roots
        # NOTE :Incomplete: InMemoryConnection only supports trivial get by id for now
        assert query._filter is not None, f"{query!r} has no filter"
        assert query._filter.property is not None, f"{query!r} has no filter property"
        assert query._filter.property.name == "id", f"{query!r} doesn't filter on id"
        roots_ids: tuple[str, ...]
        if query._filter.op == ConditionalOp.EQUALS:
            roots_ids = (str(query._filter.value),) if query._filter.value else ()
        elif query._filter.op == ConditionalOp.IN:
            roots_ids = tuple(str(v) for v in query._filter.value)
        else:
            raise RuntimeError(f"unsupported filter op {query._filter!r} for {query!r}")
        roots: list[AnyNodeData] = []
        for root_id in roots_ids:
            if root_id in visited_graph:
                continue  # dedupe
            root = loaded_graph.get(root_id)
            if root is not None:
                roots.append(root)
                visited_graph.add(root)

        # select ancestors
        ancestor_types = query._options.ancestor_types if query._options else ()
        if len(ancestor_types) > 0:
            with tracer.start_as_current_span("memory.fetch.get_ancestors"):
                current_parents = roots
                while current_parents:
                    next_parents = []
                    for node in current_parents:
                        if (
                            node.parent_ptr is not None
                            and node.parent_ptr.id is not None
                            and node.parent_ptr.id not in visited_graph
                            and node.parent_ptr.type in ancestor_types
                        ):
                            parent = loaded_graph[node.parent_ptr.id]
                            visited_graph.add(parent)
                            next_parents.append(parent)
                    current_parents = next_parents

        # select descendants
        descendant_types = query._options.descendant_types if query._options else ()
        if len(descendant_types) > 0:
            with tracer.start_as_current_span("memory.fetch.get_descendants"):
                child_types_by_parent: dict[wire.NodeType, tuple[NodeType, ...]] = {
                    cast(wire.NodeType, node_type): tuple(
                        t for t in CHILD_NODE_TYPES[node_type] if t in descendant_types
                    )
                    for node_type in query.all_node_types
                }
                current_parents = roots
                while current_parents:
                    next_parents: list[AnyNodeData] = []
                    for node in current_parents:
                        child_types = child_types_by_parent[cast(wire.NodeType, node.metatype)]
                        for child_type in child_types:
                            children = loaded_graph.collect_descendants(node, child_type)
                            visited_graph.extend(children)
                            for child in children:
                                if loaded_graph.has_descendants(child):
                                    next_parents.append(child)
                    current_parents = next_parents

        return FetchResult(
            roots=[NodeReference.from_node_data(r) for r in roots],
            nodes=list(visited_graph.nodes),
            # not yet supported
            total=None,
        )


class SplitConnection(StoreConnection[NodeT, NodeDataT], Generic[NodeT, NodeDataT]):
    """A read-only connection that can split queries across store boundaries."""

    @override
    @tracer.start_as_current_span("split.fetch")
    async def fetch(
        self, query: "QueryBuilder[NodeT, NodeDataT]", options: FetchOptions
    ) -> FetchResult:
        from bench.language import C, QueryBuilder, ReadOptions
        from bench.language.graph import NodeDataGraph

        scope = (
            self.session.tx._get_scope_for_node(query._base)
            if query._base
            else self.session._default_scope
        )

        # first trim query to nucleus around core node type (use best match)
        initial_engine = self.session.tx._get_engine(
            scope, query._node_type, best_match=list(query.all_node_types), is_readonly=True
        )
        initial_query = query.trim_to(initial_engine.node_types)
        initial_connection = await self.session.tx._get_engine_connection(initial_engine)
        initial_result = await initial_connection.fetch(initial_query, options)
        if query._options is None:
            return initial_result  # nothing more to do

        # then fetch the rest of the graph up/down from the initial nucleus
        # NOTE :Robustness: we handle splits by assuming the node type split is a 'clean' horizontal
        #  line in the ancestry tree (like the local/global split).
        remaining_ancestors = [
            t for t in query._options.ancestor_types if t not in initial_engine.node_types
        ]
        remaining_descendants = [
            t for t in query._options.descendant_types if t not in initial_engine.node_types
        ]
        if not remaining_ancestors and not remaining_descendants:
            return initial_result  # nothing more to do
        combined_graph = NodeDataGraph(initial_result.nodes)
        actual_roots = combined_graph.find_roots()
        if not actual_roots:
            return initial_result  # nothing more to do

        if remaining_ancestors:
            actual_roots_parents = tuple(n.parent_ptr for n in actual_roots if n.parent_ptr)
            actual_roots_parents_by_type = group_by(actual_roots_parents, lambda n: n.type)
            ancestor_engine = self.session.tx._get_engine(
                scope, remaining_ancestors, is_readonly=True
            )
            ancestor_connection = await self.session.tx._get_engine_connection(ancestor_engine)

            for parent_type, parents in actual_roots_parents_by_type.items():
                if parent_type not in remaining_ancestors:
                    continue
                parents_ids = tuple(p.id for p in parents)
                ancestor_query = QueryBuilder(
                    read_type=ReadType.GET,
                    node_type=parent_type,
                    filter=C(ConditionalOp.IN, property=Node.id, value=parents_ids),
                    options=ReadOptions(ancestor_types=remaining_ancestors),
                )
                ancestor_result = await ancestor_connection.fetch(ancestor_query, options)
                combined_graph.extend(ancestor_result.nodes)

        if remaining_descendants:
            raise RuntimeError(f"descendants split: {remaining_descendants!r} for {query!r}")

        # combine (keeping the 'roots' from the initial result)
        combined_result = FetchResult(
            roots=initial_result.roots,
            nodes=combined_graph.nodes,
            total=initial_result.total,
            epoch=initial_result.epoch,
        )
        return combined_result

    @override
    @tracer.start_as_current_span("split.aggregate")
    async def aggregate(self, query: "QueryBuilder[NodeT, NodeDataT]") -> AggregateResult:
        # just forward to one engine, we don't support aggregating across engines
        scope = (
            self.session.tx._get_scope_for_node(query._base)
            if query._base
            else self.session._default_scope
        )
        engine = self.session.tx._get_engine(scope, query._node_type, is_readonly=True)
        connection = await self.session.tx._get_engine_connection(engine)
        return await connection.aggregate(query)
