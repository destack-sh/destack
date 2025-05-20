import dataclasses
from contextlib import asynccontextmanager
from datetime import datetime
from typing import (
    TYPE_CHECKING,
    Callable,
    Optional,
    Sequence,
)

import structlog
from fastuuid import UUID
from opentelemetry import trace

from bench.pb2 import (
    EditData,
    EditOperationData,
    HostClient,
    OriginData,
    RpcMetadata,
    ScopeData,
    SupervisorClient,
)
from bench.utils.oracle import Oracle

from .const import (
    EditType,
    NodeMode,
)
from .graph import GraphData, Supergraph
from .node import Node, Subject
from .object import EMPTY_SCOPE_DATA
from .transaction import Transaction

if TYPE_CHECKING:
    from bench.language import Bench
    from bench.runtime.core import Runtime

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclasses.dataclass(slots=True)
class Session:
    """
    A managed Session for interacting with and running a Bench.
    """

    # node
    mode: NodeMode
    supergraph: Supergraph
    oracle: Oracle
    bench: Optional["Bench"]

    # context
    # ...HasRuntimeContext[80-99]

    # context
    origin: OriginData | None = dataclasses.field(default=None)
    subject: Subject | None = dataclasses.field(default=None)
    _is_suspended: bool = dataclasses.field(default=False)

    # transaction
    _tx: Transaction | None = dataclasses.field(default=None)
    _pending_nodes_by_id: dict[UUID, Node] = dataclasses.field(default_factory=dict)
    _default_scope: ScopeData = dataclasses.field(default_factory=lambda: EMPTY_SCOPE_DATA)
    _local_epoch: int | None = dataclasses.field(default=None)
    _on_edit_subs: dict[UUID, list[Callable[[Node], None]]] = dataclasses.field(
        default_factory=dict
    )

    # runtime
    _rpc_metadata: RpcMetadata | None = dataclasses.field(default=None)
    _rpc_headers: dict[str, str] | None = dataclasses.field(default=None)
    _runtime: Optional["Runtime"] = dataclasses.field(default=None)
    _supervisor: Optional["SupervisorClient"] = dataclasses.field(default=None)
    _self_host: Optional["HostClient"] = dataclasses.field(default=None)
    _bench_host: Optional["HostClient"] = dataclasses.field(default=None)

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

    async def open(self, *, _set_in_context: bool = True):
        """Opens the session for regular business. Activates context (by default)."""
        raise NotImplementedError

    async def close(self):
        """Closes the session, rolling back uncommitted edits. Prevents further use."""
        raise NotImplementedError

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
        now = self.oracle.utc()
        self._pending_nodes_by_id[node.id] = node
        self._tx.record_edit_event(EditType.UPSERT, node, now=now)

    def _update(
        self,
        node: Node,
        operation: EditOperationData | None = None,
    ):
        """Updates an existing Node. The operation *is not* applied directly."""
        raise NotImplementedError

    def _move(self, node: Node, old_parent: Node, new_parent: Node):
        """Moves a Node to a new parent. The operation *is not* applied directly."""
        raise NotImplementedError

    def _archive(self, *nodes: Node, _now: datetime | None = None):
        """Archives a Node. The operation *is* applied directly."""
        raise NotImplementedError

    def _unarchive(self, *nodes: Node, _now: datetime | None = None):
        """Unarchives a Node. The operation *is* applied directly."""
        raise NotImplementedError

    def _delete(self, *nodes: Node, _now: datetime | None = None):
        """Deletes a Node. The operation *is* applied directly."""
        raise NotImplementedError

    def _restore(self, *nodes: Node, _now: datetime | None = None):
        """Restores a deleted Node. The operation *is* applied directly."""
        raise NotImplementedError

    def _erase(self, *nodes: Node):
        """Erases a Node. The operation *is* applied directly."""
        raise NotImplementedError

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
        yield
        raise NotImplementedError

    @tracer.start_as_current_span("session.stage")
    def stage(self, *, include_runtime: bool = False):
        """Stage pending edits without waiting for the next background commit."""
        raise NotImplementedError

    @tracer.start_as_current_span("session.flush.schedule")
    async def flush(self) -> tuple[list[EditData], list[EditData]]:
        """
        Flushes the current pending edits.
        Cascaded edits are only returned for non-optimistic flushes.
        """
        raise NotImplementedError

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
        raise NotImplementedError

    async def __aenter__(self):
        await self.open()
        return self

    async def __aexit__(self, exc_type, exc, tb):
        await self.close()
