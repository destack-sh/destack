import asyncio
import contextvars
from datetime import datetime
from typing import TYPE_CHECKING, Awaitable, Callable, Collection, Optional
from uuid import UUID

import structlog

from bench.language.connection import StoreEngine
from bench.language.const import InterpStatus, NodeType, SessionStatus, StructType, _active_session
from bench.language.graph import NodeDict, NodeGraphLike
from bench.language.node import Node, Struct, node, struct, struct_component
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
        Step,
        Trigger,
        User,
    )

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
ExtendCommitHook = Callable[
    ["Session", NodeGraphLike, list[EditData], list[EditData]], Awaitable[list[EditData]]
]
OnCommitHook = Callable[[NodeGraphLike, list[EditData], list[EditData]], Awaitable[None]]


@node(
    NodeType.SESSION,
    local=True,
    no_ck=True,  # no persistent identity
    id_factory=UUIDT,
    index_together=(("package_id", "created_at"),),
)
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

    # transaction
    _is_readonly: bool = p_runtime(default=False)
    _is_suspended: bool = p_runtime(default=False)
    _is_suppressed: bool = p_runtime(default=False)
    _origin: ClientOrigin | None = p_runtime(default=None)
    _tx: Transaction | None = p_runtime(default=None)
    _tx_lock: asyncio.Lock = p_runtime(default_factory=lambda: CriticalLock(name="session"))
    _edited_nodes_by_id: dict[UUID, Node] = p_runtime(default_factory=dict)
    _engines: tuple["StoreEngine", ...] = p_runtime(default_factory=tuple)
    _active_session_token: contextvars.Token | None = p_runtime(default=None)
    _extend_commit_hook: Optional[ExtendCommitHook] = p_runtime(default=None)
    _on_commit_hook: Optional[OnCommitHook] = p_runtime(default=None)
    _default_scope: GraphScope = p_runtime(default_factory=GraphScope)
    _supervisor: Optional["SupervisorStub"] = p_runtime(default=None)
    _host: Optional["HostStub"] = p_runtime(default=None)

    def __content_str__(self):
        if self.closed_at:
            status_str = "closed"
        elif self.opened_at:
            status_str = "open"
        else:
            status_str = "pending"
        return f"{status_str}, " f"{self._tx or '<no tx>'}"

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
        self._tx = Transaction(session=self, origin=self._origin, is_readonly=self._is_readonly)
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

        logger.trace("session.close", session=self, duration=self.duration)

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

    async def flush(self) -> tuple[list[EditData], list[EditData]]:
        """Flushes the current pending edits. Returns *all* uncommitted edits / cascaded edits."""
        assert self.is_open, f"cannot flush {self!r} when closed"
        assert self._tx is not None, f"no active transaction in {self!r}"

        async with self._tx_lock:
            assert self.is_open, f"cannot flush {self!r} when closed"
            await self._tx.flush()
            return self._tx.edits, self._tx.cascaded_edits

    async def commit(
        self, *, skip_lock: bool = False, suppress_hooks: bool = False
    ) -> tuple[list[EditData], list[EditData]]:
        """Commits all edits. Returns *all* committed edits / cascaded edits, and resets."""
        assert self.is_open, f"cannot commit {self!r} when closed"
        assert self._tx is not None, f"no active transaction in {self!r}"

        if not self._tx.edits:
            return [], []  # nothing to do

        # TODO :Robustness :Broken: rollback edits to in-memory Nodes on session commit error
        try:
            if not skip_lock:
                await self._tx_lock.acquire()
            if suppress_hooks:
                # simple commit
                edits, cascaded_edits = await self._tx.commit()
                return edits, cascaded_edits
            else:
                # wrapped commit (used in Host)
                edit_graph = NodeDict(self._edited_nodes_by_id)
                if self._extend_commit_hook is not None:
                    # flush edits to get cascaded edits
                    edits, cascaded_edits = await self._tx.flush()
                    new_edits = await self._extend_commit_hook(
                        self, edit_graph, edits, cascaded_edits
                    )
                    self._tx._add_pending_edits(new_edits)
                edits, cascaded_edits = await self._tx.commit()
                if self._on_commit_hook is not None:
                    await self._on_commit_hook(edit_graph, edits, cascaded_edits)
                return edits, cascaded_edits
        finally:
            if not skip_lock:
                self._tx_lock.release()

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
            if n is not None and (n._session is not self or n._status != InterpStatus.TRACKED):
                n._track_rec(self)

    def untrack(self, node: Node):
        """Stop tracking the node in this session."""
        for node in node._walk_descendants():
            node._untrack_self()
            if node.id in self._edited_nodes_by_id:
                del self._edited_nodes_by_id[node.id]

    def untrack_many(self, *nodes: Node | None):
        """Stop tracking the nodes in this session."""
        for n in nodes:
            if n is not None:
                self.untrack(n)

    #
    # Transaction
    #

    @property
    def _edit_subject(self) -> Optional["Run"]:
        # Everything that comes this way in a Session is either system (subject=None) or in a Run.
        #  (Users add pending edits to Transactions directly with themselves as a subject)
        # All edits in a Run are attributed to the root for clarity.
        return None  # nocheckin: Session._edit_subject

    def create(self, *nodes: Node):
        """Creates a new node. Errors if the node already exists."""
        if not self._is_suppressed:
            assert self._tx is not None, f"no active transaction in {self!r}"
            assert not self._is_readonly, f"cannot edit in readonly session {self!r}"
            assert not self._is_suspended, f"cannot edit in suspended session {self!r}"
            for n in nodes:
                self._edited_nodes_by_id[n.id] = n
                self._tx.create(n, self._edit_subject)

    def upsert(self, *nodes: Node):
        """Creates or updates a node. Any non-id properties will be overwritten."""
        if not self._is_suppressed:
            assert self._tx is not None, f"no active transaction in {self!r}"
            assert not self._is_readonly, f"cannot edit in readonly session {self!r}"
            assert not self._is_suspended, f"cannot edit in suspended session {self!r}"
            for n in nodes:
                self._edited_nodes_by_id[n.id] = n
                self._tx.upsert(n, self._edit_subject)

    def update(self, *nodes: Node, properties: Collection[Property]):
        """Updates an existing node. Cannot move. The given properties are overwritten."""
        if not self._is_suppressed:
            assert self._tx is not None, f"no active transaction in {self!r}"
            assert not self._is_readonly, f"cannot edit in readonly session {self!r}"
            assert not self._is_suspended, f"cannot edit in suspended session {self!r}"
            for n in nodes:
                self._edited_nodes_by_id[n.id] = n
                self._tx.update(n, self._edit_subject, properties)

    def move(self, *nodes: Node):
        """Moves and updates an existing node."""
        if not self._is_suppressed:
            assert self._tx is not None, f"no active transaction in {self!r}"
            assert not self._is_readonly, f"cannot edit in readonly session {self!r}"
            assert not self._is_suspended, f"cannot edit in suspended session {self!r}"
            for n in nodes:
                self._edited_nodes_by_id[n.id] = n
                self._tx.move(n, self._edit_subject)

    def soft_delete(self, *nodes: Node):
        """Deletes a node with the option to recover it for a limited time."""
        if not self._is_suppressed:
            assert self._tx is not None, f"no active transaction in {self!r}"
            assert not self._is_readonly, f"cannot edit in readonly session {self!r}"
            assert not self._is_suspended, f"cannot edit in suspended session {self!r}"
            for n in nodes:
                self._edited_nodes_by_id[n.id] = n
                # descendants will be removed from graph, so track them manually
                for descendant in n._graph.iter_descendants(n, recursive=True):
                    self._edited_nodes_by_id[descendant.id] = descendant
                self._tx.soft_delete(n, self._edit_subject)

    def restore(self, *nodes: Node):
        """Restore a soft deleted node."""
        if not self._is_suppressed:
            assert self._tx is not None, f"no active  transaction in {self!r}"
            assert not self._is_readonly, f"cannot edit in readonly session {self!r}"
            assert not self._is_suspended, f"cannot edit in suspended session {self!r}"
            for n in nodes:
                self._edited_nodes_by_id[n.id] = n
                self._tx.restore(n, self._edit_subject)

    def archive(self, *nodes: Node):
        """Marks a node as archived, so it will be hidden by default."""
        if not self._is_suppressed:
            assert self._tx is not None, f"no active transaction in {self!r}"
            assert not self._is_readonly, f"cannot edit in readonly session {self!r}"
            assert not self._is_suspended, f"cannot edit in suspended session {self!r}"
            for n in nodes:
                self._edited_nodes_by_id[n.id] = n
                # descendants will be removed from graph, so track them manually
                for descendant in n._graph.iter_descendants(n, recursive=True):
                    self._edited_nodes_by_id[descendant.id] = descendant
                self._tx.archive(n, self._edit_subject)

    def unarchive(self, *nodes: Node):
        """Re-track a node from the archive in its original place."""
        if not self._is_suppressed:
            assert self._tx is not None, f"no active transaction in {self!r}"
            assert not self._is_readonly, f"cannot edit in readonly session {self!r}"
            assert not self._is_suspended, f"cannot edit in suspended session {self!r}"
            for n in nodes:
                self._edited_nodes_by_id[n.id] = n
                self._tx.unarchive(n, self._edit_subject)

    def hard_delete(self, *nodes: Node):
        """Irreversibly deletes a node."""
        if not self._is_suppressed:
            assert self._tx is not None, f"no active transaction in {self!r}"
            assert not self._is_readonly, f"cannot edit in readonly session {self!r}"
            assert not self._is_suspended, f"cannot edit in suspended session {self!r}"
            for n in nodes:
                self._edited_nodes_by_id[n.id] = n
                # descendants will be removed from graph, so track them manually
                for descendant in n._graph.iter_descendants(n, recursive=True):
                    self._edited_nodes_by_id[descendant.id] = descendant
                self._tx.delete(n, self._edit_subject)


@struct_component()
class HasSessionContext(Struct):
    """Context for the creation of a node in some Session."""

    block: Optional["Block"] = p_system(60, require=False, array=False, references=NodeType.BLOCK)
    step: Optional["Step"] = p_system(61, require=False, array=False, references=NodeType.STEP)
    session: Optional["Session"] = p_system(
        62, require=False, array=False, references=NodeType.SESSION, is_bench_implicit=True
    )
    run: Optional["Run"] = p_system(
        63, require=False, array=False, references=NodeType.RUN, is_bench_implicit=True
    )
    client: Optional["Client"] = p_system(
        64, require=False, array=False, references=NodeType.CLIENT, is_bench_implicit=True
    )
    machine: Optional["Machine"] = p_system(
        65, require=False, array=False, references=NodeType.MACHINE, is_bench_implicit=True
    )
    server: Optional["Server"] = p_system(
        66, require=False, array=False, references=NodeType.CLIENT, is_bench_implicit=True
    )
    user: Optional["User"] = p_system(67, require=False, array=False, references=NodeType.USER)

    if TYPE_CHECKING:
        block_ptr: Optional[NodeReferenceData] = None
        step_ptr: Optional[NodeReferenceData] = None
        session_ptr: Optional[NodeReferenceData] = None
        run_ptr: Optional[NodeReferenceData] = None
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
    session: Optional["Session"] = p_internal(
        50, require=False, array=False, references=NodeType.SESSION
    )
    run: Optional["Run"] = p_internal(51, require=False, array=False, references=NodeType.RUN)
    trigger: Optional["Trigger"] = p_internal(
        52, require=False, array=False, references=NodeType.TRIGGER
    )

    # custom
    # value_packed: Any = p_value_packed(50)
    # secret_value_packed: Any = p_secret_value_packed(51)
    # value: Any = p_value_runtime(50, 51)
