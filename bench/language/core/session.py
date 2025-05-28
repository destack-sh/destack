import dataclasses
from typing import (
    TYPE_CHECKING,
    Optional,
)

import structlog
from opentelemetry import trace

from bench.pb2 import OriginData
from bench.utils.oracle import REAL_ORACLE, Oracle

from .const import NodeMode
from .graph import Supergraph
from .node import IsSubject, Node
from .transaction import Transaction

if TYPE_CHECKING:
    from bench.language import Bench, Edit, Store
    from bench.runtime.core import Runtime

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclasses.dataclass(slots=True)
class Session:
    """
    A managed Session for interacting with a Bench.
    """

    supergraph: Supergraph = dataclasses.field(default_factory=Supergraph)
    mode: NodeMode = NodeMode.MAIN
    oracle: Oracle = REAL_ORACLE
    bench: Optional["Bench"] = None
    origin: OriginData | None = None
    subject: IsSubject | None = None
    store: "Store" = None
    tx: Transaction = None
    _runtime: Optional["Runtime"] = None

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
        """Creates a new Node."""
        raise NotImplementedError

    def upsert(self, node: Node):
        """Creates or updates a Node."""
        raise NotImplementedError

    def update(self, node: Node):
        """Updates an existing Node."""
        raise NotImplementedError

    def move(self, node: Node, parent: Node):
        """Moves a Node to a new parent."""
        raise NotImplementedError

    def archive(self, node: Node):
        """Archives a Node."""
        raise NotImplementedError

    def unarchive(self, node: Node):
        """Unarchives a Node."""
        raise NotImplementedError

    def delete(self, node: Node):
        """Deletes a Node."""
        raise NotImplementedError

    def restore(self, node: Node):
        """Restores a deleted Node."""
        raise NotImplementedError

    def erase(self, node: Node):
        """Erases a Node."""
        raise NotImplementedError

    @tracer.start_as_current_span("session.stage")
    def stage(self):
        """Stage pending Edits."""
        raise NotImplementedError

    @tracer.start_as_current_span("session.commit.schedule")
    async def commit(self) -> tuple[list["Edit"], list["Edit"]]:
        """
        Commits Edits. Returns applied Edits & their cascaded Edits.
        """
        raise NotImplementedError

    async def __aenter__(self):
        await self.open()
        return self

    async def __aexit__(self, exc_type, exc, tb):
        await self.close()
