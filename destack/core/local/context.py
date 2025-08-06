from datetime import datetime
from random import Random
from typing import TYPE_CHECKING, Optional, final

from ..builtin import (
    Entity,
    Handle,
    HandleType,
    declare_handle,
    declare_property_runtime,
)
from ..utility import UUID

if TYPE_CHECKING:
    from destack import Branch, Client, Environment, Event, Snapshot, Space


@declare_handle(HandleType.CONTEXT)
class Context(Handle):
    """
    The runtime Context encapsulates most general world state (like time, entropy/RNG, etc.).

    It accumulates down the Entity tree.
    Custom values may be added / updated in the Entity tree.

    Context propagates across the call stack (because it propagates through Runs).
    """

    # TODO :Incomplete: Context to replace local 'globals'
    actor: "Entity" = declare_property_runtime(401, is_repr=True)
    client: "Client" = declare_property_runtime(402, is_repr=True)
    client_nonce: UUID = declare_property_runtime(403, is_repr=True)
    event: Optional["Event"] = declare_property_runtime(404, is_repr=True)

    space: "Space" = declare_property_runtime(
        410,
        is_repr=True,
        description="The Space we're currently in.",
    )
    branch: "Branch" = declare_property_runtime(
        411,
        is_repr=True,
        description="The Branch we're currently in within the Space.",
    )
    snapshot: "Snapshot" = declare_property_runtime(
        412,
        is_repr=True,
        description="The Snapshot we're currently in within the Space and Branch.",
    )
    environment: "Environment" = declare_property_runtime(
        413,
        is_repr=True,
        description="The Environment we're currently in.",
    )

    # random_seed, random_state, ...
    # region/geolocation, ...
    # time/time_zone/time_dilation, ...
    # theme, accessibility, ...
    # logging/tracing/baggage, ...

    def __str__(self) -> str:
        return ""

    @final
    def __repr__(self) -> str:
        content_str = str(self)
        if content_str:
            return f"<{self.__class__.__name__} {content_str}>"
        else:
            return f"<{self.__class__.__name__}>"

    @property
    def random(self) -> Random:
        """Source of randomness."""
        ...

    def now(self) -> datetime:
        """Current datetime in UTC with microsecond precision."""
        ...

    async def sleep(self, duration: float) -> None:
        """Sleep for a duration in seconds. Like asyncio.sleep. Timing is relative to oracle."""
        ...

    # call_later, call_at
