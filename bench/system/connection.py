import abc
import asyncio
from itertools import chain
from typing import Any, ClassVar, final, override

import structlog
from opentelemetry import trace

from bench.language import NodeReference, Session, Subject
from bench.language.connection import (
    AggregateOptions,
    AggregateResultData,
    GetOptions,
    GetResultData,
    SearchOptions,
    SearchResultData,
    WatchAggregateUpdate,
    WatchGetUpdate,
    WatchSearchUpdate,
)
from bench.language.const import EditType, NodeType, ReadType
from bench.language.expression import apply_sort, evaluate_conditional
from bench.language.graph import NodeDataGraphLike
from bench.language.query import DEFAULT_READ_OPTIONS, QueryBuilder
from bench.language.transaction import unpack_node_delta
from bench.proto.wire import (
    AnyNodeData,
    EditData,
    GraphScopeData,
    NodeReferenceData,
)
from bench.utils.func import bittuple, generate_access_token
from bench.utils.oracle import Oracle
from bench.utils.utils import get_from_env

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

CONNECTION_CACHE_ENABLED = get_from_env(
    "CONNECTION_CACHE_ENABLED",
    typ=bool,
    default=True,
    description="Whether to cache system graph connections",
)
CONNECTION_REPLAY_BUFFER_SIZE = get_from_env(
    "CONNECTION_REPLAY_BUFFER_SIZE",
    typ=int,
    default=64,
    description="Number of recent updates to retain for replay per system connection",
)
CONNECTION_CACHE_EXPIRE_SECONDS = get_from_env(
    "CONNECTION_CACHE_EXPIRE_SECONDS",
    typ=int,
    default=120,
    description="How long to keep unused connections around in seconds",
)
MAX_TIME_DRIFT_SECONDS = get_from_env(
    "MAX_TIME_DRIFT_SECONDS",
    typ=int,
    default=60,
    description="Maximum allowable delta between our time and client transaction time",
)


class Connection[
    ResultT: GetResultData | SearchResultData | AggregateResultData,
    UpdateT: WatchGetUpdate | WatchSearchUpdate | WatchAggregateUpdate,
](abc.ABC):
    """
    A system-side query connection to a (sub)graph.
    This is the counterpart to the runtime/language connections that answers the calls,
     where we cache results, replay and push updates to connection subscribers.
    """

    read_type: ClassVar[ReadType]

    def __init__(self, scope: GraphScopeData, query: QueryBuilder, oracle: Oracle):
        self.scope = scope
        self.hash = query._stable_hash()
        self.token: str = generate_access_token(length=8)
        self.query = query
        self._node_types = bittuple(*query.all_node_types)
        self._subscribers: list[ConnectionSubscription[UpdateT]] = []
        self.oracle = oracle
        now_ns = oracle.time_ns()
        self._created_at_ns = now_ns
        self._last_active_at_ns = now_ns
        self._last_referenced_at_ns = now_ns
        self._result_data: ResultT | None = None
        self._replay_buffer: list[UpdateT] = []

    @abc.abstractmethod
    def __result_str__(self, result: ResultT) -> str: ...

    @final
    def __str__(self):
        content_str = self.__result_str__(self._result_data) if self._result_data else "<no result>"
        now_ns = self.oracle.time_ns()
        alive_duration = (now_ns - self._created_at_ns) / 1_000_000_000
        active_duration = (now_ns - self._last_active_at_ns) / 1_000_000_000
        return f"{content_str} (hash={self.hash}, token={self.token}, alive={alive_duration:.1f}s, last_active={active_duration:.1f}s, subscribers={len(self._subscribers)})"

    @final
    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def has_result(self) -> bool:
        return self._result_data is not None

    @property
    def result(self) -> ResultT:
        assert self._result_data is not None, f"no result for {self!r}"
        return self._result_data

    @property
    def has_subscribers(self) -> bool:
        return len(self._subscribers) > 0

    def bump_active(self):
        self._last_active_at_ns = self.oracle.time_ns()

    def bump_referenced(self):
        self._last_referenced_at_ns = self.oracle.time_ns()

    @abc.abstractmethod
    async def connect(self, session: Session) -> ResultT:
        """Execute the query."""
        ...

    @final
    async def subscribe(
        self, subject: Subject, since_epoch: int
    ) -> "ConnectionSubscription[UpdateT]":
        """Subscribe to the query results."""
        subscription = ConnectionSubscription(self, subject, since_epoch)

        # replay updates with epoch < since_epoch
        for update in self._replay_buffer:
            if update.epoch > since_epoch:
                subscription._update_queue.put_nowait(update)

        self._subscribers.append(subscription)
        return subscription

    @final
    def unsubscribe(self, subscription: "ConnectionSubscription[UpdateT]"):
        if subscription not in self._subscribers:
            raise ValueError(f"{subscription!r} is not subscribed to {self!r}")
        self.bump_referenced()  # set last referenced to now
        self._subscribers.remove(subscription)

    @abc.abstractmethod
    def on_commit(
        self,
        graph: NodeDataGraphLike,
        edits: list[EditData],
        cascaded_edits: list[EditData],
        epoch: int,
    ):
        """Update result according to the commit, updating subscribers (maybe asynchronously)."""
        ...

    @final
    def notify_update(self, update: UpdateT):
        self.bump_active()
        self._replay_buffer.append(update)
        if len(self._replay_buffer) > CONNECTION_REPLAY_BUFFER_SIZE:
            self._replay_buffer.pop(0)

        for subscriber in self._subscribers:
            subscriber._update_queue.put_nowait(update)


class ConnectionSubscription[UpdateT: Any]:
    """An active subscriber to the query connection."""

    def __init__(self, connection: Connection, subject: Subject, since_epoch: int):
        self.connection = connection
        self.subject = subject
        self._since_epoch = since_epoch
        self._subscribed_at_ns = connection.oracle.time_ns()
        self._closed_at_ns: int | None = None
        self._update_queue: asyncio.Queue[UpdateT] = asyncio.Queue()

    def __str__(self):
        return f"{self.subject!r} on {self.connection!r}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def active_duration(self) -> float:
        return (self.connection.oracle.time_ns() - self._subscribed_at_ns) / 1_000_000

    @property
    def queue(self) -> asyncio.Queue[UpdateT]:
        return self._update_queue

    def cancel(self):
        if self._closed_at_ns is not None:
            raise RuntimeError(f"{self!r} is already closed")
        self._closed_at_ns = self.connection.oracle.time_ns()
        self.connection.unsubscribe(self)


# TODO :Security!: apply policies to connection subscriptions
#  (need some per-connection-type subscription info?)


class NodeConnection[
    ResultT: "GetResultData | SearchResultData",
    UpdateT: "WatchGetUpdate | WatchSearchUpdate",
](Connection[ResultT, UpdateT]):
    """Base connection for get/search queries in the graph."""

    def _unpack_edited_node(self, updated_graph: NodeDataGraphLike, edit: EditData) -> AnyNodeData:
        """Extract the full edited node data."""
        node_id = edit.node_ptr.id
        assert node_id, f"missing node id for {edit.node_ptr!r} in {edit!r}"
        updated_node = updated_graph.get(node_id)
        if updated_node is None:
            if edit.type in (EditType.ARCHIVE, EditType.DELETE, EditType.ERASE):
                assert edit.old_node_packed, f"missing old node data for {edit!r}"
                updated_node = unpack_node_delta(
                    edit.old_node_packed, node_type=self.query._node_type
                )
            elif edit.type in (
                EditType.UNARCHIVE,
                EditType.RESTORE,
                EditType.CREATE,
                EditType.UPSERT,
            ):
                assert edit.new_node_packed, f"missing new node data for {edit!r}"
                updated_node = unpack_node_delta(
                    edit.new_node_packed, node_type=self.query._node_type
                )
            else:
                raise RuntimeError(f"unexpected empty edit type {edit.type} in {edit!r}")
        assert updated_node is not None, f"missing node data for {edit.node_ptr!r}"
        return updated_node

    def _apply_node_edits_in_scope(
        self,
        updated_graph: NodeDataGraphLike,
        edits: list[EditData],
        is_cascaded: bool,
    ) -> list[EditData]:
        """
        Apply node edits that are in the scope of the result graph, return applied edits.
        'in scope' here means the node or its parent is already in the given graph.
        """
        options = self.query._options or DEFAULT_READ_OPTIONS
        result_graph = self.result.graph
        applied_edits: list[EditData] = []
        for edit in edits:
            # filter type
            node_type = NodeType(edit.node_ptr.type)
            if node_type not in self._node_types:
                continue  # irrelevant type
            edit_type = EditType(edit.type)

            # filter scope
            updated_node = self._unpack_edited_node(updated_graph, edit)
            if edit.type in (EditType.CREATE, EditType.UPSERT) or (
                not options.include_hidden and edit_type in (EditType.UNARCHIVE, EditType.RESTORE)
            ):
                # parent must be in our result graph
                #  (cannot be a root type here, so must have a parent)
                parent_id = updated_node.parent_ptr.id if updated_node.parent_ptr else None
                assert parent_id, f"missing parent for {updated_node!r}"
                is_in_scope = parent_id in result_graph
            else:
                # node must be in our result graph
                is_in_scope = updated_node.id in result_graph
            if not is_in_scope:
                continue  # irrelevant scope

            # apply (just copy node instead of actually applying edit, we don't modify anything)
            applied_edits.append(edit)
            if edit_type == EditType.ERASE or (
                not options.include_hidden and edit_type in (EditType.ARCHIVE, EditType.DELETE)
            ):
                if updated_node.id in result_graph:
                    result_graph.remove(updated_node)
            elif updated_node.id in result_graph:
                result_graph.update(updated_node)
            else:
                result_graph.add(updated_node)
        return applied_edits


class GetConnection(NodeConnection[GetResultData, WatchGetUpdate]):
    """
    Connected get query in the graph.
    If live and any root is removed, we error (like the usual get behavior; not sure about this).
    """

    read_type: ClassVar[ReadType] = ReadType.GET

    def __result_str__(self, result: GetResultData) -> str:
        return f"nodes={len(result.graph)}"

    @override
    async def connect(self, session: Session) -> GetResultData:
        channel = await session._get_channel_for(
            self.scope, self.query.all_node_types, is_readonly=True
        )
        connection = await channel.get(self.query, GetOptions(live=False, unpack=False))
        self._result_data = connection.result_data
        return self._result_data

    def on_commit(
        self,
        graph: NodeDataGraphLike,
        edits: list[EditData],
        cascaded_edits: list[EditData],
        epoch: int,
    ):
        # filter to relevant edits & update result graph
        filtered_edits = self._apply_node_edits_in_scope(
            updated_graph=graph, edits=edits, is_cascaded=False
        )
        filtered_cascaded_edits = self._apply_node_edits_in_scope(
            updated_graph=graph, edits=cascaded_edits, is_cascaded=True
        )

        # emit update if any
        if filtered_edits or filtered_cascaded_edits:
            update = WatchGetUpdate(
                edits=filtered_edits,
                cascaded_edits=filtered_cascaded_edits,
                # no added/removed nodes as roots are static in get
                #  (we don't handle permissions here)
                added_nodes=[],
                removed_nodes_ptr=[],
                epoch=epoch,
            )
            self.notify_update(update)


class SearchConnection(NodeConnection[SearchResultData, WatchSearchUpdate]):
    """
    Connected search query in the graph.
    If live, we update the result set dynamically (with added/removed nodes).
    In its final form, this should be a proper incremental materialized view.
    """

    def __init__(self, scope: GraphScopeData, query: QueryBuilder, oracle: Oracle):
        super().__init__(scope, query, oracle)
        #   and we also need to separate roots from descendants/ancestors (like in Runs).
        self._filter = query._filter
        self._result_roots_ids: set[str] | None = None

    read_type: ClassVar[ReadType] = ReadType.SEARCH

    def __result_str__(self, result: SearchResultData) -> str:
        return f"nodes={len(result.graph)}, roots={len(result.roots)}, total={result.total}"

    @override
    async def connect(self, session: Session) -> SearchResultData:
        channel = await session._get_channel_for(
            self.scope, self.query.all_node_types, is_readonly=True
        )
        connection = await channel.search(
            self.query, SearchOptions(live=False, unpack=False, count=True)
        )
        self._result_data = connection.result_data
        self._result_roots_ids = {node.id for node in self._result_data.roots}
        return self._result_data

    def on_commit(
        self,
        graph: NodeDataGraphLike,
        edits: list[EditData],
        cascaded_edits: list[EditData],
        epoch: int,
    ):
        assert self._result_data is not None, f"no result for {self!r}"
        assert self._result_roots_ids is not None, f"no result for {self!r}"

        # NOTE :Incomplete: support ancestors/descendants in (live) search connection
        #  This seems tricky because we'll have to re-query somehow when a new root is added
        #   (we don't have its ancestors/descendants ready anywhere),
        #  and because we need to somehow split roots from descendants/ancestors if they
        #  are the same type (like when querying Runs with some filter and their descendants).

        # filter all edits to figure out new roots
        relevant_edits: list[EditData] = []
        added_nodes: list[AnyNodeData] = []
        removed_nodes_ptr: list[NodeReferenceData] = []
        for edit in chain(edits, cascaded_edits):
            if edit.node_ptr.type != self.query._node_type:
                continue
            node_id = edit.node_ptr.id
            assert node_id, f"missing node id for {edit.node_ptr!r} in {edit!r}"
            node = self._unpack_edited_node(graph, edit)
            is_relevant = self._filter is None or evaluate_conditional(self._filter, node)
            is_extant = node_id in self._result_roots_ids
            if not is_relevant and not is_extant:
                continue  # ignore irrelevant edit
            is_add = edit.type in (EditType.CREATE, EditType.UPSERT) or (
                edit.type in (EditType.UNARCHIVE, EditType.RESTORE)
                and not DEFAULT_READ_OPTIONS.include_hidden
            )
            is_remove = edit.type == EditType.ERASE or (
                edit.type in (EditType.ARCHIVE, EditType.DELETE)
                and not DEFAULT_READ_OPTIONS.include_hidden
            )
            if is_extant:
                if is_relevant and not is_remove:
                    # regular update
                    relevant_edits.append(edit)
                    self._result_data.graph.update(node)
                elif not is_relevant or is_remove:
                    # remove (either because no longer relevant or directly removed)
                    if is_remove:
                        relevant_edits.append(edit)
                    else:
                        removed_nodes_ptr.append(edit.node_ptr)
                    self._result_roots_ids.remove(node_id)
                    self._result_data.graph.remove(node)
                    self._result_data.roots = [
                        node for node in self._result_data.roots if node.id != node_id
                    ]
                    if self._result_data.total is not None:
                        self._result_data.total -= 1
            else:  # is_relevant
                # add (either because now relevant or directly added)
                if is_add:
                    relevant_edits.append(edit)
                elif is_remove:
                    continue  # skip irrelevant remove (wasn't extant.. can this happen?)
                else:
                    added_nodes.append(node)
                self._result_roots_ids.add(node_id)
                self._result_data.graph.add(node)
                self._result_data.roots.insert(0, node)  # insert at front
                if self._result_data.total is not None:
                    self._result_data.total += 1

        if relevant_edits or added_nodes or removed_nodes_ptr:
            # apply sort & limit
            if self.query._sort:
                apply_sort(self.query._sort, self._result_data.roots)
            if self.query._first is not None:
                trimmed = self._result_data.roots[self.query._first :]
                self._result_data.roots = self._result_data.roots[: self.query._first]
                for node in trimmed:
                    removed_nodes_ptr.append(NodeReference.from_node_data(node))
                    self._result_data.graph.remove(node)
                    self._result_roots_ids.remove(node.id)
            self._result_data.roots_ptr = [
                NodeReference.from_node_data(node) for node in self._result_data.roots
            ]

            # emit update
            update = WatchSearchUpdate(
                edits=relevant_edits,
                cascaded_edits=[],
                roots_ptr=self._result_data.roots_ptr,
                added_nodes=added_nodes,
                removed_nodes_ptr=removed_nodes_ptr,
                total=self._result_data.total,
                epoch=epoch,
            )
            self.notify_update(update)


class AggregateConnection(Connection[AggregateResultData, WatchAggregateUpdate]):
    """
    Connected aggregate query in the graph.
    Live isn't supported yet, but eventually (like search) this should be incremental materialized view.
    """

    read_type: ClassVar[ReadType] = ReadType.AGGREGATE

    def __result_str__(self, result: AggregateResultData) -> str:
        return f"aggregation={result.aggregation!r}"

    @override
    async def connect(self, session: Session) -> AggregateResultData:
        channel = await session._get_channel_for(
            self.scope, self.query.all_node_types, is_readonly=True
        )
        connection = await channel.aggregate(self.query, AggregateOptions(live=False, unpack=False))
        self._result_data = connection.result_data
        return self._result_data

    def on_commit(
        self,
        graph: NodeDataGraphLike,
        edits: list[EditData],
        cascaded_edits: list[EditData],
        epoch: int,
    ):
        pass  # not yet implemented (watch_aggregate errors with not implemented for now)


class ConnectionIndex:
    """Connect and cache queries to the graph."""

    def __init__(self, scope: GraphScopeData, oracle: Oracle):
        self.scope = scope
        self.oracle = oracle
        self._connections_by_hash: dict[int, Connection] = {}
        self._connections_by_token: dict[str, Connection] = {}
        self._lock_by_connection: dict[int, asyncio.Lock] = {}

    def _add_connection(self, connection: Connection):
        logger.trace("connect.add", connection=connection)
        self._connections_by_hash[connection.hash] = connection
        self._connections_by_token[connection.token] = connection

    def _remove_connection(self, connection: Connection):
        logger.trace("connect.remove", connection=connection)
        assert not connection.has_subscribers, f"cannot remove {connection!r} with subscribers"
        del self._connections_by_hash[connection.hash]
        del self._connections_by_token[connection.token]
        if connection.hash in self._lock_by_connection:
            del self._lock_by_connection[connection.hash]

    def gc_connections(self):
        """Removes unused connections from the cache."""
        if len(self._connections_by_hash) == 0:
            return  # nothing to do
        before_count = len(self._connections_by_hash)
        now_ns = self.oracle.time_ns()
        for connection in tuple(self._connections_by_hash.values()):
            if (
                not connection.has_subscribers
                and (connection._last_referenced_at_ns - now_ns) > CONNECTION_CACHE_EXPIRE_SECONDS
            ):
                logger.debug("connect.gc", connection=connection)
                self._remove_connection(connection)
        logger.trace(
            "connect.gc",
            now_ns=now_ns,
            before_connections=before_count,
            after_connections=len(self._connections_by_hash),
        )

    async def connect[ConnectionT: Connection](
        self, query: QueryBuilder, session: Session, connection_t: type[ConnectionT], *, cache: bool
    ) -> ConnectionT:
        """Creates or reuses a connection to the graph."""
        assert (
            query._read_type == connection_t.read_type
        ), f"unexpected {query!r} (want {connection_t})"
        if not cache:
            connection = connection_t(self.scope, query, self.oracle)
            await connection.connect(session)
            logger.debug(
                f"connect.{query._read_type.name.lower()}",
                query=query,
                query_hash=connection.hash,
                span="current",
            )
            return connection
        else:
            # ensure there's only one connection per query (even for simultaneous requests)
            query_hash = query._stable_hash()
            connection_lock = self._lock_by_connection.get(query_hash)
            if connection_lock is None:
                connection_lock = asyncio.Lock()
                self._lock_by_connection[query_hash] = connection_lock
            async with connection_lock:
                connection = self._connections_by_hash.get(query_hash)
                was_cached = connection is not None
                if connection is None:
                    connection = connection_t(self.scope, query, self.oracle)
                    await connection.connect(session)
                    self._add_connection(connection)
                else:
                    assert isinstance(connection, connection_t), f"unexpected {connection!r}"
                    connection.bump_active()
                    connection.bump_referenced()
                logger.debug(
                    f"connect.{query._read_type.name.lower()}",
                    query=query,
                    was_cached=was_cached,
                    query_hash=query_hash,
                    span="current",
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
        graph: NodeDataGraphLike,
        edits: list[EditData],
        cascaded_edits: list[EditData],
        epoch: int,
    ):
        """Updates all active connections with a new commit (maybe async)."""
        for connection in self._connections_by_hash.values():
            connection.on_commit(graph, edits, cascaded_edits, epoch)
