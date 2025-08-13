from datetime import datetime
from random import Random
from typing import TYPE_CHECKING, final

from ..builtin import (
    Handle,
    HandleType,
    declare_handle,
)

if TYPE_CHECKING:
    pass


@declare_handle(HandleType.CONTEXT)
class Context(Handle):
    """
    The runtime Context encapsulates most general world state (like time, entropy/RNG, etc.).

    It accumulates down the Entity tree.
    Custom values may be added / updated in the Entity tree.

    Context propagates across the call stack (because it propagates through Runs).
    """

    # nocheckin: Context
    #   make Context a Node again? with partials?
    #   different Contexts for different UniverseDomains/Categories?
    #   Context/UniverseContext
    #    -> SimulationContext
    #      -> PhysicsContext
    #      -> PerceptionContext
    #      -> ...
    #    -> CustomContext? (or actually just Context.custom_values if it's a regular Entity)

    # actor: "Entity"
    # client: "Client"
    # client_nonce: UInt8

    # mode
    # event: Optional["Event"]
    # space: "Space"
    # branch: "Branch"
    # snapshot: "Snapshot"
    # environment: "Environment"

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
        raise NotImplementedError

    def now(self) -> datetime:
        """Current datetime in UTC with microsecond precision."""
        raise NotImplementedError

    async def sleep(self, duration: float) -> None:
        """Sleep for a duration in seconds. Like asyncio.sleep. Timing is relative to oracle."""
        raise NotImplementedError

    # call_later, call_at
