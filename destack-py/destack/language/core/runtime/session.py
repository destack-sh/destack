from collections.abc import Sequence
from datetime import datetime
from typing import (
    TYPE_CHECKING,
    Any,
)

import structlog
from opentelemetry import trace

from ..builtin import (
    ACTIVE_SESSION,
    EditEvent,
    EditOperation,
    EditType,
    Entity,
    Event,
    NodeReference,
    PropertyDeclaration,
)
from ..common import to_value
from .graph import Graph
from .oracle import WORLD_ORACLE, Oracle

if TYPE_CHECKING:
    from destack.language import GraphConnection

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class Session:
    """
    A managed Session for interacting with Spaces.
    """

    __slots__ = (
        "_epoch",
        "_token",
        "actor_ptr",
        "closed_at",
        "connections",
        "graph",
        "oracle",
        "pending_events",
        "runtime",
    )

    def __init__(
        self,
        *,
        oracle: Oracle = WORLD_ORACLE,
        actor_ptr: NodeReference | None = None,
        graph: "Graph",
        epoch: int | None = None,
    ):
        self.oracle: Oracle = oracle
        self.actor_ptr: NodeReference | None = actor_ptr
        self.graph: Graph = graph
        self.pending_events: list[Event] = []
        self.connections: list[GraphConnection] = []
        self.closed_at: datetime | None = None
        self._epoch: int | None = epoch
        self._token: Any | None = None

    def __str__(self) -> str:
        content_parts: list[str] = []
        if self.actor_ptr is not None:
            content_parts.append(f"actor={self.actor_ptr!r}")
        if self.graph is not None:
            content_parts.append(f"graph={self.graph!r}")
        if self._epoch is not None:
            content_parts.append(f"epoch={self._epoch}")
        if self.closed_at is not None:
            content_parts.append(f"closed_at={self.closed_at.isoformat()}")
        return ", ".join(content_parts)

    def __repr__(self) -> str:
        return f"<Session {self!s}>"

    @property
    def epoch(self) -> int:
        """The epoch of the Session."""
        assert self._epoch is not None, f"{self!r} has no epoch"
        return self._epoch

    async def open(self):
        """Opens the Session."""
        assert self.closed_at is None, f"{self!r} is already closed"
        assert self._token is None, f"{self!r} is already open"
        self._token = ACTIVE_SESSION.set(self)

    async def close(self):
        """Closes the Session."""
        assert self.closed_at is None, f"{self!r} is already closed"
        if self._token is not None:
            try:  # noqa: SIM105
                ACTIVE_SESSION.reset(self._token)
            except ValueError:
                pass  # token was created in a different context (during testing usually)
            self._token = None
        self.closed_at = self.oracle.now()

    def append(self, event: Event):
        """Appends an Event."""
        assert self.closed_at is None, f"{self!r} is closed"
        self.pending_events.append(event)

    def create(self, node: Entity):
        """Creates a new Entity."""
        assert self.closed_at is None, f"{self!r} is closed"
        edit = EditEvent(
            type=EditType.CREATE,
            node=node,
            value=to_value(node, node_as_value=True),
            space_ptr=node.space_ptr,
            branch_ptr=node.branch_ptr,
            snapshot_ptr=node.snapshot_ptr,
        )
        self.pending_events.append(edit)
        node._is_new = False

    def upsert(self, node: Entity):
        """Creates or updates an Entity."""
        assert self.closed_at is None, f"{self!r} is closed"
        edit = EditEvent(
            type=EditType.UPSERT,
            node=node,
            value=to_value(node, node_as_value=True),
            space_ptr=node.space_ptr,
            branch_ptr=node.branch_ptr,
            snapshot_ptr=node.snapshot_ptr,
        )
        self.pending_events.append(edit)
        node._is_new = False

    def update_set_property(self, node: Entity, prop: PropertyDeclaration, new_value: Any):
        """Set a Property on this Node (direct SET/CLEAR operations)."""
        assert self.closed_at is None, f"{self!r} is closed"
        old_value = getattr(node, prop.name)
        prop_type = prop.to_type()

        undo_operation = EditOperation.SET
        old_value = to_value(old_value, prop_type)
        operation = EditOperation.SET
        new_value = to_value(new_value, prop_type)
        edit = EditEvent(
            type=EditType.UPDATE,
            space_ptr=node.space_ptr,
            branch_ptr=node.branch_ptr,
            snapshot_ptr=node.snapshot_ptr,
            node=node,
            property_id=prop.id,
            operation=operation,
            value=new_value,
            reverse_operation=undo_operation,
            reverse_value=old_value,
        )
        self.pending_events.append(edit)

    def update(self, node: Entity, edit: EditEvent):
        """Updates an Entity."""
        assert self.closed_at is None, f"{self!r} is closed"
        self.pending_events.append(edit)

    def move(self, node: Entity, parent: Entity):
        """Moves an Entity to a new parent."""
        assert self.closed_at is None, f"{self!r} is closed"
        old_parent = node.parent
        edit = EditEvent(
            type=EditType.MOVE,
            space_ptr=node.space_ptr,
            branch_ptr=node.branch_ptr,
            snapshot_ptr=node.snapshot_ptr,
            node=node,
            value=to_value(parent),
            reverse_value=to_value(old_parent),
        )
        self.pending_events.append(edit)

    def delete(self, node: Entity):
        """Deletes an Entity."""
        assert self.closed_at is None, f"{self!r} is closed"
        reverse_value = to_value(node, node_as_value=True)
        edit = EditEvent(
            type=EditType.DELETE,
            reverse_value=reverse_value,
            space_ptr=node.space_ptr,
            branch_ptr=node.branch_ptr,
            snapshot_ptr=node.snapshot_ptr,
            node=node,
        )
        self.pending_events.append(edit)

    def restore(self, node: Entity):
        """Restores a deleted Entity."""
        assert self.closed_at is None, f"{self!r} is closed"
        edit = EditEvent(
            type=EditType.RESTORE,
            space_ptr=node.space_ptr,
            branch_ptr=node.branch_ptr,
            snapshot_ptr=node.snapshot_ptr,
            node=node,
        )
        self.pending_events.append(edit)

    def _on_flush(self):
        """Stage pending Edits without committing them."""
        pass  # nothing to do here

    def flush(self):
        """Stage pending Edits without committing them."""
        assert self.closed_at is None, f"{self!r} is closed"
        events = self.pending_events
        self.pending_events = []
        self.graph.append(events)
        self._on_flush()

    async def commit(self) -> Sequence[Event]:
        """
        Commits all Events. Returns applied Events.
        """
        assert self.closed_at is None, f"{self!r} is closed"
        self.flush()
        # events = list(self.pending_events)
        # self.pending_events = []
        # nocheckin: 'process' Events (.status, time/epoch, in Space? what authority?)
        #  -> general concept of 'authority' over certain Nodes and their processing?
        #   (like "who runs the timer"? "who runs physics"?)
        #  1) update Event status and 2) do something on failure :RejectedEvents
        #     raise RuntimeError(f"failed to commit {len(failed_events)} Events: {failed_events!r}")
        raise NotImplementedError

    async def __aenter__(self):
        await self.open()
        return self

    async def __aexit__(self, exc_type, exc, tb):
        await self.close()
