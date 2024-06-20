import abc
import asyncio
import random
import time
from datetime import datetime
from random import Random
from typing import Callable, final, override

import pytz

MAX_SCHEDULE_DURATION = 100 * 365 * 24 * 60 * 60  # 100 years in seconds
MAX_SCHEDULE_DURATION_NS = MAX_SCHEDULE_DURATION * 1e9  # 100 years in nanoseconds


class Timer:
    """Simple Oracle-backed timer."""

    def __init__(self, oracle: "Oracle") -> None:
        self.oracle = oracle
        self.start_ns = None
        self.end_ns = None

    def __str__(self) -> str:
        if self.start_ns is None:
            return "not started"
        elif self.end_ns is None:
            return "running"
        else:
            return f"{self.elapsed:.3f}s"

    def __repr__(self) -> str:
        return f"<Timer {self!s}>"

    def start(self):
        assert self.start_ns is None, f"f{self!r} already started"
        self.start_ns = self.oracle.time_ns()

    def stop(self):
        assert self.start_ns is not None, f"{self!r} not started"
        assert self.end_ns is None, f"{self!r} already stopped"
        self.end_ns = self.oracle.time_ns()

    def __enter__(self):
        self.start()
        return self

    def __exit__(self, exc_type, exc_value, traceback):
        self.stop()

    async def __aenter__(self):
        self.start()
        return self

    async def __aexit__(self, exc_type, exc_value, traceback):
        self.stop()

    @property
    def elapsed_ns(self) -> int:
        if self.start_ns is None:
            return 0
        elif self.end_ns is None:
            return self.oracle.time_ns() - self.start_ns
        else:
            return self.end_ns - self.start_ns

    @property
    def elapsed(self) -> float:
        return self.elapsed_ns / 1e9


class Oracle(abc.ABC):
    """
    The oracle for all our entropy, like time and randomness.
    Useful to isolate non-determinsim, and of course to mock in simulation testing.
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
        """Sleep for a duration in seconds. Like asyncio.sleep. Timing is relative to oracle."""
        ...

    @abc.abstractmethod
    def call_later(self, duration: float, callback: Callable, *args) -> None:
        """Call a callback after a duration in seconds. Like asyncio.call_later. Timing is relative to oracle."""
        ...

    @abc.abstractmethod
    def call_at(self, when: float, callback: Callable, *args) -> None:
        """Call a callback at a specific time in seconds. Like asyncio.call_at. Timing is relative to oracle."""
        ...

    @abc.abstractmethod
    def call_soon(self, callback: Callable, *args) -> None:
        """Call a callback as soon as possible. Like asyncio.call_soon."""
        ...

    def timer(self) -> Timer:
        """Create a timer."""
        return Timer(self)


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
    def call_later(self, duration: float, callback: Callable, *args) -> None:
        asyncio.get_event_loop().call_later(duration, callback, *args)

    @override
    def call_at(self, when: float, callback: Callable, *args) -> None:
        asyncio.get_event_loop().call_at(when, callback, *args)

    @override
    def call_soon(self, callback: Callable, *args) -> None:
        asyncio.get_event_loop().call_soon(callback, *args)


REAL_ORACLE = RealOracle()
