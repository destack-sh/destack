import abc
import asyncio
import contextvars
import random
import time
from datetime import UTC, datetime
from random import Random
from typing import Callable, final, override


class Oracle(abc.ABC):
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

    @abc.abstractmethod
    def call_later(
        self, delay: float, callback: Callable, *args, context: contextvars.Context | None = None
    ) -> asyncio.TimerHandle:
        """Call a callback after a duration in seconds. Like asyncio.call_later. Timing is relative to oracle."""
        ...

    @abc.abstractmethod
    def call_at(
        self, when: float, callback: Callable, *args, context: contextvars.Context | None = None
    ) -> asyncio.TimerHandle:
        """Call a callback at a specific time in seconds. Like asyncio.call_at. Timing is relative to oracle."""
        ...

    @abc.abstractmethod
    def call_soon(
        self, callback: Callable, *args, context: contextvars.Context | None = None
    ) -> None:
        """Call a callback as soon as possible. Like asyncio.call_soon."""
        ...


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

    @override
    def call_later(
        self, delay: float, callback: Callable, *args, context: contextvars.Context | None = None
    ) -> asyncio.TimerHandle:
        return asyncio.get_event_loop().call_later(delay, callback, *args, context=context)

    @override
    def call_at(
        self, when: float, callback: Callable, *args, context: contextvars.Context | None = None
    ) -> asyncio.TimerHandle:
        return asyncio.get_event_loop().call_at(when, callback, *args, context=context)

    @override
    def call_soon(
        self, callback: Callable, *args, context: contextvars.Context | None = None
    ) -> None:
        asyncio.get_event_loop().call_soon(callback, *args, context=context)


WORLD_ORACLE = WorldOracle()
