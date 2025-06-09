import asyncio
import contextvars
import time
from asyncio.unix_events import DefaultEventLoopPolicy, SelectorEventLoop
from datetime import datetime, timedelta
from random import Random
from typing import Callable, Union, override

import pytz
import structlog
from opentelemetry import trace

from destack.utils.oracle import MAX_SCHEDULE_DURATION, Oracle

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

TimeBaseNs = Union[int, Callable[[], int]]


class SimulatedEventLoop(SelectorEventLoop):
    """
    'Simulated' selector event loop where we can fast forward if there are no pending external selectors.
    Essentially, we just want to safely skip over schedule-only waits during testing.
    In theory, this approach also works with proactor loops for Windows, but we don't need that yet.
    TODO :Test: support automatic fast-forwarding in SimulatedEventLoop (again)
    """

    def __init__(self, selector=None):
        super().__init__(selector)
        self._offset_ns = 0

    def __str__(self) -> str:
        return f"offset_ns={self._offset_ns}"

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {self!s}>"

    def time_ns(self):
        return time.time_ns() + self._offset_ns

    @override
    def time(self):
        return self.time_ns() / 1e9


class SimulatedEventLoopPolicy(DefaultEventLoopPolicy):
    _loop_factory = SimulatedEventLoop


class SimulatedOracle(Oracle):
    """A simulated oracle for deterministic 'randomness' and fast-forwardable timing."""

    def __init__(
        self,
        *,
        loop: SimulatedEventLoop,
        random: Random,
        static_offset: timedelta | int | None = None,
    ):
        self._random = random
        self._loop = loop
        if isinstance(static_offset, timedelta):  # convert to ns
            self._static_offset_ns = (
                (static_offset.days * 24 * 60 * 60 + static_offset.seconds) * 10**6
                + static_offset.microseconds
            ) * 10**3
        elif isinstance(static_offset, int):
            self._static_offset_ns = static_offset
        else:
            self._static_offset_ns = 0

    @property
    @override
    def random(self) -> Random:
        return self._random

    @override
    def time_ns(self) -> int:
        return self._loop.time_ns() + self._static_offset_ns

    @override
    def time(self) -> float:
        return self.time_ns() / 10**9

    @override
    def utc(self) -> datetime:
        return datetime.fromtimestamp(self.time_ns() / 1e9, tz=pytz.utc)

    @override
    async def sleep(self, duration: float) -> None:
        assert duration >= 0, f"invalid sleep duration: {duration}"
        event = asyncio.Event()
        self.call_later(duration, event.set)
        await event.wait()

    @override
    def call_later(
        self, delay: float, callback: Callable, *args, context: contextvars.Context | None = None
    ) -> asyncio.TimerHandle:
        return self.call_at(self.time() + delay, callback, *args, context=context)

    @override
    def call_at(
        self, when: float, callback: Callable, *args, context: contextvars.Context | None = None
    ) -> asyncio.TimerHandle:
        duration = when - self.time()
        assert duration <= MAX_SCHEDULE_DURATION, f"call_at duration too long: {duration:.3f}s"
        # time includes oracle's static offset, so normalize to loop time
        when_loop = when - (self._static_offset_ns / 1e9)
        return self._loop.call_at(when_loop, callback, *args, context=context)

    @override
    def call_soon(
        self, callback: Callable, *args, context: contextvars.Context | None = None
    ) -> None:
        self._loop.call_soon(callback, *args, context=context)
