import asyncio
import contextvars
from contextlib import asynccontextmanager, suppress
from datetime import datetime
from typing import (
    TYPE_CHECKING,
    Awaitable,
    Callable,
    Optional,
    Sequence,
)

import structlog
from attr import dataclass
from fastuuid import UUID, uuid4
from opentelemetry import trace

from bench import pb2
from bench.language.registry import DESCENDANT_NODE_TYPES
from bench.pb2 import (
    ClientOriginData,
    ContextData,
    EditData,
    EditOperationData,
    GraphScopeData,
    HostClient,
    RpcMetadata,
    SupervisorClient,
    lang_pb2,
)
from bench.utils.func import uuid_to_str
from bench.utils.oracle import Oracle
from bench.utils.sync import CriticalLock

from .const import (
    ACTIVE_SESSION,
    EditType,
    NodeMode,
    NodeType,
    bittuple,
)
from .graph import Graph, GraphData, Supergraph
from .node import BenchNode, Node, PackageNode, Subject
from .object import EMPTY_SCOPE_DATA
from .property import p_node_parent, p_runtime
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
    ["Session", Graph, GraphData, Sequence["EditData"], Sequence["EditData"]],
    Awaitable[None],
]
CommitHook = Callable[
    [
        "Session",
        Graph,
        GraphData,
        Sequence["EditData"],
        Sequence["EditData"],
    ],
    Awaitable[None],
]
CommitFailedHook = Callable[["Session", BaseException], Awaitable[None]]


class Session:
    """
    A managed Session for interacting with and running a Bench.
    """

    parent: Optional["Bench"] = p_node_parent(4, NodeType.BENCH, is_system=True)

    # node
    mode: NodeMode
    bench: Optional["Bench"]
    _supergraph: Supergraph

    # context
    # ...HasRuntimeContext[80-99]

    # context
    _origin: ClientOriginData | None = p_runtime(default=None)
    _subject: Subject | None = p_runtime(default=None)
    _context_data: ContextData | None = p_runtime(default=None)
    _is_suspended: bool = p_runtime(default=False)

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

    async def open(self, *, _set_in_context: bool = True):
        """Opens the session for regular business. Activates context (by default)."""
        assert not self.closed_at, f"session already closed {self!r}"
        assert not self.opened_at, f"session already open {self!r}"

        # setup transaction
        self.opened_at = self._oracle.utc()
        self._session = self
        async with self._tx_lock:
            self._tx = Transaction(id=uuid4(), session=self)

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
            self._tx = None

        # close session
        self.closed_at = self._oracle.utc()
        self.duration = self.closed_at - self.opened_at
        for token in self._active_session_tokens:
            with suppress(ValueError):  # ignore error if token is from other context
                ACTIVE_SESSION.reset(token)
        self._active_session_tokens.clear()

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

    def _create(self, node: Node):
        """Creates a new Node. The operation *is not* applied directly."""
        assert self._tx is not None, f"no active transaction for {node!r} in {self!r}"
        assert not self._is_suspended, f"cannot edit {node!r} in {self!r}"
        self._pending_nodes_by_id[node.id] = node
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
        new_value_packed = pack_value(new_parent.to_ref(), parent_typ)
        operation = EditOperationData(
            metatype=lang_pb2.OBJECT_TYPE_EDIT_OPERATION,
            type=lang_pb2.EDIT_OPERATION_TYPE_SET,  # type: ignore
            path=[parent_property.key],
            new_value_packed=pack_proto_json(new_value_packed),
        )
        self._tx.record_edit_event(EditType.MOVE, node, operation=operation)
        # also update any computed ancestor properties
        for prop in node.__node_ancestor_properties__.values():
            if prop.runtime_prop is not None:
                prop = prop.runtime_prop
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

    def _make_pending_graph(self) -> Graph:
        node_types = {node.metatype for node in self._pending_nodes_by_id.values()}
        descendant_node_types = set()  # include descendants for cascading edits
        for node_type in node_types:
            descendant_node_types.update(DESCENDANT_NODE_TYPES[node_type])
        node_types = node_types | descendant_node_types
        graph = Graph(
            scope=self._default_scope,
            node_types=bittuple(*node_types, enum_cls=NodeType),
            nodes=self._pending_nodes_by_id.values(),
            supergraph=self._supergraph,
        )
        self._supergraph.add_graph(graph)  # cleaned up in _do_commit
        return graph

    def _make_pending_data_graph(self) -> GraphData:
        """Get graphs with all the pending nodes."""
        node_types = {node.metatype for node in self._pending_nodes_by_id.values()}
        data_graph = GraphData(
            scope=self._default_scope, node_types=bittuple(*node_types, enum_cls=NodeType)
        )
        for node in self._pending_nodes_by_id.values():
            data_graph.add(node._to_data())
        return data_graph

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
        _data_graph: GraphData | None = None,
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
