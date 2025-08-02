from collections.abc import Sequence
from typing import TYPE_CHECKING

from destack.language.core import (
    Handle,
    HandleType,
    NodeReference,
    builtin_handle,
    builtin_property_runtime,
)
from destack.utils.uuid import UUID

if TYPE_CHECKING:
    from destack.language import Event


@builtin_handle(HandleType.CONNECTION)
class Connection(Handle):
    """
    A connection between a local and a remote Graph.
    """

    space_ptr: "NodeReference" = builtin_property_runtime(500, is_repr=True)

    async def open(self) -> None:
        raise NotImplementedError

    async def pull(
        self,
        space_id: UUID,
        branch_id: UUID | None,
        snapshot_id: UUID | None,
    ) -> None:
        """Pull the relevant Entities and Events from the remote Graph into this Graph."""
        raise NotImplementedError

    async def push(self, events: Sequence["Event"]) -> Sequence["Event"]:
        """Push the Events to the remote Graph."""
        raise NotImplementedError

    async def close(self) -> None:
        raise NotImplementedError
