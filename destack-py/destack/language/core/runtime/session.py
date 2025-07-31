from collections.abc import Sequence
from datetime import datetime
from typing import (
    TYPE_CHECKING,
    Any,
)

from destack.utils.log import get_logger
from destack.utils.telemetry import get_tracer
from destack.utils.uuid import UUID

from ..builtin import (
    ACTIVE_SESSION,
    EditEvent,
    EditOperation,
    EditType,
    Entity,
    Event,
    NodeReference,
    PropertyDeclaration,
    Value,
)
from .graph import Graph
from .oracle import WORLD_ORACLE, Oracle

if TYPE_CHECKING:
    from destack.language import GraphConnection

# pyright: reportIncompatibleVariableOverride=false

logger = get_logger(__name__)
tracer = get_tracer(__name__)


class Session:
    """
    A managed Session for interacting with Spaces.
    """

    __slots__ = (
        "_token",
        "actor_ptr",
        "client_nonce",
        "client_ptr",
        "closed_at",
        "connections",
        "graph",
        "local_epoch",
        "oracle",
        "pending_events",
        "remote_epoch",
        "runtime",
    )

    def __init__(
        self,
        *,
        graph: "Graph",
        remote_epoch: int,
        local_epoch: int,
        actor_ptr: "NodeReference",
        client_ptr: "NodeReference",
        client_nonce: UUID,
        oracle: Oracle = WORLD_ORACLE,
    ):
        self.remote_epoch: int = remote_epoch
        self.local_epoch: int = local_epoch
        self.graph: Graph = graph
        self.actor_ptr: NodeReference = actor_ptr
        self.client_ptr: NodeReference = client_ptr
        self.client_nonce: UUID = client_nonce
        self.oracle: Oracle = oracle

        self.pending_events: list[Event] = []
        self.connections: list[GraphConnection] = []
        self.closed_at: datetime | None = None
        self._token: Any | None = None

    def __str__(self) -> str:
        content_parts: list[str] = []
        if self.actor_ptr is not None:
            content_parts.append(f"actor={self.actor_ptr!r}")
        if self.graph is not None:
            content_parts.append(f"graph={self.graph!r}")
        if self.remote_epoch is not None:
            content_parts.append(f"remote_epoch={self.remote_epoch}")
        if self.local_epoch is not None:
            content_parts.append(f"local_epoch={self.local_epoch}")
        if self.closed_at is not None:
            content_parts.append(f"closed_at={self.closed_at.isoformat()}")
        return ", ".join(content_parts)

    def __repr__(self) -> str:
        return f"<Session {self!s}>"

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
            value=Value.wrap(node, node_as_value=True),
            space_ptr=node.space_ptr,
            branch_ptr=node.branch_ptr,
            snapshot_ptr=node.snapshot_ptr,
            created_by_ptr=self.actor_ptr,
        )
        self.pending_events.append(edit)
        node._is_new = False

    def upsert(self, node: Entity):
        """Creates or updates an Entity."""
        assert self.closed_at is None, f"{self!r} is closed"
        edit = EditEvent(
            type=EditType.UPSERT,
            node=node,
            value=Value.wrap(node, node_as_value=True),
            space_ptr=node.space_ptr,
            branch_ptr=node.branch_ptr,
            snapshot_ptr=node.snapshot_ptr,
            created_by_ptr=self.actor_ptr,
        )
        self.pending_events.append(edit)
        node._is_new = False

    def update_set_property(self, node: Entity, prop: PropertyDeclaration, new_value: Any):
        """Set a Property on this Node (direct SET/CLEAR operations)."""
        assert self.closed_at is None, f"{self!r} is closed"
        assert prop.id is not None, f"no id for {prop!r}"
        old_value = getattr(node, prop.name)
        prop_type = prop.type.to_type()

        undo_operation = EditOperation.SET
        old_value = Value.wrap(old_value, prop_type)
        operation = EditOperation.SET
        new_value = Value.wrap(new_value, prop_type)
        edit = EditEvent(
            type=EditType.UPDATE,
            space_ptr=node.space_ptr,
            branch_ptr=node.branch_ptr,
            snapshot_ptr=node.snapshot_ptr,
            created_by_ptr=self.actor_ptr,
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
            created_by_ptr=self.actor_ptr,
            node=node,
            value=Value.wrap(parent),
            reverse_value=Value.wrap(old_parent),
        )
        self.pending_events.append(edit)

    def delete(self, node: Entity):
        """Deletes an Entity."""
        assert self.closed_at is None, f"{self!r} is closed"
        reverse_value = Value.wrap(node, node_as_value=True)
        edit = EditEvent(
            type=EditType.DELETE,
            reverse_value=reverse_value,
            space_ptr=node.space_ptr,
            branch_ptr=node.branch_ptr,
            snapshot_ptr=node.snapshot_ptr,
            created_by_ptr=self.actor_ptr,
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
            created_by_ptr=self.actor_ptr,
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
        # nocheckin: 'process' Events (.status, time/epoch, in Space? who has authority?)
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
