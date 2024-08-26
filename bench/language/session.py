import asyncio
import contextvars
from contextlib import asynccontextmanager, suppress
from datetime import datetime
from typing import (
    TYPE_CHECKING,
    Any,
    Awaitable,
    Callable,
    Collection,
    Iterable,
    Optional,
    Sequence,
)
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language.connection import (
    Channel,
    ChannelUnavailableError,
    Connection,
    GraphEngine,
    MemoryEngine,
    NullEngine,
    SplitChannel,
    scope_includes,
)
from bench.language.const import (
    NODE_TYPES,
    BenchError,
    BlockType,
    NodeType,
    SessionStatus,
    StructType,
    _active_session,
)
from bench.language.node import (
    EMPTY_SCOPE,
    BenchNode,
    BuiltinObject,
    EditSubject,
    HasTimeIdentity,
    Node,
    NodeReferenceBase,
    PackageNode,
    Struct,
    object_,
    struct_,
    timed_node_,
)
from bench.language.property import Property, p_internal, p_node_parent, p_runtime, p_system
from bench.language.transaction import Transaction
from bench.language.validation import constraint
from bench.proto import wire
from bench.proto.wire import (
    ClientOriginData,
    EditContextData,
    EditData,
    GraphScopeData,
    HostClient,
    NodeReferenceData,
    RpcMetadata,
    SessionContextData,
    SessionData,
    SupervisorClient,
)
from bench.utils.func import CriticalLock, uuid_to_str
from bench.utils.oracle import Oracle
from bench.utils.uuidt import UUIDT

if TYPE_CHECKING:
    from bench.language import (
        Block,
        Client,
        Machine,
        NodeReference,
        Package,
        QueryBuilder,
        Run,
        Server,
        Step,
        User,
    )
    from bench.runtime.runtime import Runtime

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

CustomCommit = Callable[["Session"], Awaitable[tuple[list[EditData], list[EditData]]]]


@timed_node_(NodeType.SESSION)
class Session(PackageNode[SessionData], HasTimeIdentity):
    """
    A managed Session for interacting with and running a Bench in a Client.
    If a Run spans multiple Clients, each Client will have its own Session.
    On some Clients a Session may persist across Runs (like in the web client).
    Once closed, a Session (like a Run) is effectively immutable.
    """

    parent: "Package | None" = p_node_parent(4, NodeType.PACKAGE, is_system=True)

    # status
    status: SessionStatus = p_system(40, default=SessionStatus.PENDING, index_in_pg=True)
    duration: Optional[float] = p_system(41, default=None)
    opened_at: Optional[datetime] = p_system(42, default=None)
    closed_at: Optional[datetime] = p_system(43, default=None)

    # context
    client: Optional["Client"] = p_internal(
        61, require=False, array=False, references=NodeType.CLIENT
    )
    server: Optional["Server"] = p_internal(
        62, require=False, array=False, references=NodeType.SERVER
    )
    machine: Optional["Machine"] = p_internal(
        63, require=False, array=False, references=NodeType.MACHINE
    )
    user: Optional["User"] = p_internal(64, require=False, array=False, references=NodeType.USER)
    if TYPE_CHECKING:
        client_ptr: Optional[NodeReference] = None
        server_ptr: Optional[NodeReference] = None
        machine_ptr: Optional[NodeReference] = None
        user_ptr: Optional[NodeReference] = None

    # flags
    _is_readonly: bool = p_runtime(default=False)
    _is_suspended: bool = p_runtime(default=False)

    # transaction
    _split_read: bool = p_runtime(default=False)
    _split_read_channel: Channel | None = p_runtime(default=None)
    _engines: tuple["GraphEngine", ...] = p_runtime(default_factory=tuple)
    _channels: list[Channel] = p_runtime(default_factory=list)
    _connections: list[Connection] = p_runtime(default_factory=list)
    _origin: ClientOriginData | None = p_runtime(default=None)
    _subject: EditSubject | None = p_runtime(default=None)
    _tx: Transaction | None = p_runtime(default=None)
    _tx_lock: asyncio.Lock = p_runtime(default_factory=lambda: CriticalLock(name="session"))
    _edited_nodes_by_id: dict[UUID, Node] = p_runtime(default_factory=dict)
    _default_scope: GraphScopeData = p_runtime(default_factory=lambda: EMPTY_SCOPE._to_data())

    # in-system transaction
    _system_epoch: int | None = p_runtime(default=None)
    _on_error: Callable[[Exception], None] | None = p_runtime(default=None)
    _custom_commit: CustomCommit | None = p_runtime(default=None)

    # runtime
    _oracle: Oracle = p_runtime()
    _active_session_token: contextvars.Token | None = p_runtime(default=None)
    _rpc_metadata: RpcMetadata | None = p_runtime(default=None)
    _rpc_headers: dict[str, str] | None = p_runtime(default=None)
    _runtime: Optional["Runtime"] = p_runtime(default=None)
    _supervisor: Optional["SupervisorClient"] = p_runtime(default=None)
    _host: Optional["HostClient"] = p_runtime(default=None)

    def __content_str__(self):
        status_strs = []
        if self.closed_at:
            status_strs.append("closed")
        elif self.opened_at:
            status_strs.append("open")
        else:
            status_strs.append("declared")
        if self._is_readonly:
            status_strs.append("readonly")
        if self._is_suspended:
            status_strs.append("suspended")
        if self.duration is not None:
            return f"{', '.join(status_strs)}, tx={self._tx or '<no tx>'}, duration={self.duration:.3f}s"
        else:
            return f"{', '.join(status_strs)}, tx={self._tx or '<no tx>'}"

    @property
    def epoch(self) -> int:
        return max((*(c.epoch for c in self._connections), -1))

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
    def is_closed(self) -> bool:
        return self.closed_at is not None

    @property
    def is_active(self):
        return self._active_session_token is not None

    @property
    def is_suspended(self):
        return self._is_suspended

    @property
    def runtime(self) -> "Runtime":
        assert self._runtime is not None, f"no runner in {self!r}"
        return self._runtime

    @property
    def host(self) -> HostClient:
        assert self._host is not None, f"no host in {self!r}"
        return self._host

    @property
    def supervisor(self) -> SupervisorClient:
        assert self._supervisor is not None, f"no supervisor in {self!r}"
        return self._supervisor

    def _get_scope_for_node(self, n: Node) -> GraphScopeData:
        """Get the scope for a node in this session."""
        scope = GraphScopeData(metatype=wire.ObjectType.GRAPH_SCOPE)
        if isinstance(n, BenchNode):
            scope.bench_id = uuid_to_str(n.bench_id) or self._default_scope.bench_id
        if isinstance(n, PackageNode):
            scope.package_id = uuid_to_str(n.package_id) or self._default_scope.package_id
        return scope

    def _get_scope_for_node_ptr(self, ptr: NodeReferenceBase) -> GraphScopeData:
        """Get the scope for a node pointer in this session."""
        scope = GraphScopeData(metatype=wire.ObjectType.GRAPH_SCOPE)
        if ptr.bench_id is not None:
            scope.bench_id = uuid_to_str(ptr.bench_id)
        return scope

    def _get_scope_for_query(self, query: "QueryBuilder") -> GraphScopeData:
        """Get the scope for a query in this session."""
        if query._base is not None:
            return self._get_scope_for_node(query._base)
        else:
            return self._default_scope

    def _get_engine_for(
        self,
        scope: GraphScopeData,
        node_types: NodeType | Iterable[NodeType],
        *,
        is_readonly: bool,
        include_hidden: bool,
        best_match: Collection[NodeType] | None = None,
    ) -> GraphEngine:
        """Gets the appropriate engine"""
        node_types = (node_types,) if isinstance(node_types, NodeType) else tuple(node_types)
        candidate_engines = [
            engine
            for engine in self._engines
            if (
                (is_readonly or not engine.is_readonly)
                and (not include_hidden or engine.includes_hidden)
                and scope_includes(engine.scope, scope)
                and all(t in engine.node_types for t in node_types)
            )
        ]
        if not candidate_engines:
            raise BenchError(
                f"no engine for [scope={scope!r}, node_types={'|'.join(t.bench_name for t in node_types)}] in {self!r}"
                f" (engines: {self._engines!r})"
            )
        if best_match is None or len(candidate_engines) < 2:
            return candidate_engines[0]
        else:
            # try to find best match (most type overlap, best first)
            candidate_engines.sort(key=lambda e: -len([t for t in best_match if t in e.node_types]))
            # prefer in-memory engines
            for e in candidate_engines:
                if isinstance(e, MemoryEngine):
                    return e
            return candidate_engines[0]

    async def _get_channel(self, engine: GraphEngine) -> Channel:
        """Gets or creates a channel"""
        for channel in self._channels:
            if channel.engine.id == engine.id:
                return channel
        else:
            channel = await engine.connect(self)
            self._channels.append(channel)
            return channel

    async def _get_channel_for(
        self,
        scope: GraphScopeData,
        node_types: NodeType | Iterable[NodeType],
        *,
        is_readonly: bool,
        include_hidden: bool,
        best_match: Collection[NodeType] | None = None,
    ) -> Channel:
        """Gets or creates a store channel for a scope and node types."""
        if is_readonly and self._split_read:
            if self._split_read_channel is None:
                self._split_read_channel = SplitChannel(
                    NullEngine(self._default_scope, NODE_TYPES), self
                )
            return self._split_read_channel
        else:
            engine = self._get_engine_for(
                scope=scope,
                node_types=node_types,
                is_readonly=is_readonly,
                include_hidden=include_hidden,
                best_match=best_match,
            )
            return await self._get_channel(engine)

    def _on_connection_begin(self, connection: Connection):
        """Called when a connection begins."""
        self._connections.append(connection)

    def _on_connection_end(self, connection: Connection):
        """Called when a connection ends."""
        self._connections.remove(connection)

    async def open(self, *, set_in_context: bool = True):
        """Opens the session for regular business. Activates context (by default)."""
        assert not self.closed_at, f"session already closed {self!r}"
        assert not self.opened_at, f"session already open {self!r}"
        async with self._tx_lock:
            self._tx = Transaction(id=UUIDT(), session=self, is_readonly=self._is_readonly)
        self.opened_at = self._oracle.utc()
        self._session = self
        if self.parent is not None:
            self._default_scope = GraphScopeData(
                bench_id=uuid_to_str(self.parent.bench_id), package_id=uuid_to_str(self.parent.id)
            )
        if set_in_context:
            self._active_session_token = _active_session.set(self)
        logger.trace("session.open", session=self)

    async def close(self, _suppress_error: bool = False):
        """Closes the session, rolling back uncommitted edits. Prevents further use."""
        assert self.opened_at, f"session not open {self!r}"
        assert not self.closed_at, f"session already closed {self!r}"

        # close connections
        for connection in self._connections:
            connection.close()

        # close transaction
        async with self._tx_lock:
            await asyncio.gather(
                *(channel.close() for channel in self._channels), return_exceptions=_suppress_error
            )
            self._channels.clear()
            self._tx = None

        # close session
        self.closed_at = self._oracle.utc()
        self.duration = (self.closed_at - self.opened_at).total_seconds()
        if self._active_session_token is not None:
            with suppress(ValueError):  # ignore error if token is from other context
                _active_session.reset(self._active_session_token)
                self._active_session_token = None
        # remove dangling graph if this was a solo session
        # NOTE :Cleanup: not sure how to prune graphs from temporary objects like request sessions :TransientGraphs
        if self._is_new:
            if len(self._graph) == 1:
                self._graph.supergraph.remove_graph(self._graph)
            elif self.id in self._graph:  # (may not have been added)
                self._graph.remove(self)

        logger.trace("session.close", session=self)

    # NOTE :Cleanup :Robustness: Session suspend/unsuspend is pretty clumsy
    #  (also getting occassional 'ContextVar was created in a different context' errors...)

    def suspend(self):
        """Suspend the session, *erroring* on further edits."""
        self._is_suspended = True
        self._active_session_token = None

    def unsuspend(self):
        """Stop suspending the session, allowing further edits."""
        self._is_suspended = False

    @asynccontextmanager
    async def active(self, readonly: bool = False):
        """Activate this session in context (as active i.e. not suspended)."""
        was_readonly = self._is_readonly
        was_suspended = self._is_suspended
        was_active = self._active_session_token is not None
        self._is_readonly = readonly
        self.unsuspend()
        self._active_session_token = _active_session.set(self)
        try:
            yield self
        finally:
            if was_suspended:
                self.suspend()
            elif not was_active and self._active_session_token is not None:
                _active_session.reset(self._active_session_token)
                self._active_session_token = None
            self._is_readonly = was_readonly

    # TODO :Robustness!: auto re-connect Session.flush/commit/...? on error
    #  (need to replay all previous edits, maybe do some other stuff?)

    @tracer.start_as_current_span("session.flush")
    async def flush(
        self, *, _skip_lock: bool = False, _extra_edits: list[EditData] | None = None
    ) -> tuple[list[EditData], list[EditData]]:
        """Flushes the current pending edits. Returns *all* uncommitted edits / cascaded edits."""
        assert self.is_open, f"cannot flush {self!r} when closed"
        assert self._tx is not None, f"no active transaction in {self!r}"

        try:
            if not _skip_lock:
                await self._tx_lock.acquire()
            await self._tx.flush(_extra_edits=_extra_edits)
            return self._tx._edits, self._tx._cascaded_edits
        except ChannelUnavailableError as e:
            logger.error("session.flush.error", session=self, error=e)
            await self._tx.reset()
            raise
        finally:
            if not _skip_lock:
                self._tx_lock.release()

    @tracer.start_as_current_span("session.flush")
    async def commit(
        self, *, _skip_lock: bool = False, _extra_edits: list[EditData] | None = None
    ) -> tuple[list[EditData], list[EditData]]:
        """Commits all edits. Returns *all* committed edits / cascaded edits *and* resets them."""
        assert self.is_open, f"cannot commit {self!r} when closed"
        assert self._tx is not None, f"no active transaction in {self!r}"

        if not self._tx.has_edits:
            return [], []  # nothing to do

        # TODO :Robustness :Broken: rollback edits to in-memory Nodes on session commit error
        try:
            if not _skip_lock:
                await self._tx_lock.acquire()
            if self._custom_commit is None:
                # simple commit
                return await self._tx.commit(_extra_edits=_extra_edits)
            else:
                # custom commit (in system)
                return await self._custom_commit(self)
        except ChannelUnavailableError as e:
            logger.error("session.commit.error", session=self, error=e)
            await self._tx.reset()
            raise
        finally:
            if not _skip_lock:
                self._tx_lock.release()

    async def __aenter__(self):
        await self.open()
        return self

    async def __aexit__(self, exc_type, exc, tb):
        await self.close(_suppress_error=exc is None)

    #
    # Tracking
    #

    def track(self, node: Node):
        """Start tracking the node in this session."""
        if node._session != self:
            node._track_rec(self)

    def track_many(self, *nodes: Node | None):
        """Start tracking the nodes in this session."""
        for n in nodes:
            if n is not None and n._session is not self:
                n._track_rec(self)

    def untrack(self, node: Node):
        """Stop tracking the node in this session."""
        for n in node._walk_descendants():
            n._untrack_self()
            if n.id in self._edited_nodes_by_id:
                del self._edited_nodes_by_id[n.id]

    def untrack_many(self, *nodes: Node | None):
        """Stop tracking the nodes in this session."""
        for n in nodes:
            if n is not None:
                self.untrack(n)

    #
    # Transaction
    #

    def _get_context(self) -> SessionContextData:
        """Gathers context valid for the entire session"""
        context = SessionContextData(metatype=wire.ObjectType.SESSION_CONTEXT)
        if self.client_ptr is not None:
            context.client_ptr = self.client_ptr._to_data()
        if self.machine_ptr is not None:
            context.machine_ptr = self.machine_ptr._to_data()
        if self.server_ptr is not None:
            context.server_ptr = self.server_ptr._to_data()
        if self.user_ptr is not None:
            context.user_ptr = self.user_ptr._to_data()
        return context

    def _get_edit_context(self) -> tuple[NodeReferenceData | None, EditContextData | None]:
        """Gathers current context for a specific edit"""
        # NOTE :Performance: gathering the context for every edit seems a bit expensive?
        # but it could change..

        # if we have an active run, that's the subject
        run = self._runtime.active_run if self._runtime is not None else None
        if run is not None:
            # if run has a step/block, use that
            if run.step_ptr:
                subject = run.step
            elif run.block_ptr:
                subject = run.block
            else:
                subject = run
            # if subject has an identity, use that
            if subject is not None and subject.identity_ptr:
                subject = subject.identity
        else:
            subject = self._subject
        if subject is None:
            return None, None

        # map into edit-specific context
        subject_ptr = subject._to_plain_ref_data()
        context = EditContextData(metatype=wire.ObjectType.EDIT_CONTEXT)
        if self.client_ptr is not None:
            context.client_ptr = self.client_ptr._to_data()
        if self.machine_ptr is not None:
            context.machine_ptr = self.machine_ptr._to_data()
        if self.server_ptr is not None:
            context.server_ptr = self.server_ptr._to_data()
        if self.user_ptr is not None:
            context.user_ptr = self.user_ptr._to_data()
        if run is not None:
            context.run_ptr = subject_ptr
            context.run_root_ptr = run.root_ptr._to_data() if run.root_ptr is not None else None
            context.block_ptr = run.block_ptr._to_data() if run.block_ptr is not None else None
            context.step_ptr = run.step_ptr._to_data() if run.step_ptr is not None else None
            context.identity_ptr = (
                run.identity_ptr._to_data() if run.identity_ptr is not None else None
            )

        return subject_ptr, context

    def _create(self, *nodes: Node):
        """Creates a new node. Errors if the node already exists."""
        assert self._tx is not None, f"no active transaction for {nodes!r} in {self!r}"
        assert (
            not self._is_readonly and not self._is_suspended
        ), f"cannot edit {nodes!r} in {self!r}"
        subject, context = self._get_edit_context()
        for node in nodes:
            if node._is_attached:  # ignore detached create (is created on attach)
                self._edited_nodes_by_id[node.id] = node
                self._tx.create(node, subject, self._origin, context, self._oracle.utc())
                node._is_new = False

    def _upsert(self, *nodes: Node):
        """Creates or updates a node. Any non-id properties will be overwritten."""
        assert self._tx is not None, f"no active transaction in {self!r}"
        assert (
            not self._is_readonly and not self._is_suspended
        ), f"cannot edit {nodes!r} in {self!r}"
        subject, context = self._get_edit_context()
        for node in nodes:
            assert node._is_attached, f"cannot upsert detached node {node!r}"
            self._edited_nodes_by_id[node.id] = node
            self._tx.upsert(node, subject, self._origin, context, self._oracle.utc())

    def _update(self, node: Node, properties: Collection[Property], old_values: dict[int, Any]):
        """Updates an existing node. Cannot move. The given properties are overwritten."""
        assert self._tx is not None, f"no active transaction for {node!r} in {self!r}"
        assert not self._is_readonly and not self._is_suspended, f"cannot edit {node!r} in {self!r}"
        if node._is_attached:  # ignore detached updates
            self._edited_nodes_by_id[node.id] = node
            subject, context = self._get_edit_context()
            self._tx.update(
                node, subject, self._origin, context, properties, old_values, self._oracle.utc()
            )

    def _move(self, node: Node, properties: Collection[Property], old_values: dict[int, Any]):
        """Moves and updates an existing node."""
        assert self._tx is not None, f"no active transaction for {node!r} in {self!r}"
        assert not self._is_readonly and not self._is_suspended, f"cannot edit {node!r} in {self!r}"
        if node._is_attached:  # ignore detached moves
            self._edited_nodes_by_id[node.id] = node
            subject, context = self._get_edit_context()
            self._tx.move(
                node, subject, self._origin, context, properties, old_values, self._oracle.utc()
            )

    def _archive(self, *nodes: Node):
        """Marks a node as archived, so it will be hidden by default."""
        assert self._tx is not None, f"no active transaction for {nodes!r} in {self!r}"
        assert (
            not self._is_readonly and not self._is_suspended
        ), f"cannot edit {nodes!r} in {self!r}"
        subject, context = self._get_edit_context()
        for node in nodes:
            assert node._is_attached, f"cannot archive detached node {node!r}"
            now = self._oracle.utc()
            self._edited_nodes_by_id[node.id] = node
            # descendants will be removed from graph, so track them manually
            for descendant in node._graph.iter_descendants(node, recursive=True):
                self._edited_nodes_by_id[descendant.id] = descendant
            self._tx.archive(node, subject, self._origin, context, now)
            node.archived_at = now
            node._graph.remove(node)

    def _unarchive(self, *nodes: Node):
        """Restore a node from the archive in its original place."""
        assert self._tx is not None, f"no active transaction for {nodes!r} in {self!r}"
        assert (
            not self._is_readonly and not self._is_suspended
        ), f"cannot edit {nodes!r} in {self!r}"
        subject, context = self._get_edit_context()
        for node in nodes:
            assert node._is_attached, f"cannot unarchive detached node {node!r}"
            self._edited_nodes_by_id[node.id] = node
            self._tx.unarchive(node, subject, self._origin, context, self._oracle.utc())
            node.archived_at = None

    def _delete(self, *nodes: Node):
        """Deletes a node with the option to recover it for a limited time."""
        assert self._tx is not None, f"no active transaction for {nodes!r} in {self!r}"
        assert (
            not self._is_readonly and not self._is_suspended
        ), f"cannot edit {nodes!r} in {self!r}"
        subject, context = self._get_edit_context()
        for node in nodes:
            assert node._is_attached, f"cannot delete detached node {node!r}"
            now = self._oracle.utc()
            self._edited_nodes_by_id[node.id] = node
            # descendants will be removed from graph, so track them manually
            for descendant in node._graph.iter_descendants(node, recursive=True):
                self._edited_nodes_by_id[descendant.id] = descendant
            self._tx.delete(node, subject, self._origin, context, now)
            node.deleted_at = now
            node._graph.remove(node)

    def _restore(self, *nodes: Node):
        """Restore a deleted node."""
        assert self._tx is not None, f"no active transaction for {nodes!r} in {self!r}"
        assert (
            not self._is_readonly and not self._is_suspended
        ), f"cannot edit {nodes!r} in {self!r}"
        subject, context = self._get_edit_context()
        for node in nodes:
            assert node._is_attached, f"cannot restore detached node {node!r}"
            now = self._oracle.utc()
            self._edited_nodes_by_id[node.id] = node
            self._tx.restore(node, subject, self._origin, context, now)
            node.deleted_at = None

    def _erase(self, *nodes: Node):
        """Irreversibly wipe a node and its descendants from the graph."""
        assert self._tx is not None, f"no active transaction for {nodes!r} in {self!r}"
        assert (
            not self._is_readonly and not self._is_suspended
        ), f"cannot edit {nodes!r} in {self!r}"
        subject, context = self._get_edit_context()
        for node in nodes:
            assert node._is_attached, f"cannot erase detached node {node!r}"
            now = self._oracle.utc()
            self._edited_nodes_by_id[node.id] = node
            # descendants will be removed from graph, so track them manually
            for descendant in node._graph.iter_descendants(node, recursive=True):
                self._edited_nodes_by_id[descendant.id] = descendant
            self._tx.erase(node, subject, self._origin, context, now)
            node.deleted_at = now
            node._graph.remove(node)


@struct_(StructType.EDIT_CONTEXT)
class EditContext(Struct):
    """Additional context for a specific edit (per-edit variable subset of Session context)."""

    block: Optional["Block"] = p_internal(70, require=False, array=False, references=NodeType.BLOCK)
    step: Optional["Step"] = p_internal(71, require=False, array=False, references=NodeType.STEP)
    session: Optional["Session"] = p_internal(
        72, require=False, array=False, references=NodeType.SESSION, same_bench=True
    )
    run: Optional["Run"] = p_internal(
        73, require=False, array=False, references=NodeType.RUN, same_bench=True
    )
    run_root: Optional["Run"] = p_internal(
        74, require=False, array=False, references=NodeType.RUN, same_bench=True
    )
    identity: Optional["Block"] = p_internal(
        75, require=False, array=False, references=NodeType.BLOCK
    )
    if TYPE_CHECKING:
        block_ptr: Optional[NodeReference] = None
        step_ptr: Optional[NodeReference] = None
        session_ptr: Optional[NodeReference] = None
        run_ptr: Optional[NodeReference] = None
        run_root_ptr: Optional[NodeReference] = None
        identity_ptr: Optional[NodeReference] = None


@object_()
class HasSessionContext(BuiltinObject):
    """Context for the creation of a node in some Session."""

    # NOTE :Security: session context properties are p_internal, not p_system so we can update
    #   them in all clients. But this also means users can mess with them if they really want to.
    # :SessionContext
    block: Optional["Block"] = p_internal(70, require=False, array=False, references=NodeType.BLOCK)
    step: Optional["Step"] = p_internal(71, require=False, array=False, references=NodeType.STEP)
    session: Optional["Session"] = p_internal(
        72, require=False, array=False, references=NodeType.SESSION, same_bench=True
    )
    run: Optional["Run"] = p_internal(
        73, require=False, array=False, references=NodeType.RUN, same_bench=True
    )
    run_root: Optional["Run"] = p_internal(
        74, require=False, array=False, references=NodeType.RUN, same_bench=True
    )
    client: Optional["Client"] = p_internal(
        75, require=False, array=False, references=NodeType.CLIENT, same_bench=True
    )
    machine: Optional["Machine"] = p_internal(
        76, require=False, array=False, references=NodeType.MACHINE, same_bench=True
    )
    server: Optional["Server"] = p_internal(
        77, require=False, array=False, references=NodeType.SERVER, same_bench=True
    )
    user: Optional["User"] = p_internal(78, require=False, array=False, references=NodeType.USER)
    identity: Optional["Block"] = p_internal(
        79,
        require=False,
        array=False,
        references=NodeType.BLOCK,
        constraint=constraint(block_type=BlockType.IDENTITY),
    )
    if TYPE_CHECKING:
        block_ptr: Optional[NodeReference] = None
        step_ptr: Optional[NodeReference] = None
        session_ptr: Optional[NodeReference] = None
        run_ptr: Optional[NodeReference] = None
        run_root_ptr: Optional[NodeReference] = None
        client_ptr: Optional[NodeReference] = None
        machine_ptr: Optional[NodeReference] = None
        server_ptr: Optional[NodeReference] = None
        user_ptr: Optional[NodeReference] = None
        identity_ptr: Optional[NodeReference] = None


@struct_(StructType.SESSION_CONTEXT)
class SessionContext(Struct, HasSessionContext):
    """Context information for runtime nodes created in a session."""

    pass
