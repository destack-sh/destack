from collections.abc import Sequence
from datetime import datetime
from typing import (
    TYPE_CHECKING,
    Any,
    Optional,
)

import structlog
from fastuuid import UUID
from opentelemetry import trace

from bench.utils.oracle import REAL_ORACLE, Oracle

from .const import ACTIVE_SESSION, NodeMode
from .edit import Change, ChangeResult, Edit, EditType
from .graph import Supergraph
from .node import IsSubject, Node
from .store import OptimisticStore
from .value import to_value

if TYPE_CHECKING:
    from bench.language import Bench, Edit, Origin, QueryConnection, Store
    from bench.runtime.core import Runtime

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class Session:
    """
    A managed Session for interacting with a Bench.
    """

    __slots__ = (
        "_token",
        "bench",
        "changes",
        "closed_at",
        "connections",
        "dirty",
        "edits",
        "mode",
        "oracle",
        "origin",
        "runtime",
        "store",
        "subject",
        "supergraph",
    )

    def __init__(
        self,
        mode: NodeMode = NodeMode.MAIN,
        oracle: Oracle = REAL_ORACLE,
        bench: Optional["Bench"] = None,
        origin: "Origin | None" = None,
        subject: IsSubject | None = None,
        store: "Store | None" = None,
        _runtime: Optional["Runtime"] = None,
    ):
        self.supergraph = Supergraph(self)
        self.mode: NodeMode = mode
        self.oracle: Oracle = oracle
        self.bench: Bench | None = bench
        self.origin = origin
        self.subject = subject
        self.store = store

        # transaction (pending)
        self.dirty: dict[UUID, Node] = {}
        self.edits: list[Edit] = []
        self.changes: list[Change] = []

        # runtime
        self.runtime: Runtime | None = _runtime
        self.connections: list[QueryConnection] = []
        self.closed_at: datetime | None = None
        self._token: Any | None = None

    async def open(self):
        """Opens the Session."""
        assert self._token is None, f"{self!r} is already open"
        self._token = ACTIVE_SESSION.set(self)

    async def close(self):
        """Closes the Session."""
        if self._token is not None:
            try:  # noqa: SIM105
                ACTIVE_SESSION.reset(self._token)
            except ValueError:
                pass  # token was created in a different context (during testing usually)
            self._token = None
        self.closed_at = self.oracle.utc()

    def create(self, node: Node):
        """Creates a new Node."""
        assert self.closed_at is None, f"{self!r} is closed"
        edit = Edit(type=EditType.CREATE, node=node, value=to_value(node, node_as_value=True))
        self.edits.append(edit)

    def upsert(self, node: Node):
        """Creates or updates a Node."""
        assert self.closed_at is None, f"{self!r} is closed"
        edit = Edit(type=EditType.UPSERT, node=node, value=to_value(node, node_as_value=True))
        self.edits.append(edit)

    def move(self, node: Node, parent: Node):
        """Moves a Node to a new parent."""
        assert self.closed_at is None, f"{self!r} is closed"
        edit = Edit(type=EditType.MOVE, node=node, parent=parent)
        self.edits.append(edit)

    def archive(self, node: Node):
        """Archives a Node."""
        assert self.closed_at is None, f"{self!r} is closed"
        edit = Edit(type=EditType.ARCHIVE, node=node)
        self.edits.append(edit)

    def unarchive(self, node: Node):
        """Unarchives a Node."""
        assert self.closed_at is None, f"{self!r} is closed"
        edit = Edit(type=EditType.UNARCHIVE, node=node)
        self.edits.append(edit)

    def delete(self, node: Node):
        """Deletes a Node."""
        assert self.closed_at is None, f"{self!r} is closed"
        edit = Edit(type=EditType.DELETE, node=node)
        self.edits.append(edit)

    def restore(self, node: Node):
        """Restores a deleted Node."""
        assert self.closed_at is None, f"{self!r} is closed"
        edit = Edit(type=EditType.RESTORE, node=node)
        self.edits.append(edit)

    def erase(self, node: Node):
        """Erases a Node."""
        assert self.closed_at is None, f"{self!r} is closed"
        edit = Edit(type=EditType.ERASE, node=node)
        self.edits.append(edit)

    def _flush(self):
        """Turn pending updates into Edits, and Edits into Changes."""
        pass

    async def stage(self):
        """Stage pending Edits."""
        assert self.store is not None, f"{self!r} has no Store"
        self._flush()
        if isinstance(self.store, OptimisticStore):
            await self.store.stage(self.changes)

    async def commit(self) -> Sequence[ChangeResult]:
        """
        Commits all Changes/Edits. Returns applied Changes.
        """
        assert self.store is not None, f"{self!r} has no Store"
        self._flush()
        results = await self.store.commit(self.changes)
        return results

    async def __aenter__(self):
        await self.open()
        return self

    async def __aexit__(self, exc_type, exc, tb):
        await self.close()
