from collections.abc import Sequence
from contextlib import asynccontextmanager
from datetime import datetime
from typing import (
    TYPE_CHECKING,
    Any,
)

from destack.language.core.builtin.property import builtin_property_runtime
from destack.utils.log import get_logger
from destack.utils.telemetry import get_tracer

from ..builtin import (
    ACTIVE_SESSION,
    EditEvent,
    EditOperation,
    EditType,
    Entity,
    Event,
    Handle,
    HandleType,
    Int128,
    PropertyDeclaration,
    Value,
    builtin_handle,
)
from .graph import Graph

if TYPE_CHECKING:
    from destack.language import Context, SpaceConnection

# pyright: reportIncompatibleVariableOverride=false

logger = get_logger(__name__)
tracer = get_tracer(__name__)


@builtin_handle(HandleType.SESSION)
class Session(Handle):
    """
    A managed Session for interacting with Spaces.
    """

    remote_epoch: Int128 = builtin_property_runtime(401, is_repr=True)
    local_epoch: Int128 = builtin_property_runtime(402, is_repr=True)
    root_context: "Context" = builtin_property_runtime(403, is_repr=True)
    graph: "Graph" = builtin_property_runtime(404)
    pending_events: list["Event"] = builtin_property_runtime(406)
    connections: list["SpaceConnection"] = builtin_property_runtime(407)
    closed_at: datetime | None = builtin_property_runtime(408)

    def __repr__(self) -> str:
        return f"<Session {self!s}>"

    @property
    def context(self) -> "Context":
        return self.root_context

    async def open(self):
        """Opens the Session."""
        assert self.closed_at is None, f"{self!r} is already closed"

    async def close(self):
        """Closes the Session."""
        assert self.closed_at is None, f"{self!r} is already closed"
        self.closed_at = self.context.now()

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
            created_by_ptr=self.context.actor_ptr,
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
            created_by_ptr=self.context.actor_ptr,
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
            created_by_ptr=self.context.actor_ptr,
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
            created_by_ptr=self.context.actor_ptr,
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
            created_by_ptr=self.context.actor_ptr,
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
            created_by_ptr=self.context.actor_ptr,
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
        # nocheckin(all): 'process' Events (.status, time/epoch, in Space? who has authority?)
        #  -> general concept of 'authority' over certain Nodes and their processing?
        #   (like "who runs the timer"? "who runs physics"?)
        #  1) update Event status and 2) do something on failure :RejectedEvents
        #     raise RuntimeError(f"failed to commit {len(failed_events)} Events: {failed_events!r}")
        raise NotImplementedError

    @asynccontextmanager
    async def active(self):
        """Context manager for the Session."""
        await self.open()
        token = ACTIVE_SESSION.set(self)
        try:
            try:
                yield self
            finally:
                await self.close()
        finally:
            ACTIVE_SESSION.reset(token)
