from collections.abc import Sequence
from datetime import datetime
from typing import (
    TYPE_CHECKING,
    Any,
    assert_never,
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
    EventStatus,
    NodeReference,
    PropertyDeclaration,
)
from ..common import to_value
from .graph import Supergraph
from .oracle import WORLD_ORACLE, Oracle
from .store import EntityStore, EventStore

if TYPE_CHECKING:
    from destack.language import QueryConnection

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class Session:
    """
    A managed Session for interacting with Spaces on Destack.
    """

    __slots__ = (
        "_epoch",
        "_token",
        "actor_ptr",
        "closed_at",
        "connections",
        "event_graph",
        "oracle",
        "pending_events",
        "runtime",
        "store",
        "supergraph",
    )

    def __init__(
        self,
        *,
        oracle: Oracle = WORLD_ORACLE,
        actor_ptr: NodeReference | None = None,
        store: "EventStore | EntityStore | None" = None,
        epoch: int | None = None,
    ):
        self.oracle: Oracle = oracle
        self.actor_ptr: NodeReference | None = actor_ptr
        self.store: EventStore | EntityStore | None = store
        self.supergraph = Supergraph(self)
        self.event_graph = self.supergraph.create_event_graph()

        # runtime
        self.pending_events: list[Event] = []
        self.connections: list[QueryConnection] = []
        self.closed_at: datetime | None = None
        self._epoch: int | None = epoch
        self._token: Any | None = None

    def __str__(self) -> str:
        content_parts: list[str] = []
        if self.actor_ptr is not None:
            content_parts.append(f"actor={self.actor_ptr!r}")
        if self.store is not None:
            content_parts.append(f"store={self.store!r}")
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
        self.closed_at = self.oracle.utc()

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
        node_ptr = node.to_ref()
        prop_type = prop.to_type()

        # undo
        undo_operation = EditOperation.SET
        old_value = to_value(old_value, prop_type)

        # do
        operation = EditOperation.SET
        new_value = to_value(new_value, prop_type)

        edit = EditEvent(
            type=EditType.UPDATE,
            node_ptr=node_ptr,
            property_id=prop.id,
            operation=operation,
            value=new_value,
            reverse_operation=undo_operation,
            reverse_value=old_value,
            space_ptr=node.space_ptr,
            branch_ptr=node.branch_ptr,
            snapshot_ptr=node.snapshot_ptr,
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
            node=node,
            value=to_value(parent),
            reverse_value=to_value(old_parent),
            space_ptr=node.space_ptr,
            branch_ptr=node.branch_ptr,
            snapshot_ptr=node.snapshot_ptr,
        )
        self.pending_events.append(edit)

    def delete(self, node: Entity):
        """Deletes an Entity."""
        assert self.closed_at is None, f"{self!r} is closed"
        reverse_value = to_value(node, node_as_value=True)
        edit = EditEvent(
            type=EditType.DELETE,
            node=node,
            reverse_value=reverse_value,
            space_ptr=node.space_ptr,
            branch_ptr=node.branch_ptr,
            snapshot_ptr=node.snapshot_ptr,
        )
        self.pending_events.append(edit)

    def restore(self, node: Entity):
        """Restores a deleted Entity."""
        assert self.closed_at is None, f"{self!r} is closed"
        edit = EditEvent(
            type=EditType.RESTORE,
            node=node,
            space_ptr=node.space_ptr,
            branch_ptr=node.branch_ptr,
            snapshot_ptr=node.snapshot_ptr,
        )
        self.pending_events.append(edit)

    def _on_flush(self):
        """Stage pending Edits without committing them."""
        pass  # nothing to do yet

    async def flush(self):
        """Stage pending Edits without committing them."""
        assert self.closed_at is None, f"{self!r} is closed"
        # TODO :Incomplete: optimistic :SessionStaging
        self._on_flush()

    async def commit(self) -> Sequence[Event]:
        """
        Commits all Events. Returns applied Events.
        """
        assert self.closed_at is None, f"{self!r} is closed"
        assert self.store is not None, f"{self!r} has no Store"
        self._on_flush()
        events = list(self.pending_events)
        self.pending_events = []

        # commit
        if isinstance(self.store, EventStore):
            applied_events: Sequence[Event] = await self.store.append(events)
        elif isinstance(self.store, EntityStore):
            edit_events = [event for event in events if isinstance(event, EditEvent)]
            if len(edit_events) < len(events):
                raise ValueError(f"cannot commit {len(events)} Events with {self.store!r}")
            applied_events: Sequence[Event] = await self.store.commit(edit_events)
        else:
            assert_never(self.store)

        # check
        failed_events: list[Event] = [
            event for event in events if event.status != EventStatus.APPROVED
        ]
        if failed_events:
            # nocheckin: 'process' Events (.status, time/epoch, in Space? what authority?)
            #  -> general concept of 'authority' over certain Nodes and their processing?
            #   (like "who runs the timer"? "who runs physics"?)
            #  1) update Event status and 2) do something on failure :RejectedEvents
            #     raise RuntimeError(f"failed to commit {len(failed_events)} Events: {failed_events!r}")
            pass
        return applied_events

    async def __aenter__(self):
        await self.open()
        return self

    async def __aexit__(self, exc_type, exc, tb):
        await self.close()
