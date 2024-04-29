import asyncio
from collections import deque
from contextvars import ContextVar
from datetime import datetime
from typing import TYPE_CHECKING, Any, Collection, Optional, Union, cast
from uuid import UUID

import structlog
from asgiref.sync import async_to_sync, sync_to_async

from bench.language.const import (
    TERMINAL_RUN_STATUSES,
    BenchError,
    EnumType,
    InterpStatus,
    NodeType,
    RunErrorKind,
    RunStatus,
    StructType,
    _active_session,
    enum_,
)
from bench.language.node import BasedNode, Node, Struct, _Passthrough, node, node_component, struct
from bench.language.property import (
    Property,
    p_internal,
    p_node_ancestor,
    p_node_ancestor_root,
    p_node_child,
    p_node_parent,
    p_runtime,
    p_secret_value_packed,
    p_system,
    p_value_packed,
    p_value_runtime,
)
from bench.language.query import StoreEngine
from bench.language.text import Text
from bench.language.transaction import Transaction
from bench.language.value import HasValues
from bench.proto.wire import (
    AnyNodeData,
    EditData,
    HostStub,
    NodeReferenceData,
    RunData,
    SessionData,
    SignalData,
    SupervisorStub,
)
from bench.sql.core import PrimitiveType
from bench.utils.dt import utcnow_with_tz
from bench.utils.env import IS_DEBUG
from bench.utils.func import IdEnum, _auto_async_to_sync, bytetuple
from bench.utils.uuidt import UUIDT

if TYPE_CHECKING:
    from bench.language import Block, Package, Request, Server

# pyright: reportIncompatibleVariableOverride=false,reportIncompatibleMethodOverride=false

logger = structlog.get_logger(__name__)

# we don't want edits to Signals/Logs to be logged in Signals or Logs (for obvious reasons)
MUTED_EDIT_NODE_TYPES: bytetuple[NodeType] = bytetuple(NodeType.SIGNAL, NodeType.LOG)


@node(
    NodeType.SIGNAL,
    passthrough=(("value", _Passthrough.Full),),
    local=True,
    index_in_search=True,
    id_factory=UUIDT,
)
class Signal(BasedNode[SignalData], HasValues):
    """A signal emitted in this Bench."""

    parent: "Package" = p_node_parent(4, NodeType.PACKAGE)
    # builtin_type: ...
    type: Optional["Block"] = p_internal(
        31, require=False, array=False, references=NodeType.BLOCK, index_in_pg=True
    )
    sender: Optional["Block"] = p_internal(
        33, require=False, array=False, references=NodeType.BLOCK, index_in_pg=True
    )
    value_packed: Any | None = p_value_packed(34)
    secret_value_packed: Any | None = p_secret_value_packed(35)
    value = p_value_runtime(34, 35, type=31)

    @property
    def base(self) -> Optional["Block"]:
        return self.type

    @staticmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        return (cast(SignalData, data)).type_ptr


@enum_(EnumType.LOG_KIND)
class LogKind(IdEnum):
    MESSAGE = 1
    ACCESS = 2


@enum_(EnumType.LOG_LEVEL)
class LogLevel(IdEnum):
    TRACE = 1
    DEBUG = 2
    INFO = 3
    WARNING = 4
    ERROR = 5
    FATAL = 6


@node(NodeType.LOG, stored=True, local=True, index_in_search=True, no_ck=True, id_factory=UUIDT)
class Log(Node):
    """
    A log (entry) is a timestamped event of something happening:
     a message, some Access (read, edit, use), etc.
    """

    parent: "Package" = p_node_parent(4, NodeType.PACKAGE)

    # content
    kind: LogKind = p_system(30)
    level: LogLevel = p_system(31)
    logger: Optional[str] = p_system(32, default=None)
    event: Optional[str] = p_system(33, default=None)
    message: Optional[str] = p_internal(34, default=None)  # the rendered 'text' (if any)
    text: Optional[Text] = p_internal(
        35, default=None, require=False, array=False, struct=StructType.TEXT
    )
    value_packed: Any | None = p_value_packed(36)
    request: Optional["Request"] = p_system(
        37, require=False, array=False, struct=StructType.REQUEST
    )

    # context
    session: Optional["Session"] = p_system(
        40, require=False, array=False, references=NodeType.SESSION
    )
    run: Optional["Run"] = p_system(41, require=False, array=False, references=NodeType.RUN)
    block: Optional["Block"] = p_system(42, require=False, array=False, references=NodeType.BLOCK)

    def __content_str__(self):
        return f"[{self.kind.bench_name}:{self.level.bench_name}] '{self.event or self.message}' ({self.created_at})"


@node(NodeType.SESSION, index_in_search=True, local=True, id_factory=UUIDT)
class Session(Node[SessionData]):
    """
    A managed session for interacting with Bench nodes and (if on a Server) running them.
    """

    parent: Optional["Package"] = p_node_parent(4, NodeType.PACKAGE, is_system=True)
    server: Optional["Server"] = p_system(
        31, require=False, array=False, references=NodeType.SERVER
    )
    opened_at: Optional[datetime] = p_system(32, default=None)
    closed_at: Optional[datetime] = p_system(33, default=None)
    duration: Optional[float] = p_system(34, default=None)

    is_runtime: bool = p_system(40, default=False)
    is_readonly: bool = p_system(41, default=False)

    # transaction
    _tx: Transaction | None = p_runtime(default=None)
    _engines: tuple["StoreEngine", ...] = p_runtime(default_factory=tuple)
    _fallback_engine: Optional["StoreEngine"] = p_runtime(default=None)
    _supervisor: Optional["SupervisorStub"] = p_runtime(default=None)
    _host: Optional["HostStub"] = p_runtime(default=None)

    # runtime
    _stacktrace: list["Run"] | None = p_runtime(default=None)
    _runs_by_id: dict[UUID, "Run"] | None = p_runtime(default=None)
    _pending_runs_by_id: dict[UUID, "Run"] | None = p_runtime(default=None)
    _active_nodes_by_ck: dict[UUID, Node] | None = p_runtime(default=None)

    # logs
    _cached_logs: deque[Log] | None = p_runtime(default=None)
    _pending_logs: list[Log] | None = p_runtime(default=None)
    _flush_session_loop: asyncio.Task | None = p_runtime(default=None)

    def __content_str__(self):
        if self.closed_at:
            status_str = "closed"
        elif self.opened_at:
            status_str = "open"
        else:
            status_str = "pending"
        return (
            f"{status_str}, "
            f"{self._tx or '<no tx>'}, "
            f"{len(self._runs_by_id) if self._runs_by_id is not None else 0} runs"
        )

    def _init_inner(self) -> None:
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

        # prepare runtime
        if self.is_runtime:
            # log collection
            self._pending_logs = []
            self._cached_logs = deque(maxlen=LOG_CACHE_SIZE)
            self._runs_by_id = {}
            self._pending_runs_by_id = {}
            self._active_nodes_by_ck = {}
            self._stacktrace = []

        # open transaction
        self._tx = Transaction(session=self, is_readonly=self.is_readonly)

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

        # close runtime
        if self._flush_session_loop is not None:
            self._flush_session_loop.cancel()
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
        node._untrack_self()

    def untrack_many(self, *nodes: Node):
        """Stop tracking the nodes in this session."""
        for n in nodes:
            n._untrack_self()

    #
    # Transaction
    #

    @property
    def _edit_subject(self) -> Optional["Run"]:
        # Everything that comes this way in a Session is either system (subject=None) or in a Run.
        #  (Users add pending edits to Transactions directly with themselves as a subject)
        # All edits in a Run are attributed to the root for clarity.
        return self._stacktrace[0] if self._stacktrace else None

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

    #
    # Stack: runs/logs
    #

    @property
    def stacktrace(self):
        return self._stacktrace

    # TODO :Broken: track sessions/runs (and signals/logs)
    #  what should session nodes be scoped to? what parent?


# We track the active root in a contextvar but not children
#  because they may be in different contexts, and we cannot reset across contexts.
# This will need to be expanded when we get to parallel runs.
@node(NodeType.RUN, index_in_search=True, local=True, id_factory=UUIDT)
class Run(BasedNode[RunData], HasValues):
    """
    A 'run' of a Block or something (in a session).
    """

    parent: Union["Session", "Run"] = p_node_parent(4, NodeType.SESSION, NodeType.RUN)
    session: "Session" = p_node_ancestor(
        30, NodeType.SESSION, require=True, store=True, wire=True, index_in_pg=True
    )
    root: Optional["Run"] = p_node_ancestor_root(
        31, NodeType.RUN, require=False, store=True, wire=True, index_in_pg=True
    )
    server: Optional["Server"] = p_internal(
        32, require=False, array=False, references=NodeType.SERVER
    )
    block: Optional["Block"] = p_internal(
        33, references=NodeType.BLOCK, require=False, array=False, index_in_pg=True
    )
    # block_path?
    scheduled_at: Optional[datetime] = p_internal(35, default=None)
    started_at: Optional[datetime] = p_internal(36, default=None)
    terminated_at: Optional[datetime] = p_internal(37, default=None)
    duration: float = p_internal(38, default=0)
    status: RunStatus = p_internal(39, index_in_pg=True)

    inputs_packed: Any = p_value_packed(50)
    inputs_secret_packed: Any = p_secret_value_packed(51)
    inputs: Any = p_value_runtime(50, 51)
    outputs_packed: Any = p_value_packed(52)
    outputs_secret_packed: Any = p_secret_value_packed(53)
    outputs: Any = p_value_runtime(52, 53)
    value_packed: Any = p_value_packed(54)
    value_secret_packed: Any = p_secret_value_packed(55)
    value: Any = p_value_runtime(54, 55)
    error: Optional["RunError"] = p_internal(56, default=None, primitive_type=PrimitiveType.JSON)

    runs: list["Run"] = p_node_child(NodeType.RUN)

    # inline_runs: list["Run"] = p_internal(60, require=False, array=True, struct=NodeType.RUN)?

    def __content_str__(self):
        value_keys_str = ", ".join(self.value.keys()) if self.value else ""
        return f"{self.block} ({self.status}, value={value_keys_str or '<none>'}, {self.id})"

    @property
    def active(self) -> bool:
        return self.status not in TERMINAL_RUN_STATUSES

    @property
    def base(self) -> Optional["Block"]:
        return self.block

    @staticmethod
    def get_base_from_data(data: RunData) -> Optional[NodeReferenceData]:
        return data.block_ptr


_active_root_run: ContextVar[Run | None] = ContextVar("active_root_run", default=None)
_active_run_by_root: dict[UUID, Run] = {}


@node_component()
class HasRun(Node):
    """A runnable block"""

    @property
    def _is_async(self) -> Optional[bool]:  # set in supporting components e.g. HasCode
        """Whether this block is async."""
        return None

    def _call_inner(self, *args, **kwargs):
        assert (
            self.is_attached and self._status == InterpStatus.TRACKED
        ), f"cannot call {self!r} (status={self._status!r})"
        try:
            asyncio.get_running_loop()
            is_outer_async = True
        except RuntimeError:
            is_outer_async = False
        inner_call = self._call_inner_async if self._is_async else self._call_inner_sync

        if is_outer_async and not self._is_async:
            inner_call = sync_to_async(inner_call)
        elif not is_outer_async and self._is_async:
            inner_call = async_to_sync(inner_call)

        return inner_call(*args, **kwargs)

    def _call_inner_sync(self, *args, **kwargs):
        raise NotImplementedError

    async def _call_inner_async(self, *args, **kwargs):
        raise NotImplementedError


@struct(StructType.RUN_CODE_FRAME)
class RunCodeFrame(Struct):
    node: Node = p_internal(30, array=False, require=True, references=NodeType.BLOCK)
    lineno: int = p_internal(31)
    name: str = p_internal(32)
    line: str = p_internal(33)

    # locals?


@struct(StructType.RUN_ERROR)
class RunError(Struct, BenchError):
    kind: RunErrorKind = p_internal(30)
    type: str = p_internal(31)
    message: Optional[str] = p_internal(32, default=None)
    node: Optional["Node"] = p_internal(33, require=False, array=False, references=NodeType.BLOCK)
    traceback: list[RunCodeFrame] = p_internal(34, array=True, struct=StructType.RUN_CODE_FRAME)


@node(NodeType.PAUSE, local=True)
class Pause(Node):
    """A resumable interruption in a Run."""

    parent: "Run" = p_node_parent(4, NodeType.RUN)
    session: "Session" = p_node_ancestor(
        30, NodeType.SESSION, require=True, store=True, wire=True, index_in_pg=True
    )
    # (placeholder)


LOG_CACHE_SIZE = 1000
MAX_STACK_DEPTH = 8 if IS_DEBUG else 16
