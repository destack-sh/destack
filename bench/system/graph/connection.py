import abc
import asyncio
from itertools import chain
from typing import Any, ClassVar, Sequence, final, override

import structlog
from opentelemetry import trace

from bench.language import (
    ROOT_NODE_TYPES,
    AggregateOptions,
    AggregateResultData,
    EditType,
    GetOptions,
    GetResultData,
    NodeDataGraph,
    NodeReference,
    NodeType,
    Query,
    QueryType,
    SearchOptions,
    SearchResultData,
    Session,
    Subject,
    WatchAggregateUpdateData,
    WatchGetUpdateData,
    WatchSearchUpdateData,
    apply_sort,
    evaluate_conditional,
)
from bench.language.core import bittuple
from bench.proto import (
    AnyNodeData,
    EditData,
    GraphScopeData,
    NodeReferenceData,
    unwrap_some_node,
)
from bench.utils.func import generate_access_token
from bench.utils.oracle import Oracle
from bench.utils.utils import get_from_env

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

CONNECTION_REPLAY_BUFFER_SIZE = get_from_env(
    "CONNECTION_REPLAY_BUFFER_SIZE",
    typ=int,
    default=64,
    description="Number of recent updates to retain for replay per system connection",
)
CONNECTION_CACHE_EXPIRE_SECONDS = get_from_env(
    "CONNECTION_CACHE_EXPIRE_SECONDS",
    typ=int,
    default=30,
    description="How long to keep unused connections around",
)


class Connection[
    ResultT: GetResultData | SearchResultData | AggregateResultData,
    UpdateT: WatchGetUpdateData | WatchSearchUpdateData | WatchAggregateUpdateData,
](abc.ABC):
    """
    A system-side query connection to a (sub)graph.
    This is the counterpart to the runtime/language connections that answers the calls;
     here we cache results, replay and push updates to connection subscribers.
    """

    read_type: ClassVar[QueryType]

    def __init__(self, scope: GraphScopeData, query: Query, oracle: Oracle):
        self.scope = scope
        self.hash = query._stable_hash()
        self.token: str = generate_access_token(length=8)
        self.query = query
        self.node_types = bittuple(*query.all_node_types)
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
        graph: NodeDataGraph,
        edits: Sequence[EditData],
        cascaded_edits: Sequence[EditData],
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


def _get_edited_node(updated_graph: NodeDataGraph, edit: EditData) -> AnyNodeData | None:
    """Get the full edited node data."""
    node_id = edit.node_ptr.id
    assert node_id, f"missing node id for {edit.node_ptr!r} in {edit!r}"
    updated_node = updated_graph.get(node_id)
    if updated_node is None:
        if edit.type in (EditType.CREATE, EditType.UPSERT, EditType.RESTORE):
            assert edit.HasField("node_data"), f"missing new node data for {edit!r}"
            updated_node = unwrap_some_node(edit.node_data)
        else:
            return None
    return updated_node


class GetConnection(Connection[GetResultData, WatchGetUpdateData]):
    """
    Connected get query in the graph.
    """

    read_type: ClassVar[QueryType] = QueryType.GET

    def __init__(self, scope: GraphScopeData, query: "Query", oracle: Oracle):
        super().__init__(scope, query, oracle)
        self._root_ids: set[str] = {str(r.id) for r in query._roots or () if r.id}
        self._roots_cks: set[str] = {str(r.ck) for r in query._roots or () if r.ck}

    def __result_str__(self, result: GetResultData) -> str:
        return f"nodes={len(result.graph)}"

    def _apply_node_edits_in_scope(
        self,
        updated_graph: NodeDataGraph,
        edits: Sequence[EditData],
        is_cascaded: bool,
    ):
        """
        Apply node edits that are in the scope of the result graph, return applied edits.
        'in scope' here means the node or its parent is already in the given graph.
        """
        result_graph = self.result.graph
        relevant_edits: list[EditData] = []
        added_nodes: list[AnyNodeData] = []
        removed_nodes_ptr: list[NodeReferenceData] = []  # not used yet
        for edit in edits:
            # filter type
            node_type = NodeType(edit.node_ptr.node_type)
            if node_type not in self.node_types:
                continue  # irrelevant type
            edit_type = EditType(edit.type)

            # filter scope
            if edit.type in (EditType.CREATE, EditType.UPSERT) or (
                not self.query.include_deleted and edit_type == EditType.RESTORE
            ):
                node = _get_edited_node(updated_graph, edit)
                assert node is not None, f"missing node {edit.node_ptr.id} for {edit!r}"
                # add: parent must be in a root, in our graph or be optional
                parent_id = node.parent_ptr.id if node.parent_ptr else None
                if parent_id is None:
                    if node.metatype in ROOT_NODE_TYPES:
                        continue  # unrelated root node
                    else:
                        raise RuntimeError(f"missing parent ptr for {node!r} in {edit!r}")
                if parent_id in result_graph:
                    # already have a parent, check if parent is a root or just a common ancestor
                    parent = result_graph.get(parent_id)
                    while parent is not None:
                        if (
                            parent.id in self._root_ids
                            or getattr(parent, "ck", None) in self._roots_cks
                        ):
                            # yup, parent is a real root
                            is_in_scope = True
                            break
                        if parent.parent_ptr and parent.parent_ptr.id:
                            parent = result_graph.get(parent.parent_ptr.id)
                        else:
                            parent = None
                    else:
                        # just a shared ancestor, not in scope
                        is_in_scope = False
                elif node.id in self._root_ids or getattr(node, "ck", None) in self._roots_cks:
                    # optional root, add the node and its ancestors
                    is_in_scope = True
                    ancestor = updated_graph.get(parent_id)
                    while ancestor is not None:
                        if ancestor.id not in result_graph:
                            result_graph.add(ancestor)
                            added_nodes.insert(0, ancestor)  # :ConnectionUpdateOrdering
                        if ancestor.parent_ptr and ancestor.parent_ptr.id:
                            ancestor = updated_graph.get(ancestor.parent_ptr.id)
                        else:
                            ancestor = None
                else:
                    is_in_scope = False  # not in scope
            else:
                # update/remove: node must already be in our result graph
                is_in_scope = edit.node_ptr.id in result_graph
            if is_in_scope:
                node = _get_edited_node(updated_graph, edit)
                if node is None:
                    node = result_graph[edit.node_ptr.id]
                # apply (just use updated node instead of actually applying edit, we're read only)
                relevant_edits.append(edit)
                if edit_type == EditType.ERASE or (
                    not self.query.include_deleted and edit_type == EditType.DELETE
                ):
                    if node.id in result_graph:
                        result_graph.remove(node)
                elif node.id in result_graph:
                    result_graph.update(node)
                else:
                    result_graph.add(node)
        return relevant_edits, added_nodes, removed_nodes_ptr

    @override
    async def connect(self, session: Session) -> GetResultData:
        channel = await session._get_channel_for(
            self.scope,
            self.query.all_node_types,
            include_deleted=self.query.include_deleted,
            is_readonly=True,
        )
        connection = await channel.get(self.query, GetOptions(live=False, mode="packed"))
        self._result_data = connection.result_data
        return self._result_data

    def on_commit(
        self,
        graph: NodeDataGraph,
        edits: Sequence[EditData],
        cascaded_edits: Sequence[EditData],
        epoch: int,
    ):
        # filter to relevant edits & update result graph
        # NOTE :Incomplete: GetConnection doesn't handle root/parent changes properly yet
        #  (like SearchConnection, and we also don't handle permissions anywhere yet)
        filtered_edits, added_nodes, removed_nodes_ptr = self._apply_node_edits_in_scope(
            updated_graph=graph, edits=edits, is_cascaded=False
        )
        filtered_cascaded_edits, _, _ = self._apply_node_edits_in_scope(
            updated_graph=graph, edits=cascaded_edits, is_cascaded=True
        )

        # emit update if any
        if filtered_edits or filtered_cascaded_edits:
            update = WatchGetUpdateData(
                edits=filtered_edits,
                cascaded_edits=filtered_cascaded_edits,
                added_nodes=added_nodes,
                removed_nodes_ptr=removed_nodes_ptr,
                epoch=epoch,
            )
            self.notify_update(update)


class SearchConnection(Connection[SearchResultData, WatchSearchUpdateData]):
    """
    Connected search query in the graph.
    If live, we update the result set dynamically (with added/removed nodes).
    In its final form, this should be a proper incremental materialized view.
    """

    def __init__(self, scope: GraphScopeData, query: Query, oracle: Oracle):
        super().__init__(scope, query, oracle)
        self._filter = query._filter
        self._block_ck = str(query._base_block.ck) if query._base_block else None
        self._result_roots_ids: set[str] | None = None

    read_type: ClassVar[QueryType] = QueryType.SEARCH

    def __result_str__(self, result: SearchResultData) -> str:
        return f"nodes={len(result.graph)}, roots={len(result.roots)}, total={result.total}"

    @override
    async def connect(self, session: Session) -> SearchResultData:
        channel = await session._get_channel_for(
            self.scope,
            self.query.all_node_types,
            include_deleted=self.query.include_deleted,
            is_readonly=True,
        )
        connection = await channel.search(
            self.query, SearchOptions(live=False, mode="packed", count=True)
        )
        self._result_data = connection.result_data
        self._result_roots_ids = {node.id for node in self._result_data.roots}
        return self._result_data

    def on_commit(
        self,
        graph: NodeDataGraph,
        edits: Sequence[EditData],
        cascaded_edits: Sequence[EditData],
        epoch: int,
    ):
        assert self._result_data is not None, f"no result for {self!r}"
        assert self._result_roots_ids is not None, f"no result for {self!r}"

        # NOTE :Incomplete: support ancestors/descendants/joins in (live) search connection
        #  This seems tricky because we'll have to re-query somehow when a new root is added
        #   (we don't have its ancestors/descendants ready anywhere),
        #  and because we need to somehow split roots from descendants/ancestors if they
        #  are the same type (like when querying Runs with some filter and their descendants).
        # Also I think this doesn't support moves correctly (doesn't prune/add ancestors).
        # NOTE :Architecture :Broken: we need to stablize :ConnectionUpdateOrdering to handle all cases
        #  (sometimes we need added_nodes before edits, sometimes, after, sometimes both -
        #   so we really need to put all required changes in a common order somehow)

        # filter all edits to figure out new roots
        relevant_edits: list[EditData] = []
        added_nodes: list[AnyNodeData] = []
        removed_nodes_ptr: list[NodeReferenceData] = []
        for edit in chain(edits, cascaded_edits):
            if edit.node_ptr.node_type != self.query._node_type:
                continue
            node_id = edit.node_ptr.id
            assert node_id, f"missing node id for {edit.node_ptr!r} in {edit!r}"
            is_extant = node_id in self._result_roots_ids
            node = _get_edited_node(graph, edit)
            if node is None:
                if not is_extant:
                    continue  # ignore irrelevant remove
                node = self._result_data.graph[node_id]
            is_relevant = (
                self._block_ck is None or self._block_ck == getattr(node, "block_ptr").ck
            ) and (self.query._filter is None or evaluate_conditional(self.query._filter, node))
            if not (is_relevant or is_extant):
                continue  # ignore irrelevant edit
            is_add = edit.type in (EditType.CREATE, EditType.UPSERT, EditType.RESTORE)
            is_remove = edit.type in (EditType.DELETE, EditType.ERASE)
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
            # ensure ancestors are in graph for any added/moved nodes
            for node in self._result_data.roots:
                if not node.parent_ptr or not node.parent_ptr.id:
                    continue
                ancestor = graph.get(node.parent_ptr.id)
                while ancestor is not None:
                    if ancestor.id not in self._result_data.graph:
                        self._result_data.graph.add(ancestor)
                        added_nodes.insert(0, ancestor)  # :ConnectionUpdateOrdering
                    ancestor = graph.get(ancestor.parent_ptr.id) if ancestor.parent_ptr.id else None

            # apply sort & limit
            if self.query._sort:
                apply_sort(self.query._sort, self._result_data.roots)
            if self.query._first is not None and len(self._result_data.roots) > self.query._first:
                trimmed = self._result_data.roots[self.query._first :]
                self._result_data.roots = self._result_data.roots[: self.query._first]
                for node in trimmed:
                    removed_nodes_ptr.append(NodeReference._ref_data_from_node_data(node))
                    self._result_data.graph.remove(node)
                    self._result_roots_ids.remove(node.id)
            self._result_data.roots_ptr = [  # .roots_ptr is derived from .roots
                NodeReference._ref_data_from_node_data(node) for node in self._result_data.roots
            ]

            # emit update
            update = WatchSearchUpdateData(
                edits=relevant_edits,
                cascaded_edits=[],
                roots_ptr=self._result_data.roots_ptr,
                added_nodes=added_nodes,
                removed_nodes_ptr=removed_nodes_ptr,
                total=self._result_data.total,
                epoch=epoch,
            )
            self.notify_update(update)


class AggregateConnection(Connection[AggregateResultData, WatchAggregateUpdateData]):
    """
    Connected aggregate query in the graph.
    Live isn't supported yet, but eventually (like search) this should be incremental materialized view.
    """

    read_type: ClassVar[QueryType] = QueryType.AGGREGATE

    def __result_str__(self, result: AggregateResultData) -> str:
        return f"aggregation={result.aggregation!r}"

    @override
    async def connect(self, session: Session) -> AggregateResultData:
        channel = await session._get_channel_for(
            self.scope,
            self.query.all_node_types,
            include_deleted=self.query.include_deleted,
            is_readonly=True,
        )
        connection = await channel.aggregate(
            self.query, AggregateOptions(live=False, mode="packed")
        )
        self._result_data = connection.result_data
        return self._result_data

    def on_commit(
        self,
        graph: NodeDataGraph,
        edits: Sequence[EditData],
        cascaded_edits: Sequence[EditData],
        epoch: int,
    ):
        pass  # not yet implemented (watch_aggregate errors with not implemented for now)


# NOTE :Robustness: restrict caching in global supervisor graph? or ensure there's only ever one?


class ConnectionIndex:
    """Connect and cache queries to the graph."""

    def __init__(self, owner: Any, scope: GraphScopeData, oracle: Oracle):
        self.owner = owner
        self.scope = scope
        self.oracle = oracle
        self._connections_by_hash: dict[int, Connection] = {}
        self._connections_by_token: dict[str, Connection] = {}
        self._lock_by_connection: dict[int, asyncio.Lock] = {}
        self._log = logger.bind(owner=owner, scope=scope)

    def _add_connection(self, connection: Connection):
        self._log.trace("connect.add", connection=connection)
        self._connections_by_hash[connection.hash] = connection
        self._connections_by_token[connection.token] = connection

    def _remove_connection(self, connection: Connection):
        self._log.trace("connect.remove", connection=connection)
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
                self._log.debug("connect.gc", connection=connection)
                self._remove_connection(connection)
        if before_count != len(self._connections_by_hash):
            self._log.trace(
                "connect.gc",
                now_ns=now_ns,
                before_connections=before_count,
                after_connections=len(self._connections_by_hash),
            )

    async def connect[ConnectionT: Connection](
        self, query: Query, session: Session, connection_t: type[ConnectionT], *, cache: bool
    ) -> ConnectionT:
        """Creates or reuses a connection to the graph."""
        assert query._type == connection_t.read_type, f"unexpected {query!r} (want {connection_t})"
        if not cache:
            connection = connection_t(self.scope, query, self.oracle)
            await connection.connect(session)
            self._log.debug(
                f"connect.{query._type.name.lower()}",
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
                self._log.debug(
                    f"connect.{query._type.name.lower()}",
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
        graph: NodeDataGraph,
        edits: Sequence[EditData],
        cascaded_edits: Sequence[EditData],
        epoch: int,
    ):
        """Updates all active connections with a new commit."""
        for connection in self._connections_by_hash.values():
            if (connection.node_types.bits & graph.node_types.bits).any():
                connection.on_commit(graph, edits, cascaded_edits, epoch)
