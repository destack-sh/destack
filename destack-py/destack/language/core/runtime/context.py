import abc
import asyncio
import time
from datetime import UTC, datetime
from random import Random
from typing import TYPE_CHECKING, final

from destack.utils.uuid import UUID

from ..builtin import (
    Entity,
    Handle,
    HandleType,
    NodeReference,
    builtin_handle,
    builtin_property_runtime,
)

if TYPE_CHECKING:
    from destack.language import Client


@builtin_handle(HandleType.CONTEXT)
class Context(Handle):
    """
    The runtime Context encapsulates most general world state (like time, entropy/RNG, etc.).

    It accumulates down the Entity tree.
    Custom values may be added / updated in the Entity tree.
    """

    # TODO :Incomplete: Context to replace Oracle and Session.actor/space/branch/snapshot/...?
    #  (stacked local Context with mode/time/logging/tracing/baggage/custom stuff, tree down?)
    #  (also with active snapshot_ptr, branch_ptr, ...?)

    actor: "Entity" = builtin_property_runtime(401, is_repr=True)
    client: "Client" = builtin_property_runtime(402, is_repr=True)
    client_nonce: UUID = builtin_property_runtime(403, is_repr=True)
    if TYPE_CHECKING:
        actor_ptr: "NodeReference"
        client_ptr: "NodeReference"

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
