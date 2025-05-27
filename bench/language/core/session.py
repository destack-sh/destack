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

from bench.pb2 import OriginData
from bench.utils.oracle import REAL_ORACLE, Oracle

from .const import NodeMode
from .graph import Supergraph
from .node import IsSubject, Node
from .transaction import Transaction

if TYPE_CHECKING:
    from bench.language import Bench, Edit
    from bench.runtime.core import Runtime

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclasses.dataclass(slots=True)
class Session:
    """
    A managed Session for interacting with a Bench.
    """

    # node
    supergraph: Supergraph
    mode: NodeMode = NodeMode.MAIN
    oracle: Oracle = REAL_ORACLE
    bench: Optional["Bench"] = None

    # context
    # ...HasRuntimeContext[80-99]

    # context
    origin: OriginData | None = None
    subject: IsSubject | None = None

    # transaction
    tx: Transaction | None = None
    pending: dict[UUID, Node] = dataclasses.field(default_factory=dict)
    dirty: dict[UUID, Node] = dataclasses.field(default_factory=dict)

    # runtime
    _runtime: Optional["Runtime"] = None

    @property
    def edits(self) -> Sequence["Edit"]:
        return self.tx.edits if self.tx is not None else ()

    @property
    def cascaded_edits(self) -> Sequence["Edit"]:
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

    async def open(self):
        """Opens the Session."""
        pass

    async def close(self):
        """Closes the Session."""
        pass

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
    async def commit(self) -> tuple[list["Edit"], list["Edit"]]:
        """
        Commits all edits. Returns *all* edits & cascaded edits.
        """
        raise NotImplementedError

    async def __aenter__(self):
        await self.open()
        return self

    async def __aexit__(self, exc_type, exc, tb):
        await self.close()
