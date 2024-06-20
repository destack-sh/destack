import abc
import asyncio
from datetime import datetime, timedelta
from random import Random
from typing import Callable, override

import pytz

from bench.utils.oracle import Oracle


class LoopBase(abc.ABC):
    """Abstract timing functionality from event loop."""

    @abc.abstractmethod
    async def sleep(self, duration: float) -> None: ...

    @abc.abstractmethod
    def call_later(self, duration: float, callback: Callable) -> None: ...

    @abc.abstractmethod
    def call_at(self, when: float, callback: Callable) -> None: ...

    @abc.abstractmethod
    def call_soon(self, callback: Callable) -> None: ...


class SimulatedTimeLoop:
    """Simulated async timing with fast-forwarding."""

    def __init__(self, loop: asyncio.AbstractEventLoop) -> None:
        pass

    async def sleep(self, duration: float) -> None: ...

    def call_later(self, duration: float, callback: Callable) -> None: ...

    def call_at(self, when: float, callback: Callable) -> None: ...

    def call_soon(self, callback: Callable) -> None: ...


class SimulatedOracle(Oracle):
    """A mock source of entropy for simulation."""

    def __init__(
        self,
        *,
        random: Random,
        base_ns: int | Callable[[], int],
        offset: timedelta | None,
        loop: LoopBase,
    ):
        self._random = random
        self._base_ns = base_ns
        self._offset = offset
        if offset is not None:
            self._offset_ns = (
                (offset.days * 24 * 60 * 60 + offset.seconds) * 10**6 + offset.microseconds
            ) * 10**3
        else:
            self._offset_ns = 0
        self._loop = loop

    @property
    @override
    def random(self) -> Random:
        return self._random

    @override
    def time_ns(self) -> int:
        base_ns = self._base_ns if isinstance(self._base_ns, int) else self._base_ns()
        base_ns += self._offset_ns
        return base_ns

    @override
    def time(self) -> float:
        return self.time_ns() / 10**9

    @override
    def utc(self) -> datetime:
        return datetime.fromtimestamp(self.time_ns() / 1e9, tz=pytz.utc)

    @override
    async def sleep(self, duration: float) -> None:
        return await self._loop.sleep(duration)

    @override
    def call_later(self, duration: float, callback: Callable) -> None:
        self._loop.call_later(duration, callback)

    @override
    def call_at(self, when: float, callback: Callable) -> None:
        self._loop.call_at(when, callback)

    @override
    def call_soon(self, callback: Callable) -> None:
        self._loop.call_soon(callback)
