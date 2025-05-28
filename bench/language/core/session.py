from typing import (
    TYPE_CHECKING,
    Any,
    Optional,
    Sequence,
)

import structlog
from fastuuid import UUID
from opentelemetry import trace

from bench.pb2 import OriginData
from bench.utils.oracle import REAL_ORACLE, Oracle

from .const import ACTIVE_SESSION, NodeMode
from .edit import Change, ChangeResult, Edit, EditType
from .graph import Supergraph
from .node import IsSubject, Node

if TYPE_CHECKING:
    from bench.language import Bench, Edit, Store
    from bench.runtime.core import Runtime

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class Session:
    """
    A managed Session for interacting with a Bench.
    """

    __slots__ = (
        "_runtime",
        "_token",
        "bench",
        "change",
        "changes",
        "dirty",
        "edits",
        "mode",
        "oracle",
        "origin",
        "store",
        "subject",
        "supergraph",
    )

    def __init__(
        self,
        supergraph: Supergraph | None = None,
        mode: NodeMode = NodeMode.MAIN,
        oracle: Oracle = REAL_ORACLE,
        bench: Optional["Bench"] = None,
        origin: OriginData | None = None,
        subject: IsSubject | None = None,
        store: "Store | None" = None,
        _runtime: Optional["Runtime"] = None,
    ):
        self.supergraph = supergraph if supergraph is not None else Supergraph()
        self.mode = mode
        self.oracle = oracle
        self.bench = bench
        self.origin = origin
        self.subject = subject
        self.store = store

        # transaction
        self.change: Change = Change()
        self.changes: list[Change] = []
        self.dirty: dict[UUID, Node] = {}
        self.edits: list[Edit] = []

        # runtime
        self._runtime = _runtime
        self._token: Any | None = None

    @property
    def runtime(self) -> "Runtime":
        assert self._runtime is not None, f"no active Runtime in {self!r}"
        return self._runtime

    async def open(self):
        """Opens the Session."""
        self._token = ACTIVE_SESSION.set(self)

    async def close(self):
        """Closes the Session."""
        if self._token is not None:
            ACTIVE_SESSION.reset(self._token)
            self._token = None

    def create(self, node: Node):
        """Creates a new Node."""
        edit = Edit(type=EditType.CREATE, node_id=node.id)
        self.change.edits.append(edit)
        self.edits.append(edit)

    def upsert(self, node: Node):
        """Creates or updates a Node."""
        edit = Edit(type=EditType.UPSERT, node_id=node.id)
        self.change.edits.append(edit)
        self.edits.append(edit)

    def move(self, node: Node, parent: Node):
        """Moves a Node to a new parent."""
        edit = Edit(type=EditType.MOVE, node_id=node.id, parent=parent)
        self.change.edits.append(edit)
        self.edits.append(edit)

    def archive(self, node: Node):
        """Archives a Node."""
        edit = Edit(type=EditType.ARCHIVE, node_id=node.id)
        self.change.edits.append(edit)
        self.edits.append(edit)

    def unarchive(self, node: Node):
        """Unarchives a Node."""
        edit = Edit(type=EditType.UNARCHIVE, node_id=node.id)
        self.change.edits.append(edit)
        self.edits.append(edit)

    def delete(self, node: Node):
        """Deletes a Node."""
        edit = Edit(type=EditType.DELETE, node_id=node.id)
        self.change.edits.append(edit)
        self.edits.append(edit)

    def restore(self, node: Node):
        """Restores a deleted Node."""
        edit = Edit(type=EditType.RESTORE, node_id=node.id)
        self.change.edits.append(edit)
        self.edits.append(edit)

    def erase(self, node: Node):
        """Erases a Node."""
        edit = Edit(type=EditType.ERASE, node_id=node.id)
        self.change.edits.append(edit)
        self.edits.append(edit)

    async def stage(self):
        """Stage pending Edits."""
        pass  # nocheckin

    async def commit(self) -> Sequence[ChangeResult]:
        """
        Commits Edits. Returns applied Edits & their cascaded Edits.
        """
        raise NotImplementedError

    async def __aenter__(self):
        await self.open()
        return self

    async def __aexit__(self, exc_type, exc, tb):
        await self.close()
