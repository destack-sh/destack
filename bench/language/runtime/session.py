import asyncio
import contextvars
from contextlib import asynccontextmanager, suppress
from datetime import datetime, timedelta
from typing import (
    TYPE_CHECKING,
    Awaitable,
    Callable,
    Iterable,
    Optional,
    Sequence,
)
from uuid import UUID

import structlog
from attr import dataclass
from opentelemetry import trace

from bench import pb2
from bench.language.connection import (
    Connection,
    Connector,
    Engine,
    EngineUnavailableError,
    MemoryEngine,
    NullEngine,
    SplitConnector,
    scope_includes,
)
from bench.language.core import (
    ACTIVE_SESSION,
    EMPTY_SCOPE_DATA,
    NODE_TYPES,
    BenchError,
    BenchNode,
    EditType,
    IsModal,
    IsRuntime,
    Node,
    NodeDataGraph,
    NodeGraph,
    NodeMode,
    NodeType,
    PackageNode,
    SessionStatus,
    Subject,
    bittuple,
    p_node_parent,
    p_regular,
    p_runtime,
    repr_scope,
    timed_node_,
)
from bench.language.registry import DESCENDANT_NODE_TYPES
from bench.pb2 import (
    ClientOriginData,
    ContextData,
    EditData,
    GraphScopeData,
    HostClient,
    RpcMetadata,
    SessionData,
    SupervisorClient,
    lang_pb2,
)
from bench.pb2.lang_pb2 import EditOperationData
from bench.utils.func import async_shield, uuid_to_str
from bench.utils.oracle import Oracle
from bench.utils.sync import CriticalLock
from bench.utils.uuidt import UUIDT

from .transaction import Transaction

if TYPE_CHECKING:
    from bench.language import Bench, LegacyQuery
    from bench.runtime.core import Runtime

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass(slots=True, repr=False)
class _CommitEvent:
    """A commit event."""

    id: int
    new_edits: list[EditData]

    def __str__(self):
        return f"id={self.id}, edits={len(self.new_edits)}"

    def __repr__(self):
        return f"<CommitEvent {self}>"


CommitPrepareHook = Callable[
    ["Session", NodeGraph, NodeDataGraph, Sequence["EditData"], Sequence["EditData"]],
    Awaitable[None],
]
CommitHook = Callable[
    [
        "Session",
        NodeGraph,
        NodeDataGraph,
        Sequence["EditData"],
        Sequence["EditData"],
    ],
    Awaitable[None],
]
CommitFailedHook = Callable[["Session", BaseException], Awaitable[None]]


@timed_node_(NodeType.SESSION)
class Session(BenchNode[SessionData], IsRuntime, IsModal):
    """
    A managed Session for interacting with and running a Bench.
    """

    parent: Optional["Bench"] = p_node_parent(4, NodeType.BENCH, is_system=True)

    # status
    status: SessionStatus = p_regular(40, default=SessionStatus.PENDING, index_in_pg=True)
    duration: Optional[timedelta] = p_regular(41)
    opened_at: Optional[datetime] = p_regular(42)
    closed_at: Optional[datetime] = p_regular(43)

    # context
    # ...HasRuntimeContext[80-99]

    # context
    _origin: ClientOriginData | None = p_runtime(default=None)
    _subject: Subject | None = p_runtime(default=None)
    _context_data: ContextData | None = p_runtime(default=None)
    _is_suspended: bool = p_runtime(default=False)

    # connections
    _split_read: bool = p_runtime(default=False)
    _split_read_connector: Connector | None = p_runtime(default=None)
    _engines: tuple["Engine", ...] = p_runtime(default_factory=tuple)
    _connectors: list[Connector] = p_runtime(default_factory=list)
    _connections: list[Connection] = p_runtime(default_factory=list)
    _lock_by_engine_id: dict[int, asyncio.Lock] = p_runtime(default_factory=dict)

    # transaction
    _tx: Transaction | None = p_runtime(default=None)
    _tx_lock: asyncio.Lock = p_runtime(default_factory=lambda: CriticalLock(name="session"))
    _commit_loop_task: asyncio.Task | None = p_runtime(default=None)
    _flush_counter: int = p_runtime(default=0)
    _commit_queue: asyncio.Queue[_CommitEvent] = p_runtime(default_factory=lambda: asyncio.Queue())
    _pending_nodes_by_id: dict[UUID, Node] = p_runtime(default_factory=dict)
    _default_scope: GraphScopeData = p_runtime(default_factory=lambda: EMPTY_SCOPE_DATA)
    _local_epoch: int | None = p_runtime(default=None)
    _pre_commit: CommitPrepareHook | None = p_runtime(default=None)
    _post_commit: CommitHook | None = p_runtime(default=None)
    _post_commit_failed: CommitFailedHook | None = p_runtime(default=None)
    _on_edit_subs: dict[UUID, list[Callable[[Node], None]]] = p_runtime(default_factory=dict)

    # runtime
    _oracle: Oracle = p_runtime()
    _active_session_tokens: list[contextvars.Token] = p_runtime(default_factory=list)
    _rpc_metadata: RpcMetadata | None = p_runtime(default=None)
    _rpc_headers: dict[str, str] | None = p_runtime(default=None)
    _runtime: Optional["Runtime"] = p_runtime(default=None)
    _supervisor: Optional["SupervisorClient"] = p_runtime(default=None)
    _self_host: Optional["HostClient"] = p_runtime(default=None)
    _bench_host: Optional["HostClient"] = p_runtime(default=None)

    def __content_str__(self):
        status_strs = []
        if self.closed_at:
            status_strs.append("closed")
        elif self.opened_at:
            status_strs.append("open")
        else:
            status_strs.append("pending")
        if self._is_suspended:
            status_strs.append("suspended")
        if self.duration is not None:
            duration_str = f"{self.duration.total_seconds():.3f}s"
            return f"{', '.join(status_strs)}, tx={self._tx or '<no tx>'}, duration={duration_str}s"
        else:
            return f"{', '.join(status_strs)}, tx={self._tx or '<no tx>'}"

    @property
    def tx(self) -> Transaction:
        assert self._tx is not None, f"no active transaction in {self!r}"
        return self._tx

    @property
    def edits(self) -> Sequence[EditData]:
        return self._tx._edits if self._tx is not None else ()

    @property
    def cascaded_edits(self) -> Sequence[EditData]:
        return self._tx._cascaded_edits if self._tx is not None else ()

    @property
    def has_edits(self) -> bool:
        """Whether this session has any non-session edits."""
        return self._tx is not None and self._tx.has_edits

    @property
    def has_pending_edits(self):
        """Whether this session has any pending (unflushed) edits."""
        return self._tx is not None and self._tx.has_pending_edits

    @property
    def is_open(self) -> bool:
        return self.opened_at is not None and self.closed_at is None

    @property
    def is_active(self):
        return len(self._active_session_tokens) > 0

    @property
    def is_suspended(self):
        return self._is_suspended

    @property
    def runtime(self) -> "Runtime":
        assert self._runtime is not None, f"no active Runtime in {self!r}"
        return self._runtime

    @property
    def self_host(self) -> HostClient:
        assert self._self_host is not None, f"no active self Host in {self!r}"
        return self._self_host

    @property
    def bench_host(self) -> HostClient:
        assert self._bench_host is not None, f"no active bench Host in {self!r}"
        return self._bench_host

    @property
    def supervisor(self) -> SupervisorClient:
        assert self._supervisor is not None, f"no active Supervisor in {self!r}"
        return self._supervisor

    @property
    def active_mode(self) -> NodeMode:
        return self._runtime.active_mode if self._runtime is not None else self.mode

    def _get_scope_for_node(self, n: Node) -> GraphScopeData:
        """Get the scope for a node in this session."""
        scope = GraphScopeData(metatype=pb2.ObjectType.OBJECT_TYPE_GRAPH_SCOPE)
        if isinstance(n, BenchNode):
            scope.bench_id = uuid_to_str(n.bench_id) or self._default_scope.bench_id
        if isinstance(n, PackageNode):
            package_id = uuid_to_str(n.package_id)
            if package_id is not None:
                scope.package_ids.append(package_id)
        return scope

    def _get_scope_for_query(self, query: "LegacyQuery") -> GraphScopeData:
        """Get the scope for a query in this session."""
        if query._base_type is not None:
            return self._get_scope_for_node(query._base_type)
        elif query._roots:
            scope = GraphScopeData(metatype=pb2.ObjectType.OBJECT_TYPE_GRAPH_SCOPE)
            for root in query._roots:
                if not scope.bench_id and root.bench_id:
                    scope.bench_id = str(root.bench_id)
            return scope
        else:
            return self._default_scope

    def _get_engine(
        self,
        scope: GraphScopeData,
        node_types: NodeType | Iterable[NodeType],
        *,
        is_readonly: bool,
        include_memory: bool,
        include_removed: bool,
    ) -> Engine:
        """Gets the appropriate Engine to read/write Nodes."""
        node_types = (node_types,) if isinstance(node_types, NodeType) else tuple(node_types)
        candidate_engines = [
            engine
            for engine in self._engines
            if (
                (is_readonly or not engine.is_readonly)
                and (not include_removed or engine.include_removed)
                and (include_memory or not isinstance(engine, MemoryEngine))
                and scope_includes(engine.scope, scope)
                and all(t in engine.node_types for t in node_types)
            )
        ]
        if not candidate_engines:
            raise BenchError(
                f"no engine for [scope={repr_scope(scope)}, node_types={'|'.join(t.bench_name for t in node_types)}] in {self!r}"
                f" (engines: {self._engines!r})"
            )
        return candidate_engines[0]

    async def _get_connector(self, engine: Engine) -> Connector:
        """Gets or creates a Connector into an Engine"""
        for connector in self._connectors:
            if connector.engine.id == engine.id:
                return connector
        else:
            # acquire under lock to avoid race condition
            if engine.id not in self._lock_by_engine_id:
                self._lock_by_engine_id[engine.id] = asyncio.Lock()
            async with self._lock_by_engine_id[engine.id]:
                # check again in case another task created connector while waiting
                for connector in self._connectors:
                    if connector.engine.id == engine.id:
                        return connector
                # acquire connector
                connector = await engine.connector(self)
                self._connectors.append(connector)
                return connector

    def _touch_connector(self, connector: Connector):
        """Touch a Connector to mark it as used in the current transaction."""
        self.tx._touched_engine_ids.add(connector.engine.id)

    async def _get_connector_for[ConnectorT: Connector](
        self,
        scope: GraphScopeData,
        node_types: NodeType | Iterable[NodeType],
        *,
        is_readonly: bool = False,
        include_removed: bool = False,
        include_memory: bool = True,
        expect: type[ConnectorT] = Connector,
    ) -> ConnectorT:
        """Gets or creates a store Connector to read/write Nodes."""
        if is_readonly and self._split_read:
            if self._split_read_connector is None:
                self._split_read_connector = SplitConnector(
                    NullEngine("split", self._default_scope, NODE_TYPES), self
                )
            connector = self._split_read_connector
        else:
            engine = self._get_engine(
                scope=scope,
                node_types=node_types,
                is_readonly=is_readonly,
                include_removed=include_removed,
                include_memory=include_memory,
            )
            connector = await self._get_connector(engine)
        if not isinstance(connector, expect):
            raise BenchError(f"unexpected connector type {connector!r} for {expect!r}")
        return connector

    def _on_connection_begin(self, connection: Connection):
        """Called when a connection begins."""
        self._connections.append(connection)

    def _on_connection_end(self, connection: Connection):
        """Called when a connection ends."""
        if connection in self._connections:
            self._connections.remove(connection)

    async def open(self, *, _set_in_context: bool = True):
        """Opens the session for regular business. Activates context (by default)."""
        assert not self.closed_at, f"session already closed {self!r}"
        assert not self.opened_at, f"session already open {self!r}"

        # setup transaction
        self.opened_at = self._oracle.utc()
        self._session = self
        async with self._tx_lock:
            self._tx = Transaction(id=UUIDT(), session=self)

        # set context
        if self.parent is not None:
            self._default_scope = GraphScopeData(bench_id=uuid_to_str(self.parent.bench_id))
        if _set_in_context:
            self._active_session_tokens.append(ACTIVE_SESSION.set(self))

        # start flush loop
        self._commit_loop_task = asyncio.create_task(self._run_commit_loop())

        logger.trace("session.open", session=self)

    async def close(self):
        """Closes the session, rolling back uncommitted edits. Prevents further use."""
        assert self.opened_at, f"session not open {self!r}"
        assert not self.closed_at, f"session already closed {self!r}"

        # close transaction
        async with self._tx_lock:
            # stop commit loop
            if self._commit_loop_task is not None:
                self._commit_loop_task.cancel()
                with suppress(asyncio.CancelledError):
                    await self._commit_loop_task
                self._commit_loop_task = None
            # close connections/connectors
            for connection in self._connections:
                connection.close()
                await connection.wait_closed()
            self._connections.clear()
            for connector in self._connectors:
                await connector.close()
            self._connectors.clear()
            self._tx = None

        # close session
        self.closed_at = self._oracle.utc()
        self.duration = self.closed_at - self.opened_at
        for token in self._active_session_tokens:
            with suppress(ValueError):  # ignore error if token is from other context
                ACTIVE_SESSION.reset(token)
        self._active_session_tokens.clear()

        # remove dangling graph if this was a solo session
        if self._is_new:
            if len(self._graph) == 1:
                self._graph.supergraph.remove_graph(self._graph)
            elif self.id in self._graph:  # (may not have been added)
                self._graph.remove(self)

        logger.trace("session.close", session=self)

    #
    # Tracking
    #

    def _track(self, node: Node):
        """Start tracking the Node in this session."""
        if node._session != self:
            node._track_rec(self)

    def _track_many(self, *nodes: Node | None, force: bool = False):
        """Start tracking the Nodes in this session."""
        for n in nodes:
            if n is None:
                continue
            if force and n._session is not None:
                n._untrack_rec()
            if n._session is not self:
                n._track_rec(self)

    def _untrack(self, node: Node):
        """Stop tracking the Node in this session."""
        for n in node._walk_descendants():
            n._untrack_rec()

    def _untrack_many(self, *nodes: Node | None):
        """Stop tracking the Nodes in this session."""
        for n in nodes:
            if n is not None:
                self._untrack(n)

    #
    # Edits
    #

    def on_edit(self, node: Node, sub: Callable[[Node], None]) -> Callable[[], None]:
        """Subscribe to edits on a node."""
        if node.id not in self._on_edit_subs:
            self._on_edit_subs[node.id] = []
        self._on_edit_subs[node.id].append(sub)
        return lambda: self._unsubscribe_on_edit(node, sub)

    def _unsubscribe_on_edit(self, node: Node, sub: Callable[[Node], None]) -> None:
        """Unsubscribe from edits on a node."""
        if node.id in self._on_edit_subs:
            self._on_edit_subs[node.id].remove(sub)
            if not self._on_edit_subs[node.id]:
                del self._on_edit_subs[node.id]

    def _get_context(self) -> ContextData:
        """Gathers context valid for the entire session"""
        if self._context_data is None:
            context = ContextData(metatype=pb2.ObjectType.OBJECT_TYPE_CONTEXT)
            if self.client_ptr is not None:
                context.client_ptr.CopyFrom(self.client_ptr._to_data())
            if self.user_ptr is not None:
                context.user_ptr.CopyFrom(self.user_ptr._to_data())
            if self.computer_ptr is not None:
                context.computer_ptr.CopyFrom(self.computer_ptr._to_data())
            self._context_data = context
        return self._context_data

    def _create(self, node: Node):
        """Creates a new Node. The operation *is not* applied directly."""
        assert self._tx is not None, f"no active transaction for {node!r} in {self!r}"
        assert not self._is_suspended, f"cannot edit {node!r} in {self!r}"
        self._pending_nodes_by_id[node.id] = node
        if (
            isinstance(node, IsRuntime)
            and (runtime := self._runtime) is not None
            and any(
                isinstance(ancestor, IsRuntime) and ancestor.session_id == self.id
                for ancestor in node._walk_ancestors()
            )
        ):
            # 'inherit' runtime context on new runtime nodes
            runtime._set_session_context(node)
        self._tx.record_edit_event(EditType.CREATE, node)

    def _upsert(self, node: Node):
        """Creates or updates a Node. The operation *is not* applied directly."""
        assert self._tx is not None, f"no active transaction in {self!r}"
        assert not self._is_suspended, f"cannot edit {node!r} in {self!r}"
        now = self._oracle.utc()
        self._pending_nodes_by_id[node.id] = node
        self._tx.record_edit_event(EditType.UPSERT, node, now=now)

    def _update(
        self,
        node: Node,
        operation: EditOperationData | None = None,
    ):
        """Updates an existing Node. The operation *is not* applied directly."""
        assert self._tx is not None, f"no active transaction for {node!r} in {self!r}"
        assert not self._is_suspended, f"cannot edit {node!r} in {self!r}"
        self._pending_nodes_by_id[node.id] = node
        self._tx.record_edit_event(EditType.UPDATE, node, operation=operation)

    def _move(self, node: Node, old_parent: Node, new_parent: Node):
        """Moves a Node to a new parent. The operation *is not* applied directly."""
        assert self._tx is not None, f"no active transaction for {node!r} in {self!r}"
        assert not self._is_suspended, f"cannot edit {node!r} in {self!r}"
        from bench.language import pack_value
        from bench.proto import pack_proto_json

        parent_property = node.__parent_property__
        assert parent_property is not None, f"{node!r} has no parent property"
        parent_typ = parent_property._type
        assert parent_typ is not None, f"{parent_property!r} has no type info"

        self._pending_nodes_by_id[node.id] = node
        old_value_packed = pack_value(old_parent.to_ref(), parent_typ)
        new_value_packed = pack_value(new_parent.to_ref(), parent_typ)
        operation = EditOperationData(
            metatype=lang_pb2.OBJECT_TYPE_EDIT_OPERATION,
            type=lang_pb2.EDIT_OPERATION_TYPE_SET,  # type: ignore
            path=[parent_property.key],
            new_value_packed=pack_proto_json(new_value_packed),
            old_value_packed=pack_proto_json(old_value_packed),
        )
        self._tx.record_edit_event(EditType.MOVE, node, operation=operation)
        # also update any computed ancestor properties
        for prop in node.__node_ancestor_properties__.values():
            if prop.reference_source is not None:
                prop = prop.reference_source
            new_value = getattr(node, prop.name)
            if new_value is not None:
                assert isinstance(new_value, Node), f"bad {prop!r}: {new_value!r}"
                new_value_packed = pack_value(new_value, prop.type_info)  # type: ignore
            else:
                new_value_packed = None
            operation = EditOperationData(
                metatype=lang_pb2.OBJECT_TYPE_EDIT_OPERATION,
                type=lang_pb2.EDIT_OPERATION_TYPE_SET,  # type: ignore
                path=[prop.key],
                new_value_packed=pack_proto_json(new_value_packed),
            )
            self._tx.record_edit_event(EditType.UPDATE, node, operation=operation)

    def _archive(self, *nodes: Node, _now: datetime | None = None):
        """Archives a Node. The operation *is* applied directly."""
        assert self._tx is not None, f"no active transaction for {nodes!r} in {self!r}"
        assert not self._is_suspended, f"cannot edit {nodes!r} in {self!r}"

        for node in nodes:
            assert node.is_attached, f"cannot archive detached node {node!r}"
            now = _now if _now is not None else self._oracle.utc()
            self._pending_nodes_by_id[node.id] = node
            # descendants will be removed from graph, so remember them manually
            for descendant in node._graph.iter_descendants(node, recursive=True):
                self._pending_nodes_by_id[descendant.id] = descendant
            self._tx.record_edit_event(EditType.ARCHIVE, node, now=now)
            node.archived_at = now
            node._graph.remove(node)

    def _unarchive(self, *nodes: Node, _now: datetime | None = None):
        """Unarchives a Node. The operation *is* applied directly."""
        assert self._tx is not None, f"no active transaction for {nodes!r} in {self!r}"
        assert not self._is_suspended, f"cannot edit {nodes!r} in {self!r}"

        for node in nodes:
            assert node.is_attached, f"cannot unarchive detached node {node!r}"
            now = _now if _now is not None else self._oracle.utc()
            self._pending_nodes_by_id[node.id] = node
            self._tx.record_edit_event(EditType.UNARCHIVE, node, now=now)
            node.archived_at = None
            node._graph.add(node)

    def _delete(self, *nodes: Node, _now: datetime | None = None):
        """Deletes a Node. The operation *is* applied directly."""
        assert self._tx is not None, f"no active transaction for {nodes!r} in {self!r}"
        assert not self._is_suspended, f"cannot edit {nodes!r} in {self!r}"

        for node in nodes:
            assert node.is_attached, f"cannot delete detached node {node!r}"
            now = _now if _now is not None else self._oracle.utc()
            self._pending_nodes_by_id[node.id] = node
            # descendants will be removed from graph, so remember them manually
            for descendant in node._graph.iter_descendants(node, recursive=True):
                self._pending_nodes_by_id[descendant.id] = descendant
            self._tx.record_edit_event(EditType.DELETE, node, now=now)
            node.deleted_at = now
            node._graph.remove(node)

    def _restore(self, *nodes: Node, _now: datetime | None = None):
        """Restores a deleted Node. The operation *is* applied directly."""
        assert self._tx is not None, f"no active transaction for {nodes!r} in {self!r}"
        assert not self._is_suspended, f"cannot edit {nodes!r} in {self!r}"

        for node in nodes:
            assert node.is_attached, f"cannot restore detached node {node!r}"
            now = _now if _now is not None else self._oracle.utc()
            self._pending_nodes_by_id[node.id] = node
            self._tx.record_edit_event(EditType.RESTORE, node, now=now)
            node.deleted_at = None
            node._graph.add(node)

    def _erase(self, *nodes: Node):
        """Erases a Node. The operation *is* applied directly."""
        assert self._tx is not None, f"no active transaction for {nodes!r} in {self!r}"
        assert not self._is_suspended, f"cannot edit {nodes!r} in {self!r}"

        for node in nodes:
            assert node.is_attached, f"cannot erase detached node {node!r}"
            now = self._oracle.utc()
            self._pending_nodes_by_id[node.id] = node
            # descendants will be removed from graph, so remember them manually
            for descendant in node._graph.iter_descendants(node, recursive=True):
                self._pending_nodes_by_id[descendant.id] = descendant
            self._tx.record_edit_event(EditType.ERASE, node, now=now)
            node.deleted_at = now
            node._graph.remove(node)

    #
    # Transactions
    #

    def suspend(self):
        """Suspend the session, *erroring* on further edits."""
        self._is_suspended = True

    def unsuspend(self):
        """Stop suspending the session, allowing further edits."""
        self._is_suspended = False

    @asynccontextmanager
    async def active(self):
        """Activate this session in context (as active i.e. not suspended)."""
        was_suspended = self._is_suspended
        was_active = self._active_session_tokens is not None
        self.unsuspend()
        active_session_token = ACTIVE_SESSION.set(self)
        self._active_session_tokens.append(active_session_token)
        try:
            yield self
        finally:
            if was_suspended:
                self.suspend()
            elif not was_active and active_session_token in self._active_session_tokens:
                with suppress(ValueError):  # ignore error from bad token
                    ACTIVE_SESSION.reset(active_session_token)
                self._active_session_tokens.remove(active_session_token)

    async def _run_commit_loop(self):
        """Commits pending edits (on request) while the session is open."""
        while True:
            try:
                event = await self._commit_queue.get()
                assert self._tx is not None, f"no active transaction in {self!r}"
                if not self._tx.has_edits:
                    self._commit_queue.task_done()
                    logger.trace(
                        "session.queue.skip",
                        session=self,
                        e=event,
                        qsize=self._commit_queue.qsize(),
                    )
                    continue  # nothing to do
                _ = await self._do_commit()
                self._commit_queue.task_done()
                logger.trace(
                    "session.queue.tick", session=self, e=event, qsize=self._commit_queue.qsize()
                )
            except asyncio.CancelledError:
                if not self._commit_queue.empty():
                    logger.debug(
                        "session.queue.cancel",
                        session=self,
                        qsize=self._commit_queue.qsize(),
                    )
                break
            except BaseException as e:
                logger.error(
                    "session.queue.error",
                    session=self,
                    exc_info=e,
                    qsize=self._commit_queue.qsize(),
                )
                raise

    def _is_current_runtime_node(self, node: Node) -> bool:
        """Whether this is a runtime node tied to the current session."""
        if node.metatype == NodeType.SESSION:
            return self.id == node.id
        elif isinstance(node, IsRuntime):
            return node.session_id == self.id
        else:
            return False

    def _preflush(
        self, *, include_runtime: bool = True, exclude: Sequence[NodeType] = ()
    ) -> list[EditData]:
        """
        Creates an "edit boundary" by accumulating edit events & marking all nodes as 'flushed'.
        This means any new edits won't be debounced after this point (e.g. to create before update).
        Runtime nodes from the *current* runtime are not flushed.
        """
        assert self._tx is not None, f"no active transaction in {self!r}"

        def _filter(node: Node) -> bool:
            return (
                include_runtime or not self._is_current_runtime_node(node)
            ) and node.metatype not in exclude

        # return new_edits
        for node in self._pending_nodes_by_id.values():
            if node._is_new and _filter(node):
                node._is_new = False
        new_edits = self._tx.preflush(filter=lambda e: _filter(e.node))
        return new_edits

    @tracer.start_as_current_span("session.flush")
    @async_shield
    async def _do_flush(self) -> tuple[list[EditData], list[EditData]]:
        assert self.is_open, f"cannot commit {self!r} when closed"
        assert self._tx is not None, f"no active transaction in {self!r}"
        try:
            self._preflush()
            async with self._tx_lock:
                edits, cascaded_edits = await self._tx.flush()
                logger.trace(
                    "session.flush",
                    session=self,
                    edits=len(edits),
                    cascaded_edits=len(cascaded_edits),
                    span="current",
                )
                return edits, cascaded_edits
        except EngineUnavailableError as e:
            logger.error("session.flush.error", session=self, error=e)
            self._tx.reset()
            raise

    def _make_pending_graph(self) -> NodeGraph:
        node_types = {node.metatype for node in self._pending_nodes_by_id.values()}
        descendant_node_types = set()  # include descendants for cascading edits
        for node_type in node_types:
            descendant_node_types.update(DESCENDANT_NODE_TYPES[node_type])
        node_types = node_types | descendant_node_types
        graph = NodeGraph(
            scope=self._default_scope,
            node_types=bittuple(*node_types, enum_cls=NodeType),
            nodes=self._pending_nodes_by_id.values(),
            supergraph=self._supergraph,
        )
        self._supergraph.add_graph(graph)  # cleaned up in _do_commit
        return graph

    def _make_pending_data_graph(self) -> NodeDataGraph:
        """Get graphs with all the pending nodes."""
        node_types = {node.metatype for node in self._pending_nodes_by_id.values()}
        data_graph = NodeDataGraph(
            scope=self._default_scope, node_types=bittuple(*node_types, enum_cls=NodeType)
        )
        for node in self._pending_nodes_by_id.values():
            data_graph.add(node._to_data())
        return data_graph

    @tracer.start_as_current_span("session.commit")
    @async_shield
    async def _do_commit(
        self, *, data_graph: NodeDataGraph | None = None
    ) -> tuple[list[EditData], list[EditData]]:
        """Attempt to commit pending edits."""
        from bench.proto.wiring import unwrap_some_node

        # TODO :Architecture :Cleanup: revamp Session handling across internal, system and remote
        #  It feels quite clumsy and mixes concerns (why are we talking about TablePlugin here?);
        #   and in general, we should probably pull apart system and runtime Sessions somehow
        #  Maybe Session should remain a Node, but the Session logic goes elsewhere...?

        assert self._tx is not None, f"no active transaction in {self!r}"

        log = logger.bind(session=self, span="current")

        pending_graphs: list[NodeGraph] = []
        try:
            async with self._tx_lock:
                assert self._tx is not None, f"no active transaction in {self!r}"
                # prepare commit
                if self._pre_commit is not None:
                    # NOTE :Architecture: we exclude Records from preflush in commit prepare
                    #  because our TablePlugin needs to update schemas before touching any Records.
                    excluded_node_types = (NodeType.RECORD,)
                    self._preflush(exclude=excluded_node_types)
                    edits, cascaded_edits = await self._tx.flush(
                        filter=lambda e: NodeType(e.node_ptr.node_type) not in excluded_node_types
                    )
                    graph = self._make_pending_graph()
                    pending_graphs.append(graph)
                    if data_graph is None:
                        data_graph = self._make_pending_data_graph()
                    await self._pre_commit(self, graph, data_graph, edits, cascaded_edits)

                # do commit
                self._preflush()
                edits, cascaded_edits = await self._tx.commit()
                log = log.bind(edits=len(edits), cascaded_edits=len(cascaded_edits))

            # on commit hook
            if self._post_commit is not None:
                graph = self._make_pending_graph()
                pending_graphs.append(graph)
                if data_graph is None:
                    data_graph = self._make_pending_data_graph()

                # update data graph from cascaded edits
                for edit in cascaded_edits:
                    if edit.HasField("node_data"):
                        node_data = unwrap_some_node(edit.node_data)
                        if node_data.id in data_graph:
                            data_graph.update(node_data)

                await self._post_commit(self, graph, data_graph, edits, cascaded_edits)
                self._pending_nodes_by_id = {}

            log.trace("session.commit")
            return edits, cascaded_edits
        except BaseException as e:
            if self._post_commit_failed is not None:
                await self._post_commit_failed(self, e)
            if isinstance(e, EngineUnavailableError):
                logger.error("session.commit.error", error=e, connectors=self._connectors)
                for connector in self._connectors:
                    await connector.reset()
                self._tx.reset()
            raise
        finally:
            for graph in pending_graphs:
                self._supergraph.remove_graph(graph)

    @tracer.start_as_current_span("session.stage")
    def stage(self, *, include_runtime: bool = False):
        """Stage pending edits without waiting for the next background commit."""
        # schedule a new commit
        assert self.is_open, f"cannot commit {self!r} when closed"
        assert self._tx is not None, f"no active transaction in {self!r}"
        new_edits = self._preflush(include_runtime=include_runtime)
        event = _CommitEvent(id=self._flush_counter, new_edits=new_edits)
        if not self._tx.has_edits:
            return [], []  # nothing to do
        self._commit_queue.put_nowait(event)
        logger.trace("session.stage", session=self, e=event, span="current")
        return event.new_edits, []

    @tracer.start_as_current_span("session.flush.schedule")
    async def flush(self) -> tuple[list[EditData], list[EditData]]:
        """
        Flushes the current pending edits.
        Cascaded edits are only returned for non-optimistic flushes.
        """
        assert self.is_open, f"cannot flush {self!r} when closed"
        assert self._tx is not None, f"no active transaction in {self!r}"

        with tracer.start_as_current_span("session.flush.wait"):
            await self._commit_queue.join()  # wait for any pending commit
        if not self._tx.has_pending_edits:
            return [], []  # nothing to do
        return await self._do_flush()

    @tracer.start_as_current_span("session.commit.schedule")
    async def commit(
        self,
        *,
        _data_graph: NodeDataGraph | None = None,
        _ignore_open: bool = False,
    ) -> tuple[list[EditData], list[EditData]]:
        """
        Commits all edits. Returns *all* edits & cascaded edits. Resets tx state.
        If optimistic, we schedule a new commit and return immediately.
        If not optimistic, we wait for any pending commit to complete, then commit.
        Cascaded edits are only returned for non-optimistic commits.
        """
        if self._tx is None or (not self._tx.has_edits and not self._tx._touched_engine_ids):
            return [], []  # nothing to do

        assert self.is_open or _ignore_open, f"cannot commit {self!r} when closed"

        # wait for any pending commit, then commit directly
        with tracer.start_as_current_span("session.commit.wait"):
            await self._commit_queue.join()
        if not self._tx.has_edits and not self._tx._touched_engine_ids:
            return [], []  # nothing to do
        new_edits, cascaded_edits = await self._do_commit(data_graph=_data_graph)
        return new_edits, cascaded_edits

    async def __aenter__(self):
        await self.open()
        return self

    async def __aexit__(self, exc_type, exc, tb):
        await self.close()
