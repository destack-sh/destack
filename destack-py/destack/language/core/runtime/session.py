from collections.abc import Sequence
from datetime import datetime
from typing import (
    TYPE_CHECKING,
    Any,
    Optional,
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
    IsSubject,
    PropertyDeclaration,
    TypeCardinality,
)
from ..common import to_value
from .graph import Supergraph
from .oracle import WORLD_ORACLE, Oracle
from .store import EntityStore, EventStore

if TYPE_CHECKING:
    from destack.language import QueryConnection, Space

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class Session:
    """
    A managed Session for interacting with Destack.
    """

    __slots__ = (
        "_token",
        "closed_at",
        "connections",
        "oracle",
        "pending_events",
        "runtime",
        "space",
        "store",
        "subject",
        "supergraph",
    )

    def __init__(
        self,
        oracle: Oracle = WORLD_ORACLE,
        space: Optional["Space"] = None,
        subject: IsSubject | None = None,
        store: "EventStore | EntityStore | None" = None,
    ):
        self.oracle: Oracle = oracle
        self.space: Space | None = space
        self.subject: IsSubject | None = subject
        self.store: EventStore | EntityStore | None = store
        self.supergraph = Supergraph(self)

        # runtime
        self.pending_events: list[Event] = []
        self.connections: list[QueryConnection] = []
        self.closed_at: datetime | None = None
        self._token: Any | None = None

    def __str__(self) -> str:
        content_parts: list[str] = []
        if self.space:
            content_parts.append(f"destack={self.space.slug}")
        if self.subject is not None:
            content_parts.append(f"subject={self.subject!r}")
        if self.store is not None:
            content_parts.append(f"store={self.store!r}")
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
        self.closed_at = self.oracle.utc()

    def create(self, node: Entity):
        """Creates a new Entity."""
        assert self.closed_at is None, f"{self!r} is closed"
        edit = EditEvent(type=EditType.CREATE, node=node, value=to_value(node, node_as_value=True))
        self.pending_events.append(edit)
        node._is_new = False

    def upsert(self, node: Entity):
        """Creates or updates an Entity."""
        assert self.closed_at is None, f"{self!r} is closed"
        edit = EditEvent(type=EditType.UPSERT, node=node, value=to_value(node, node_as_value=True))
        self.pending_events.append(edit)
        node._is_new = False

    def update_set_property(self, node: Entity, prop: PropertyDeclaration, new_value: Any):
        """Set a Property on this Node (direct SET/CLEAR operations)."""
        old_value = getattr(node, prop.name)
        node_ptr = node.to_ref()
        prop_ptr = prop.to_ref()
        prop_type = prop.to_type()

        # undo
        if old_value is None or (prop.cardinality != TypeCardinality.SCALAR and not old_value):
            undo_operation = EditOperation.CLEAR
            old_value = None
        else:
            undo_operation = EditOperation.SET
            old_value = to_value(old_value, prop_type)

        # do
        if new_value is None or (prop.cardinality != TypeCardinality.SCALAR and not new_value):
            operation = EditOperation.CLEAR
            new_value = None
        else:
            operation = EditOperation.SET
            new_value = to_value(new_value, prop_type)

        edit = EditEvent(
            type=EditType.UPDATE,
            node_ptr=node_ptr,
            attribute=prop_ptr,
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
            node=node,
            value=to_value(parent),
            reverse_value=to_value(old_parent),
        )
        self.pending_events.append(edit)

    def archive(self, node: Entity):
        """Archives an Entity."""
        assert self.closed_at is None, f"{self!r} is closed"
        reverse_value = to_value(node, node_as_value=True)
        edit = EditEvent(
            type=EditType.ARCHIVE,
            node=node,
            reverse_value=reverse_value,
        )
        self.pending_events.append(edit)

    def unarchive(self, node: Entity):
        """Unarchives an Entity."""
        assert self.closed_at is None, f"{self!r} is closed"
        edit = EditEvent(type=EditType.UNARCHIVE, node=node)
        self.pending_events.append(edit)

    def delete(self, node: Entity):
        """Deletes an Entity."""
        assert self.closed_at is None, f"{self!r} is closed"
        reverse_value = to_value(node, node_as_value=True)
        edit = EditEvent(
            type=EditType.DELETE,
            node=node,
            reverse_value=reverse_value,
        )
        self.pending_events.append(edit)

    def restore(self, node: Entity):
        """Restores a deleted Entity."""
        edit = EditEvent(type=EditType.RESTORE, node=node)
        self.pending_events.append(edit)

    def flush(self):
        """Turn pending updates into Edits, and Edits into Changes."""
        pass  # ?

    async def stage(self):
        """Stage pending Edits. Also stages pending Changes in the Store if possible."""
        assert self.closed_at is None, f"{self!r} is closed"
        self.flush()

    async def commit(self) -> Sequence[Event]:
        """
        Commits all Events. Returns applied Events.
        """
        assert self.closed_at is None, f"{self!r} is closed"
        assert self.store is not None, f"{self!r} has no Store"
        self.flush()
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
            event for event in events if event.status != EventStatus.COMPLETED
        ]
        if failed_events:
            pass  # nocheckin: 1) update Event status and 2) do something on failure :RejectedEvents
        #     raise RuntimeError(f"failed to commit {len(failed_events)} Events: {failed_events!r}")
        return applied_events

    async def __aenter__(self):
        await self.open()
        return self

    async def __aexit__(self, exc_type, exc, tb):
        await self.close()
