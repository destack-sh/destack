import dataclasses
from datetime import datetime
from typing import (
    TYPE_CHECKING,
    Optional,
    Sequence,
)

import structlog
from fastuuid import UUID
from opentelemetry import trace

from bench.pb2 import (
    EditData,
    HostClient,
    OriginData,
    RpcMetadata,
    ScopeData,
    SupervisorClient,
)
from bench.utils.oracle import Oracle

from .const import NodeMode
from .graph import Supergraph
from .node import IsSubject, Node
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
    origin: OriginData | None = None
    subject: IsSubject | None = None

    # transaction
    tx: Transaction | None = None
    pending: dict[UUID, Node] = dataclasses.field(default_factory=dict)

    # runtime
    _rpc_metadata: RpcMetadata | None = None
    _rpc_headers: dict[str, str] | None = None
    _runtime: Optional["Runtime"] = None
    _supervisor: Optional["SupervisorClient"] = None
    _self_host: Optional["HostClient"] = None
    _bench_host: Optional["HostClient"] = None

    @property
    def edits(self) -> Sequence[EditData]:
        return self.tx.edits if self.tx is not None else ()

    @property
    def cascaded_edits(self) -> Sequence[EditData]:
        return self.tx.cascaded_edits if self.tx is not None else ()

    @property
    def has_edits(self) -> bool:
        """Whether this session has any non-session edits."""
        return self.tx is not None and self.tx.has_edits

    @property
    def has_pending_edits(self):
        """Whether this session has any pending (unflushed) edits."""
        return self.tx is not None and self.tx.has_pending_edits

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

    async def open(self):
        """Opens the session for regular business. Activates context (by default)."""
        pass

    async def close(self):
        """Closes the session, rolling back uncommitted edits. Prevents further use."""
        pass

    def get_scope(self, node: Node | None = None) -> ScopeData:
        """Get the scope for a node."""
        raise NotImplementedError

    #
    # Edits
    #

    def create(self, node: Node):
        """Creates a new Node. The operation *is not* applied directly."""
        raise NotImplementedError

    def upsert(self, node: Node):
        """Creates or updates a Node. The operation *is not* applied directly."""
        raise NotImplementedError

    def update(self, node: Node):
        """Updates an existing Node. The operation *is not* applied directly."""
        raise NotImplementedError

    def move(self, node: Node, old_parent: Node, new_parent: Node):
        """Moves a Node to a new parent. The operation *is not* applied directly."""
        raise NotImplementedError

    def archive(self, *nodes: Node, _now: datetime | None = None):
        """Archives a Node. The operation *is* applied directly."""
        raise NotImplementedError

    def unarchive(self, *nodes: Node, _now: datetime | None = None):
        """Unarchives a Node. The operation *is* applied directly."""
        raise NotImplementedError

    def delete(self, *nodes: Node, _now: datetime | None = None):
        """Deletes a Node. The operation *is* applied directly."""
        raise NotImplementedError

    def restore(self, *nodes: Node, _now: datetime | None = None):
        """Restores a deleted Node. The operation *is* applied directly."""
        raise NotImplementedError

    def erase(self, *nodes: Node):
        """Erases a Node. The operation *is* applied directly."""
        raise NotImplementedError

    @tracer.start_as_current_span("session.stage")
    def stage(self, *, include_runtime: bool = False):
        """Stage pending edits without waiting for the next background commit."""
        raise NotImplementedError

    @tracer.start_as_current_span("session.commit.schedule")
    async def commit(self) -> tuple[list[EditData], list[EditData]]:
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
