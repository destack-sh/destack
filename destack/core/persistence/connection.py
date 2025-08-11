from collections.abc import Sequence
from typing import TYPE_CHECKING

from ..builtin import (
    UUID,
    Handle,
    HandleType,
    declare_handle,
)

if TYPE_CHECKING:
    from destack import Event


@declare_handle(HandleType.CONNECTION)
class Connection(Handle):
    """
    A connection between a local and a remote Graph.
    """

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
