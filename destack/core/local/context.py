import abc
import asyncio
import time
from datetime import UTC, datetime
from random import Random
from typing import TYPE_CHECKING, final

from ..builtin import (
    Entity,
    Handle,
    HandleType,
    declare_handle,
    declare_property_runtime,
)
from ..utility import UUID

if TYPE_CHECKING:
    from destack import Client


@declare_handle(HandleType.CONTEXT)
class Context(Handle):
    """
    The runtime Context encapsulates most general world state (like time, entropy/RNG, etc.).

    It accumulates down the Entity tree.
    Custom values may be added / updated in the Entity tree.

    Context propagates across the call stack (because it propagates through Runs).
    """

    # nocheckin(language) :Incomplete: Context to replace local 'globals'
    #   - logging/tracing/baggage
    #   - actor/client/client_nonce
    #   - snapshot_ptr/branch_ptr
    #   - custom stuff
    #   - randomness, time, region/geolocation, ...
    #   - mode
    #   - theme

    actor: "Entity" = declare_property_runtime(401, is_repr=True)
    client: "Client" = declare_property_runtime(402, is_repr=True)
    client_nonce: UUID = declare_property_runtime(403, is_repr=True)

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
    @abc.abstractmethod
    def random(self) -> Random:
        """Source of randomness."""

    @abc.abstractmethod
    def time(self) -> float:
        """Current time in seconds since the epoch."""
        return time.time()

    @abc.abstractmethod
    def now(self) -> datetime:
        """Current datetime in UTC with microsecond precision."""
        return datetime.now(UTC)

    @abc.abstractmethod
    async def sleep(self, duration: float) -> None:
        """Sleep for a duration in seconds. Like asyncio.sleep. Timing is relative to oracle."""
        await asyncio.sleep(duration)

    # call_later, call_at
