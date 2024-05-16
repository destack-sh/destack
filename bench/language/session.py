from datetime import datetime
from typing import TYPE_CHECKING, Collection, Optional

import structlog

from bench.language.connection import StoreEngine
from bench.language.const import InterpStatus, NodeType, SessionStatus, _active_session
from bench.language.node import Node, node
from bench.language.property import Property, p_internal, p_node_parent, p_runtime, p_system
from bench.language.transaction import Transaction
from bench.proto.wire import EditData, HostStub, SessionData, SupervisorStub
from bench.utils.dt import utcnow_with_tz
from bench.utils.func import _auto_async_to_sync
from bench.utils.uuidt import UUIDT

if TYPE_CHECKING:
    from bench.language import Client, Package, Run, Server, User

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)

# we don't want edits to core runtime types to trigger logs/signals (circular, and very noisy)
MUTED_NODE_TYPES: tuple[NodeType, ...] = (
    NodeType.SESSION,
    NodeType.RUN,
    NodeType.SIGNAL,
    NodeType.LOG,
)


@node(NodeType.SESSION, local=True, id_factory=UUIDT)
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
    user: Optional["User"] = p_internal(63, require=False, array=False, references=NodeType.USER)

    # transaction
    _is_readonly: bool = p_runtime(default=False)
    _tx: Transaction | None = p_runtime(default=None)
    _engines: tuple["StoreEngine", ...] = p_runtime(default_factory=tuple)
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
    def supervisor(self) -> "SupervisorStub":
        """The remote supervisor."""
        assert self._supervisor is not None, f"supervisor not available in {self!r}"
        return self._supervisor

    @property
    def host(self) -> "HostStub":
        """The remote host."""
        assert self._host is not None, f"host not available in {self!r}"
        return self._host

    async def open(self, session_flush_interval: float = 0.1):
        """Opens the session for regular business."""

        if self.opened_at is not None:
            raise RuntimeError(f"session already open: {self!r}")
        if _active_session.get() is not None:
            raise RuntimeError(f"another session is active: {_active_session.get()!r}")
        _active_session.set(self)

        # open transaction
        self._tx = Transaction(session=self, is_readonly=self._is_readonly)

        self.opened_at = utcnow_with_tz()
        logger.trace("session.open")

    @_auto_async_to_sync
    async def flush(self):
        assert self.is_open, f"cannot flush {self!r} when closed"
        await self.tx.flush()

    @_auto_async_to_sync
    async def commit(self) -> Collection[EditData]:
        assert self.is_open, f"cannot commit {self!r} when closed"
        await self.tx.commit()
        return self.tx.edits

    @_auto_async_to_sync
    async def rollback(self):
        assert self.is_open, f"cannot rollback {self!r} when closed"
        await self.tx.rollback()

    @_auto_async_to_sync
    async def close(self):
        """Closes the session, rolling back uncommitted edits. Prevents further runs/edits."""
        assert self.opened_at is not None, f"session not open {self!r}"
        if self.closed_at is not None:
            raise RuntimeError(f"session already closed {self}")

        # close transaction
        await self.tx.close()
        self._tx = None

        # close session
        self.closed_at = utcnow_with_tz()
        self.duration = (self.closed_at - self.opened_at).total_seconds()
        _active_session.set(None)

        logger.trace("session.close", duration=self.duration)

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
            node._track_self(self)

    def track_many(self, *nodes: Node):
        """Start tracking the nodes in this session."""
        for n in nodes:
            if n._session != self or n._status != InterpStatus.TRACKED:
                n._track_self(self)

    def untrack(self, node: Node):
        """Stop tracking the node in this session."""
        node._untrack_rec()

    def untrack_many(self, *nodes: Node):
        """Stop tracking the nodes in this session."""
        for n in nodes:
            n._untrack_rec()

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
        assert self._tx is not None, f"no active transaction in {self!r}"
        for n in nodes:
            self._tx.create(n, self._edit_subject)

    def upsert(self, *nodes: Node):
        """Creates or updates a node. Any non-id properties will be overwritten."""
        assert self._tx is not None, f"no active transaction in {self!r}"
        for n in nodes:
            self._tx.upsert(n, self._edit_subject)

    def update(self, *nodes: Node, properties: Collection[Property]):
        """Updates an existing node. Cannot move. The given properties are overwritten."""
        assert self._tx is not None, f"no active transaction in {self!r}"
        for n in nodes:
            self._tx.update(n, self._edit_subject, properties)

    def move(self, *nodes: Node):
        """Moves and updates an existing node."""
        assert self._tx is not None, f"no active transaction in {self!r}"
        for n in nodes:
            self._tx.move(n, self._edit_subject)

    def delete(self, *nodes: Node):
        """Deletes a node with the option to recover it for a limited time."""
        assert self._tx is not None, f"no active transaction in {self!r}"
        for n in nodes:
            self._tx.soft_delete(n, self._edit_subject)

    def restore(self, *nodes: Node):
        """Restore a soft deleted node."""
        assert self._tx is not None, f"no active  transaction in {self!r}"
        for n in nodes:
            self._tx.restore(n, self._edit_subject)

    def archive(self, *nodes: Node):
        """Marks a node as archived, so it will be hidden by default."""
        assert self._tx is not None, f"no active transaction in {self!r}"
        for n in nodes:
            self._tx.archive(n, self._edit_subject)

    def unarchive(self, *nodes: Node):
        """Re-track a node from the archive in its original place."""
        assert self._tx is not None, f"no active transaction in {self!r}"
        for n in nodes:
            self._tx.unarchive(n, self._edit_subject)

    def hard_delete_forever(self, *nodes: Node):
        """Irreversibly deletes a node."""
        assert self._tx is not None, f"no active transaction in {self!r}"
        for n in nodes:
            self._tx.delete(n, self._edit_subject)
