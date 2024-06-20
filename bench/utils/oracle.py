import abc
import asyncio
import random
import time
from datetime import datetime
from random import Random
from typing import Callable, final, override

import pytz


class Oracle(abc.ABC):
    """
    The oracle for all our entropy, any non-deterministic information.
    Stuff like time and randomness. Used for simulation testing.
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
    def time_ns(self) -> int:
        """Current time in nanoseconds since the epoch."""
        ...

    @abc.abstractmethod
    def time(self) -> float:
        """Current time in seconds since the epoch."""
        ...

    @abc.abstractmethod
    def utc(self) -> datetime:
        """Current datetime in UTC with microsecond precision."""
        ...

    @abc.abstractmethod
    async def sleep(self, duration: float) -> None:
        """Sleep for a duration in seconds. Like asyncio.sleep."""
        ...

    @abc.abstractmethod
    def call_later(self, duration: float, callback: Callable) -> None:
        """Call a callback after a duration in seconds. Like asyncio.call_later."""
        ...

    @abc.abstractmethod
    def call_at(self, when: float, callback: Callable) -> None:
        """Call a callback at a specific time. Like asyncio.call_at."""
        ...

    @abc.abstractmethod
    def call_soon(self, callback: Callable) -> None:
        """Call a callback as soon as possible. Like asyncio.call_soon."""
        ...


class RealOracle(Oracle):
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
    def time_ns(self) -> int:
        return self._time.time_ns()

    @override
    def utc(self) -> datetime:
        return datetime.fromtimestamp(self._time.time_ns() / 1e9, tz=pytz.utc)

    @override
    async def sleep(self, duration: float) -> None:
        await asyncio.sleep(duration)

    @override
    def call_later(self, duration: float, callback: Callable) -> None:
        asyncio.get_event_loop().call_later(duration, callback)

    @override
    def call_at(self, when: float, callback: Callable) -> None:
        asyncio.get_event_loop().call_at(when, callback)

    @override
    def call_soon(self, callback: Callable) -> None:
        asyncio.get_event_loop().call_soon(callback)


REAL_ORACLE = RealOracle()
