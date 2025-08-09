from datetime import datetime
from typing import (
    TYPE_CHECKING,
    Any,
)

from ..builtin import (
    Entity,
    Event,
    Handle,
    HandleType,
    Int128,
    declare_handle,
    declare_method,
    declare_property_runtime,
)

if TYPE_CHECKING:
    from destack import ChangeEvent, Connection, Context, Graph, PropertyDefinition


@declare_handle(HandleType.SESSION)
class Session(Handle):
    """
    A managed Session for interacting with Spaces.
    """

    remote_epoch: Int128 = declare_property_runtime(401, is_repr=True)
    local_epoch: Int128 = declare_property_runtime(402, is_repr=True)
    root_context: "Context" = declare_property_runtime(403, is_repr=True)
    graph: "Graph" = declare_property_runtime(404)
    pending_events: list["Event"] = declare_property_runtime(406)
    connections: list["Connection"] = declare_property_runtime(407)
    closed_at: datetime | None = declare_property_runtime(408)

    @property
    def context(self) -> "Context":
        return self.root_context

    @declare_method(100)
    async def open(self):
        """Opens the Session."""
        raise NotImplementedError

    @declare_method(101)
    async def close(self):
        """Closes the Session."""
        raise NotImplementedError

    @declare_method(102)
    def append(self, event: Event):
        """Appends an Event."""
        raise NotImplementedError

    @declare_method(103)
    def create(self, node: Entity):
        """Creates a new Entity."""
        raise NotImplementedError

    @declare_method(104)
    def upsert(self, node: Entity):
        """Creates or updates an Entity."""
        raise NotImplementedError

    @declare_method(105)
    def update_set_property(self, node: Entity, prop: "PropertyDefinition", new_value: Any):
        """Set a Property on this Node (direct SET/CLEAR operations)."""
        raise NotImplementedError

    @declare_method(106)
    def update(self, node: Entity, edit: "ChangeEvent"):
        """Updates an Entity."""
        raise NotImplementedError

    @declare_method(107)
    def move(self, node: Entity, parent: Entity):
        """Moves an Entity to a new parent."""
        raise NotImplementedError

    @declare_method(108)
    def delete(self, node: Entity):
        """Deletes an Entity."""
        raise NotImplementedError

    @declare_method(109)
    def restore(self, node: Entity):
        """Restores a deleted Entity."""
        raise NotImplementedError

    @declare_method(110)
    def flush(self):
        """Stage pending Edits without committing them."""
        raise NotImplementedError

    @declare_method(111)
    async def commit(self) -> list[Event]:
        """
        Commits all Events. Returns applied Events.
        """
        raise NotImplementedError
