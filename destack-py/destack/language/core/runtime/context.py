import abc
import asyncio
import random
import time
from datetime import UTC, datetime
from random import Random
from typing import final, override

# CONTEXT = 1_300, "Context", "Context", "fas fa-dot"
# nocheckin: Context to replace Oracle and Session.actor_ptr/space_ptr.../epoch? as a Struct/Node/...?
#  (as local partial instances set on some Entities? or as Structs? or something even more implicit?)
#  (stacked local Context with mode/time/logging/tracing/baggage/custom stuff, tree down?,
#   merge Oracle/Session.actor_ptr/.../epoch into Context?)
# also with active snapshot_ptr, branch_ptr, ...?


class Oracle:
    """
    The oracle for all our entropy (e.g., time, randomness).
    """

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
        ...

    @abc.abstractmethod
    def now(self) -> datetime:
        """Current datetime in UTC with microsecond precision."""
        ...

    @abc.abstractmethod
    async def sleep(self, duration: float) -> None:
        """Sleep for a duration in seconds. Like asyncio.sleep. Timing is relative to oracle."""
        ...

    # call_later, call_at


class WorldOracle(Oracle):
    """The real world oracle."""

    def __init__(self, *, seed: int | None = None):
        self._random = random.Random()
        if seed is not None:
            self._random.seed(seed)
        self._time = time

    @property
    @override
    def random(self) -> Random:
        return self._random

    @override
    def time(self) -> float:
        return self._time.time()

    @override
    def now(self) -> datetime:
        return datetime.fromtimestamp(self._time.time_ns() / 1e9, tz=UTC)

    @override
    async def sleep(self, duration: float) -> None:
        await asyncio.sleep(duration)


WORLD_ORACLE = WorldOracle()
