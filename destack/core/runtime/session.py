from collections.abc import Sequence
from contextlib import asynccontextmanager
from datetime import datetime
from typing import (
    TYPE_CHECKING,
    Any,
)

from ..builtin import (
    ACTIVE_SESSION,
    EditEvent,
    Entity,
    Event,
    Handle,
    HandleType,
    Int128,
    PropertyDeclaration,
    builtin_handle,
    builtin_method,
    builtin_property_runtime,
)
from .graph import Graph

if TYPE_CHECKING:
    from destack import Connection, Context

# pyright: reportIncompatibleVariableOverride=false


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
    connections: list["Connection"] = builtin_property_runtime(407)
    closed_at: datetime | None = builtin_property_runtime(408)

    @property
    def context(self) -> "Context":
        return self.root_context

    @builtin_method(100)
    async def open(self):
        """Opens the Session."""
        ...

    @builtin_method(101)
    async def close(self):
        """Closes the Session."""
        ...

    @builtin_method(102)
    def append(self, event: Event):
        """Appends an Event."""
        self.pending_events.append(event)

    @builtin_method(103)
    def create(self, node: Entity):
        """Creates a new Entity."""
        ...

    @builtin_method(104)
    def upsert(self, node: Entity):
        """Creates or updates an Entity."""
        ...

    @builtin_method(105)
    def update_set_property(self, node: Entity, prop: PropertyDeclaration, new_value: Any):
        """Set a Property on this Node (direct SET/CLEAR operations)."""
        ...

    @builtin_method(106)
    def update(self, node: Entity, edit: EditEvent):
        """Updates an Entity."""
        ...

    @builtin_method(107)
    def move(self, node: Entity, parent: Entity):
        """Moves an Entity to a new parent."""
        ...

    @builtin_method(108)
    def delete(self, node: Entity):
        """Deletes an Entity."""
        ...

    @builtin_method(109)
    def restore(self, node: Entity):
        """Restores a deleted Entity."""
        ...

    @builtin_method(110)
    def flush(self):
        """Stage pending Edits without committing them."""
        ...

    @builtin_method(111)
    async def commit(self) -> Sequence[Event]:
        """
        Commits all Events. Returns applied Events.
        """
        ...

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
