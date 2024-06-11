import abc
import asyncio
from dataclasses import dataclass
from itertools import chain
from typing import Any, cast, final, override

import structlog
from opentelemetry import trace

from bench.language import Session, Subject
from bench.language.connection import FetchOptions
from bench.language.const import NodeType
from bench.language.graph import NodeDataGraph, NodeDataGraphLike, NodeGraphLike
from bench.language.query import QueryBuilder
from bench.proto.wire import (
    AggregationData,
    AnyNodeData,
    EditData,
    GraphScope,
    NodeReferenceData,
)
from bench.utils.dt import monons
from bench.utils.func import bittuple, generate_access_token
from bench.utils.utils import get_from_env

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

MAX_TIME_DRIFT_SECONDS = get_from_env("MAX_TIME_DRIFT_SECONDS", typ=int, default=60)


class Connection[ResultT: Any, UpdateT: Any](abc.ABC):
    """A (usually live) query connection to a (sub)graph."""

    def __init__(self, scope: GraphScope, query: QueryBuilder):
        self.scope = scope
        self.hash = query._stable_hash()
        self.token: str = generate_access_token(length=8)
        self.query = query
        self._subscribers: list[ConnectionSubscription[UpdateT]] = []
        self._created_at = monons()
        self._last_active_at = monons()
        self._result: ResultT | None = None

    @abc.abstractmethod
    def __result_str__(self, result: ResultT) -> str: ...

    @final
    def __str__(self):
        content_str = self.__result_str__(self._result) if self._result else "<no result>"
        return f"(hash={self.hash}, token={self.token}) -> {content_str} (alive={self.alive_duration:.1f}s, last_active={self.active_duration:.1f}s, subscribers={len(self._subscribers)})"

    @final
    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def has_result(self) -> bool:
        return self._result is not None

    @property
    def result(self) -> ResultT:
        assert self._result is not None, f"no result for {self!r}"
        return self._result

    @property
    def alive_duration(self) -> float:
        return (monons() - self._created_at) / 1_000_000

    @property
    def active_duration(self) -> float:
        return (monons() - self._last_active_at) / 1_000_000

    def bump_active(self):
        self._last_active_at = monons()

    @abc.abstractmethod
    async def connect(self, session: Session) -> ResultT:
        """Execute the query."""
        ...

    @final
    async def subscribe(
        self, subject: Subject, since_epoch: int
    ) -> "ConnectionSubscription[UpdateT]":
        """Subscribe to the query results."""
        subscription = await self._do_subscribe(subject, since_epoch)
        self._subscribers.append(subscription)
        return subscription

    @abc.abstractmethod
    async def _do_subscribe(
        self, subject: Subject, since_epoch: int
    ) -> "ConnectionSubscription[UpdateT]": ...

    @final
    def unsubscribe(self, subscription: "ConnectionSubscription[UpdateT]"):
        if subscription not in self._subscribers:
            raise ValueError(f"{subscription!r} is not subscribed to {self!r}")
        self._subscribers.remove(subscription)

    @abc.abstractmethod
    def on_commit(
        self,
        graph: NodeGraphLike,
        data_graph: NodeDataGraphLike,
        edits: list[EditData],
        cascaded_edits: list[EditData],
        epoch: int,
    ):
        """Update result according to the commit, updating subscribers (maybe asynchronously)."""
        ...

    @final
    def _do_notify(self, update: UpdateT):
        for subscriber in self._subscribers:
            subscriber._update_queue.put_nowait(update)


class ConnectionSubscription[UpdateT: Any]:
    """An active subscriber to the query connection."""

    def __init__(self, connection: Connection, subject: Subject, since_epoch: int):
        self.connection = connection
        self.subject = subject
        self._since_epoch = since_epoch
        self._subscribed_at = monons()
        self._closed_at: int | None = None
        self._update_queue: asyncio.Queue[UpdateT] = asyncio.Queue()

    def __str__(self):
        return f"{self.subject!r} on {self.connection!r}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def active_duration(self) -> float:
        return (monons() - self._subscribed_at) / 1_000_000

    @property
    def queue(self) -> asyncio.Queue[UpdateT]:
        return self._update_queue

    def cancel(self):
        if self._closed_at is not None:
            raise RuntimeError(f"{self!r} is already closed")
        self._closed_at = monons()
        self.connection.unsubscribe(self)


# NOTE :Architecture :Security!: filter watch updates to allowed subset
#  (might need a per-connection-type subscription subtype?)


@dataclass(slots=True)
class GetResult:
    graph: NodeDataGraph


@dataclass(slots=True)
class WatchEditsUpdate:
    edits: list[EditData]
    cascaded_edits: list[EditData]
    epoch: int


class GetConnection(Connection[GetResult, WatchEditsUpdate]):
    """Connected get query in the graph."""

    def __init__(self, scope: GraphScope, query: QueryBuilder):
        super().__init__(scope, query)
        self._node_types = bittuple(*query.all_node_types)

    def __result_str__(self, result: GetResult) -> str:
        return f"{len(result.graph)} nodes"

    @override
    async def connect(self, session: Session) -> GetResult:
        fetch = await session.tx._read_connection.fetch(self.query, FetchOptions(count=False))
        graph = NodeDataGraph(
            scope=self.scope, node_types=tuple(self.query.all_node_types), nodes=fetch.nodes
        )
        result = GetResult(graph)
        self._result = result
        return result

    @override
    async def _do_subscribe(
        self, subject: Subject, since_epoch: int
    ) -> ConnectionSubscription[WatchEditsUpdate]:
        # nocheckin: GetConnection replay
        return ConnectionSubscription(self, subject, since_epoch)

    def on_commit(
        self,
        graph: NodeGraphLike,
        data_graph: NodeDataGraphLike,
        edits: list[EditData],
        cascaded_edits: list[EditData],
        epoch: int,
    ):
        # filter relevant edits & update result graph
        cached_graph = self.result.graph
        filtered_edits = []
        filtered_cascaded_edits = []
        for edit in chain(edits, cascaded_edits):
            node_type = NodeType(edit.node_ptr.type)
            if node_type not in self._node_types:
                continue
            assert edit.node_ptr.id, f"missing node id for {edit.node_ptr!r}"
            old_node_data = cached_graph.get(edit.node_ptr.id)
            new_node_data = data_graph.get(edit.node_ptr.id)
            assert new_node_data is not None, f"missing node data for {edit.node_ptr!r}"
            # cached_graph.update(new_node_data)  # nocheckin

        if filtered_edits or filtered_cascaded_edits:
            self.bump_active()
            # notify subscribers
            update = WatchEditsUpdate(
                edits=filtered_edits, cascaded_edits=filtered_cascaded_edits, epoch=epoch
            )
            self._do_notify(update)


@dataclass(slots=True)
class SearchResult:
    graph: NodeDataGraph
    roots: list[AnyNodeData]
    roots_ptr: list[NodeReferenceData]
    total: int | None


@dataclass(slots=True)
class WatchSearchUpdate:
    edits: list[EditData]
    cascaded_edits: list[EditData]
    added_nodes: list[AnyNodeData]
    removed_nodes: list[AnyNodeData]
    epoch: int


class SearchConnection(Connection):
    """Connected search query in the graph."""

    def __init__(self, scope: GraphScope, query: QueryBuilder):
        super().__init__(scope, query)
        self._node_types = bittuple(*query.all_node_types)

    def __result_str__(self, result: SearchResult) -> str:
        return f"{len(result.graph)} nodes, {len(result.roots)} roots, total={result.total}"

    @override
    async def connect(self, session: Session) -> SearchResult:
        fetch = await session.tx._read_connection.fetch(self.query, FetchOptions(count=True))
        graph = NodeDataGraph(
            scope=self.scope, node_types=tuple(self.query.all_node_types), nodes=fetch.nodes
        )
        roots = [cast(AnyNodeData, graph[cast(str, ptr.id)]) for ptr in fetch.roots]
        result = SearchResult(
            graph=graph, roots=roots, roots_ptr=list(fetch.roots), total=fetch.total
        )
        self._result = result
        return result

    @override
    async def _do_subscribe(
        self, subject: Subject, since_epoch: int
    ) -> ConnectionSubscription[WatchSearchUpdate]:
        # nocheckin: SearchConnection replay
        return ConnectionSubscription(self, subject, since_epoch)

    def on_commit(
        self,
        graph: NodeGraphLike,
        data_graph: NodeDataGraphLike,
        edits: list[EditData],
        cascaded_edits: list[EditData],
        epoch: int,
    ):
        raise NotImplementedError("nocheckin: SearchConnection.on_commit")


@dataclass(slots=True)
class AggregationResult:
    aggregation: AggregationData


@dataclass(slots=True)
class AggregationUpdate:
    aggregation: AggregationData


class AggregateConnection(Connection[AggregationResult, AggregationUpdate]):
    """Connected aggregate query in the graph."""

    def __result_str__(self, result: AggregationResult) -> str:
        return f"{result.aggregation!r}"

    @override
    async def connect(self, session: Session) -> AggregationResult:
        aggregate = await session.tx._read_connection.aggregate(self.query)
        result = AggregationResult(aggregation=aggregate.aggregation)
        self._result = result
        return result

    @override
    async def _do_subscribe(
        self, subject: Subject, since_epoch: int
    ) -> ConnectionSubscription[AggregationUpdate]:
        raise NotImplementedError("watch aggregation not yet supported")

    def on_commit(
        self,
        graph: NodeGraphLike,
        data_graph: NodeDataGraphLike,
        edits: list[EditData],
        cascaded_edits: list[EditData],
        epoch: int,
    ):
        pass  # not yet implemented (see above)


class QueryConnector:
    """Connect and cache queries to the graph."""

    def __init__(self, scope: GraphScope):
        self.scope = scope
        self._connections_by_hash: dict[int, Connection] = {}
        self._connections_by_token: dict[str, Connection] = {}
        self._lock_by_hash: dict[int, asyncio.Lock] = {}

    def _add_connection(self, connection: Connection):
        self._connections_by_hash[connection.hash] = connection
        self._connections_by_token[connection.token] = connection

    async def connect[ConnectionT: Connection](
        self, query: QueryBuilder, session: Session, connection_t: type[ConnectionT], *, cache: bool
    ) -> ConnectionT:
        """Creates or reuses a connection to the graph."""
        if not cache:
            connection = connection_t(self.scope, query)
            await connection.connect(session)
            logger.debug(
                f"connect.{query._read_type.name.lower()}",
                query=query,
                query_hash=connection.hash,
            )
            return connection
        else:
            # ensure there's only one connection per query
            query_hash = query._stable_hash()
            lock = self._lock_by_hash.get(query_hash)
            if lock is None:
                lock = asyncio.Lock()
                self._lock_by_hash[query_hash] = lock
            async with lock:
                connection = self._connections_by_hash.get(query_hash)
                was_cached = connection is not None
                if connection is None:
                    connection = connection_t(self.scope, query)
                    await connection.connect(session)
                    self._add_connection(connection)
                else:
                    if not isinstance(connection, connection_t):
                        raise ValueError(f"unexpected {connection!r} (want {connection_t})")
                    connection.bump_active()
                logger.debug(
                    f"connect.{query._read_type.name.lower()}",
                    query=query,
                    was_cached=was_cached,
                    query_hash=query_hash,
                )
                return connection

    async def subscribe[ConnectionT: Connection, UpdateT: Any](
        self,
        subject: Subject,
        connection_t: type[ConnectionT],
        update_t: type[UpdateT],
        connection_token: str,
        since_epoch: int,
    ) -> ConnectionSubscription[UpdateT]:
        """Subscribes to an existing graph connection."""
        connection = self._connections_by_token.get(connection_token)
        if connection is None:
            raise ValueError(f"no connection with token {connection_token!r}")
        if not isinstance(connection, connection_t):
            raise ValueError(f"unexpected connection {connection!r} (want {connection_t})")
        subscription = await connection.subscribe(subject, since_epoch)
        return subscription

    @tracer.start_as_current_span("connection.on_commit")
    def on_commit(
        self,
        graph: NodeGraphLike,
        data_graph: NodeDataGraphLike,
        edits: list[EditData],
        cascaded_edits: list[EditData],
        epoch: int,
    ):
        """Updates all active connections with a new commit (maybe async)."""
        for connection in self._connections_by_hash.values():
            connection.on_commit(graph, data_graph, edits, cascaded_edits, epoch)
