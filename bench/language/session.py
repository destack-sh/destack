import asyncio
import contextvars
from contextlib import asynccontextmanager
from datetime import datetime
from typing import TYPE_CHECKING, Any, Awaitable, Callable, Collection, Optional
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language.connection import ConnectionFailedError, StoreEngine
from bench.language.const import (
    NodeType,
    PrimitiveType,
    SessionStatus,
    StructType,
    _active_session,
    get_active_run,
)
from bench.language.node import (
    EditSubject,
    Node,
    Struct,
    struct,
    struct_component,
    timed_node,
)
from bench.language.property import Property, p_internal, p_node_parent, p_runtime, p_system
from bench.language.transaction import Transaction
from bench.proto.wire import (
    ClientOrigin,
    EditData,
    GraphScope,
    HostStub,
    NodeReferenceData,
    SessionData,
    SupervisorStub,
)
from bench.utils.dt import utcnow
from bench.utils.func import CriticalLock, uuid_to_str
from bench.utils.uuidt import UUIDT

if TYPE_CHECKING:
    from bench.language import (
        Bench,
        Block,
        Branch,
        Client,
        Environment,
        Machine,
        Package,
        Run,
        Server,
        Signal,
        Step,
        Trigger,
        User,
    )

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

CustomCommit = Callable[["Session"], Awaitable[tuple[list[EditData], list[EditData]]]]


@timed_node(NodeType.SESSION)
class Session(Node[SessionData]):
    """
    A managed Session for interacting with and running a Package in a Client.
    If a Run spans multiple Clients, each Client will have its own Session.
    On some Clients a Session may persist across Runs (like in the web client).
    Once closed, a Session (like a Run) is effectively immutable.
    """

    parent: Optional["Package"] = p_node_parent(4, NodeType.PACKAGE, is_system=True)

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

    # flags
    _is_readonly: bool = p_runtime(default=False)
    _is_suspended: bool = p_runtime(default=False)
    _is_suppressed: bool = p_runtime(default=False)

    # transaction
    _origin: ClientOrigin | None = p_runtime(default=None)
    _subject: EditSubject | None = p_runtime(default=None)
    _engines: tuple["StoreEngine", ...] = p_runtime(default_factory=tuple)
    _tx: Transaction | None = p_runtime(default=None)
    _tx_lock: asyncio.Lock = p_runtime(default_factory=lambda: CriticalLock(name="session"))
    _edited_nodes_by_id: dict[UUID, Node] = p_runtime(default_factory=dict)

    # runtime
    _default_scope: GraphScope = p_runtime(default_factory=GraphScope)
    _active_session_token: contextvars.Token | None = p_runtime(default=None)
    _supervisor: Optional["SupervisorStub"] = p_runtime(default=None)
    _host: Optional["HostStub"] = p_runtime(default=None)

    # system
    _epoch: int | None = p_runtime(default=None)
    _custom_commit: CustomCommit | None = p_runtime(default=None)

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
        if self._is_suppressed:
            status_strs.append("suppressed")
        if self.duration is not None:
            return f"{', '.join(status_strs)}, tx={self._tx or '<no tx>'}, duration={self.duration:.3f}s"
        else:
            return f"{', '.join(status_strs)}, tx={self._tx or '<no tx>'}"

    def _init_component(self) -> None:
        self._session = self
        if self.parent is not None:
            self._default_scope = GraphScope(
                bench_id=uuid_to_str(self.parent.bench_id), package_id=uuid_to_str(self.parent.id)
            )

    @property
    def tx(self) -> Transaction:
        assert self._tx is not None, f"no active transaction in {self!r}"
        return self._tx

    @property
    def has_edits(self) -> bool:
        """Whether this session has any non-session edits."""
        return self._tx is not None and self._tx.has_edits

    @property
    def has_pending_edits(self):
        """Whether this session has any pending (unflushed) edits."""
        return self._tx is not None and self._tx.has_pending_edits

    @property
    def epoch(self) -> int:
        assert self._epoch is not None, f"epoch not available in {self!r}"
        return self._epoch

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
    def supervisor(self) -> "SupervisorStub":
        """The remote supervisor."""
        assert self._supervisor is not None, f"supervisor not available in {self!r}"
        return self._supervisor

    @property
    def host(self) -> "HostStub":
        """The remote host."""
        assert self._host is not None, f"host not available in {self!r}"
        return self._host

    async def open(self, *, in_context: bool = True):
        """Opens the session for regular business. Activates context (by default)."""
        assert not self.closed_at, f"session already closed {self!r}"
        assert not self.opened_at, f"session already open {self!r}"
        if in_context:
            self._active_session_token = _active_session.set(self)
        self._tx = Transaction(id=UUIDT(), session=self, is_readonly=self._is_readonly)
        self.opened_at = utcnow()
        logger.trace("session.open", session=self)

    async def close(self):
        """Closes the session, rolling back uncommitted edits. Prevents further use."""
        assert self.opened_at, f"session not open {self!r}"
        assert not self.closed_at, f"session already closed {self!r}"

        # close transaction
        async with self._tx_lock:
            await self.tx.close()
            self._tx = None

        # close session
        self.closed_at = utcnow()
        self.duration = (self.closed_at - self.opened_at).total_seconds()
        if self._active_session_token is not None:
            _active_session.reset(self._active_session_token)
            self._active_session_token = None

        logger.trace("session.close", session=self)

    def suppress(self):
        """Suppress any the session, *ignoring* further edits."""
        self._is_suppressed = True

    def unsuppress(self):
        """Stop suppressing the session, accepting further edits."""
        self._is_suppressed = False

    def suspend(self):
        """Suspend the session, *erroring* on further edits. Deactivates context (if active)."""
        self._is_suspended = True
        assert self._active_session_token is not None, f"session not active {self!r}"
        _active_session.reset(self._active_session_token)
        self._active_session_token = None

    def unsuspend(self):
        """Stop suspending the session, allowing further edits. Activates context."""
        assert self._active_session_token is None, f"session already active {self!r}"
        self._is_suspended = False
        self._active_session_token = _active_session.set(self)

    # TODO :Robustness!: auto re-connect Session.flush/commit/...? on ConnectionFailedError
    #  (need to replay all previous edits, maybe do some other stuff?)

    @tracer.start_as_current_span("session.flush")
    async def flush(self, *, _skip_lock: bool = False) -> tuple[list[EditData], list[EditData]]:
        """Flushes the current pending edits. Returns *all* uncommitted edits / cascaded edits."""
        assert self.is_open, f"cannot flush {self!r} when closed"
        assert self._tx is not None, f"no active transaction in {self!r}"

        try:
            if not _skip_lock:
                await self._tx_lock.acquire()
            await self._tx.flush()
            return self._tx.edits, self._tx.cascaded_edits
        except ConnectionFailedError as e:
            logger.error("session.flush.error", session=self, error=e)
            await self._tx.reset()
            raise
        finally:
            if not _skip_lock:
                self._tx_lock.release()

    @tracer.start_as_current_span("session.flush")
    async def commit(self, *, _skip_lock: bool = False) -> tuple[list[EditData], list[EditData]]:
        """Commits all edits. Returns *all* committed edits / cascaded edits *and* resets them."""
        assert self.is_open, f"cannot commit {self!r} when closed"
        assert self._tx is not None, f"no active transaction in {self!r}"

        if not self._tx.edits:
            return [], []  # nothing to do

        # TODO :Robustness :Broken: rollback edits to in-memory Nodes on session commit error
        try:
            if not _skip_lock:
                await self._tx_lock.acquire()
            if self._custom_commit is None:
                # simple commit
                return await self._tx.commit()
            else:
                # custom commit (in system)
                return await self._custom_commit(self)
        except ConnectionFailedError as e:
            logger.error("session.commit.error", session=self, error=e)
            await self._tx.reset()
            raise
        finally:
            if not _skip_lock:
                self._tx_lock.release()

    @tracer.start_as_current_span("session.rollback")
    async def rollback(self):
        assert self.is_open, f"cannot rollback {self!r} when closed"
        async with self._tx_lock:
            await self.tx.rollback()

    async def __aenter__(self):
        await self.open()
        return self

    async def __aexit__(self, exc_type, exc, tb):
        await self.close()

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

    def _get_edit_subject(self) -> Optional[EditSubject]:
        if self._subject is not None:
            return self._subject
        else:
            return get_active_run()

    def create(self, *nodes: Node):
        """Creates a new node. Errors if the node already exists."""
        if not self._is_suppressed:
            assert self._tx is not None, f"no active transaction in {self!r}"
            assert not self._is_readonly and not self._is_suspended, f"cannot edit in {self!r}"
            for n in nodes:
                self._edited_nodes_by_id[n.id] = n
                self._tx.create(n, self._get_edit_subject(), self._origin)

    def upsert(self, *nodes: Node):
        """Creates or updates a node. Any non-id properties will be overwritten."""
        if not self._is_suppressed:
            assert self._tx is not None, f"no active transaction in {self!r}"
            assert not self._is_readonly and not self._is_suspended, f"cannot edit in {self!r}"
            for n in nodes:
                self._edited_nodes_by_id[n.id] = n
                self._tx.upsert(n, self._get_edit_subject(), self._origin)

    def update(self, node_: Node, properties: Collection[Property], old_values: dict[int, Any]):
        """Updates an existing node. Cannot move. The given properties are overwritten."""
        if not self._is_suppressed:
            assert self._tx is not None, f"no active transaction in {self!r}"
            assert not self._is_readonly and not self._is_suspended, f"cannot edit in {self!r}"
            self._edited_nodes_by_id[node_.id] = node_
            self._tx.update(node_, self._get_edit_subject(), self._origin, properties, old_values)

    def move(self, node_: Node, properties: Collection[Property], old_values: dict[int, Any]):
        """Moves and updates an existing node."""
        if not self._is_suppressed:
            assert self._tx is not None, f"no active transaction in {self!r}"
            assert not self._is_readonly and not self._is_suspended, f"cannot edit in {self!r}"
            self._edited_nodes_by_id[node_.id] = node_
            self._tx.move(node_, self._get_edit_subject(), self._origin, properties, old_values)

    def soft_delete(self, *nodes: Node):
        """Deletes a node with the option to recover it for a limited time."""
        if not self._is_suppressed:
            assert self._tx is not None, f"no active transaction in {self!r}"
            assert not self._is_readonly and not self._is_suspended, f"cannot edit in {self!r}"
            for n in nodes:
                self._edited_nodes_by_id[n.id] = n
                # descendants will be removed from graph, so track them manually
                for descendant in n._graph.iter_descendants(n, recursive=True):
                    self._edited_nodes_by_id[descendant.id] = descendant
                self._tx.soft_delete(n, self._get_edit_subject(), self._origin)

    def restore(self, *nodes: Node):
        """Restore a soft deleted node."""
        if not self._is_suppressed:
            assert self._tx is not None, f"no active  transaction in {self!r}"
            assert not self._is_readonly and not self._is_suspended, f"cannot edit in {self!r}"
            for n in nodes:
                self._edited_nodes_by_id[n.id] = n
                self._tx.restore(n, self._get_edit_subject(), self._origin)

    def archive(self, *nodes: Node):
        """Marks a node as archived, so it will be hidden by default."""
        if not self._is_suppressed:
            assert self._tx is not None, f"no active transaction in {self!r}"
            assert not self._is_readonly and not self._is_suspended, f"cannot edit in {self!r}"
            for n in nodes:
                self._edited_nodes_by_id[n.id] = n
                # descendants will be removed from graph, so track them manually
                for descendant in n._graph.iter_descendants(n, recursive=True):
                    self._edited_nodes_by_id[descendant.id] = descendant
                self._tx.archive(n, self._get_edit_subject(), self._origin)

    def unarchive(self, *nodes: Node):
        """Re-track a node from the archive in its original place."""
        if not self._is_suppressed:
            assert self._tx is not None, f"no active transaction in {self!r}"
            assert not self._is_readonly and not self._is_suspended, f"cannot edit in {self!r}"
            for n in nodes:
                self._edited_nodes_by_id[n.id] = n
                self._tx.unarchive(n, self._get_edit_subject(), self._origin)

    def hard_delete(self, *nodes: Node):
        """Irreversibly deletes a node."""
        if not self._is_suppressed:
            assert self._tx is not None, f"no active transaction in {self!r}"
            assert not self._is_readonly and not self._is_suspended, f"cannot edit in {self!r}"
            for n in nodes:
                self._edited_nodes_by_id[n.id] = n
                # descendants will be removed from graph, so track them manually
                for descendant in n._graph.iter_descendants(n, recursive=True):
                    self._edited_nodes_by_id[descendant.id] = descendant
                self._tx.delete(n, self._get_edit_subject(), self._origin)


@struct_component()
class HasSessionContext(Struct):
    """Context for the creation of a node in some Session."""

    # NOTE :Security: session context properties are p_internal, not p_system so we can update
    #   them in all clients. This however also means users can mess with them if they really want to.
    # :SessionContext
    block: Optional["Block"] = p_internal(60, require=False, array=False, references=NodeType.BLOCK)
    step: Optional["Step"] = p_internal(61, require=False, array=False, references=NodeType.STEP)
    session: Optional["Session"] = p_internal(
        62, require=False, array=False, references=NodeType.SESSION, same_bench=True
    )
    run: Optional["Run"] = p_internal(
        63, require=False, array=False, references=NodeType.RUN, same_bench=True
    )
    run_root: Optional["Run"] = p_internal(
        64, require=False, array=False, references=NodeType.RUN, same_bench=True
    )
    client: Optional["Client"] = p_internal(
        65, require=False, array=False, references=NodeType.CLIENT, same_bench=True
    )
    machine: Optional["Machine"] = p_internal(
        66, require=False, array=False, references=NodeType.MACHINE, same_bench=True
    )
    server: Optional["Server"] = p_internal(
        67, require=False, array=False, references=NodeType.SERVER, same_bench=True
    )
    user: Optional["User"] = p_internal(68, require=False, array=False, references=NodeType.USER)

    if TYPE_CHECKING:
        block_ptr: Optional[NodeReferenceData] = None
        step_ptr: Optional[NodeReferenceData] = None
        session_ptr: Optional[NodeReferenceData] = None
        run_ptr: Optional[NodeReferenceData] = None
        run_root_ptr: Optional[NodeReferenceData] = None
        client_ptr: Optional[NodeReferenceData] = None
        machine_ptr: Optional[NodeReferenceData] = None
        server_ptr: Optional[NodeReferenceData] = None
        user_ptr: Optional[NodeReferenceData] = None


@struct(StructType.CONTEXT)
class Context(Struct):
    """A semi-magical value of context down a Bench tree (starting with system context)."""

    # location
    bench: Optional["Bench"] = p_internal(30, require=False, array=False, references=NodeType.BENCH)
    environment: Optional["Environment"] = p_internal(
        31, require=False, array=False, references=NodeType.ENVIRONMENT
    )
    branch: Optional["Branch"] = p_internal(
        32, require=False, array=False, references=NodeType.BRANCH
    )
    package: Optional["Package"] = p_internal(
        33, require=False, array=False, references=NodeType.PACKAGE
    )
    module: Optional["Block"] = p_internal(
        34, require=False, array=False, references=NodeType.BLOCK
    )
    page: Optional["Block"] = p_internal(35, require=False, array=False, references=NodeType.BLOCK)
    block: Optional["Block"] = p_internal(36, require=False, array=False, references=NodeType.BLOCK)
    step: Optional["Step"] = p_internal(37, require=False, array=False, references=NodeType.STEP)

    # runtime
    client: Optional["Client"] = p_internal(
        40, require=False, array=False, references=NodeType.CLIENT
    )
    server: Optional["Server"] = p_internal(
        41, require=False, array=False, references=NodeType.SERVER
    )
    user: Optional["User"] = p_internal(42, require=False, array=False, references=NodeType.USER)

    # session
    epoch: Optional[int] = p_internal(
        50, require=False, array=False, primitive_type=PrimitiveType.INT64
    )
    session: Optional["Session"] = p_internal(
        51, require=False, array=False, references=NodeType.SESSION
    )
    run: Optional["Run"] = p_internal(52, require=False, array=False, references=NodeType.RUN)
    run_root: Optional["Run"] = p_internal(53, require=False, array=False, references=NodeType.RUN)
    trigger: Optional["Trigger"] = p_internal(
        54, require=False, array=False, references=NodeType.TRIGGER
    )
    signal: Optional["Signal"] = p_internal(
        55, require=False, array=False, references=NodeType.SIGNAL
    )

    # custom
    # value_packed: Any = p_value_packed(60)
    # secret_value_packed: Any = p_secret_value_packed(61)
    # value: Any = p_value_runtime(60, 61)


@asynccontextmanager
async def unsuspend_session(session: Session, readonly: bool, autocommit: bool):
    """Gets exclusive query and edit access to the main session."""
    was_readonly = session._is_readonly
    session._is_readonly = readonly
    session.unsuspend()
    try:
        yield session
        if autocommit:
            await session.commit()
        elif session.tx.edits:
            raise RuntimeError(f"uncommitted edits in {session!r}: {session.tx.edits!r}")
    finally:
        session.suspend()  # suspend by default
        session._is_readonly = was_readonly
