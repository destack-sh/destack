import asyncio
import contextlib
import dataclasses
import math
import time
from dataclasses import dataclass
from datetime import datetime, timedelta
from random import Random
from typing import Callable, override

import pytz
from hypothesis import given
from hypothesis import strategies as st

from bench.test.strategies import DURATION_STRATEGY
from bench.utils.oracle import MAX_SCHEDULE_DURATION, REAL_ORACLE, Oracle


@dataclass(order=True, slots=True)
class _ScheduledCallable:
    when: float
    when_ns: int
    callback: Callable = dataclasses.field(compare=False)
    args: tuple = dataclasses.field(default_factory=tuple, compare=False)


class SimulatedOracle(Oracle):
    """A simulated oracle for deterministic 'randomness' and fast-forwardable timing."""

    def __init__(
        self,
        *,
        random: Random,
        base_ns: int | Callable[[], int],
        offset: timedelta | int | None,
    ):
        self._random = random
        self._base_ns = base_ns

        # init offset
        if isinstance(offset, timedelta):  # convert to ns
            self._offset_ns = (
                (offset.days * 24 * 60 * 60 + offset.seconds) * 10**6 + offset.microseconds
            ) * 10**3
        elif isinstance(offset, int):
            self._offset_ns = offset
        else:
            self._offset_ns = 0

        # scheduling
        self._scheduled_callbacks: asyncio.Queue[_ScheduledCallable] = asyncio.PriorityQueue()
        self._loop_task: asyncio.Task | None = None

    async def start(self):
        self._loop_task = asyncio.create_task(self._tick_forever())

    async def _tick_forever(self):
        while True:
            scheduled = await self._scheduled_callbacks.get()
            now_ns = self.time_ns()
            if scheduled.when_ns > now_ns:
                # jump forward in time
                self._offset_ns += scheduled.when_ns - now_ns
            asyncio.get_event_loop().call_soon(scheduled.callback, *scheduled.args)

    def close(self):
        if self._loop_task is not None:
            self._loop_task.cancel()

    async def wait_closed(self):
        if self._loop_task is not None:
            with contextlib.suppress(asyncio.CancelledError):
                await self._loop_task
            self._loop_task = None

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
        assert duration >= 0, f"invalid sleep duration: {duration}"
        event = asyncio.Event()
        self.call_later(duration, event.set)
        await event.wait()

    @override
    def call_later(self, duration: float, callback: Callable, *args) -> None:
        self.call_at(self.time() + duration, callback, *args)

    @override
    def call_at(self, when: float, callback: Callable, *args) -> None:
        duration = when - self.time()
        assert duration <= MAX_SCHEDULE_DURATION, f"call_at duration too long: {duration:.3f}s"
        scheduled = _ScheduledCallable(
            when=when, when_ns=int(when * 1e9), callback=callback, args=args
        )
        self._scheduled_callbacks.put_nowait(scheduled)

    @override
    def call_soon(self, callback: Callable, *args) -> None:
        # add to loop directly
        asyncio.get_event_loop().call_soon(callback, *args)


#
# Tests (testing the test simulation)
#


class Timer:
    """Simple Oracle-backed timer."""

    def __init__(self, oracle: Oracle) -> None:
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


@contextlib.asynccontextmanager
async def simulated_oracle():
    simulated_oracle = SimulatedOracle(random=Random(0), base_ns=time.time_ns, offset=0)
    await simulated_oracle.start()
    yield simulated_oracle
    simulated_oracle.close()
    await simulated_oracle.wait_closed()


@given(duration=DURATION_STRATEGY)
async def test_simulated_time_linear(duration: float):
    """Fast forward in a straight line."""
    async with simulated_oracle() as sim_oracle:
        with Timer(sim_oracle) as sim_timer, Timer(REAL_ORACLE) as real_timer:
            await sim_oracle.sleep(duration)
            assert math.isclose(sim_timer.elapsed, duration, abs_tol=0.01)
            assert math.isclose(real_timer.elapsed, 0, abs_tol=0.01)


@given(duration=DURATION_STRATEGY, splits=st.integers(min_value=1, max_value=100))
async def test_simulated_time_linear_cumulative(duration: float, splits: int):
    """Fast forward in a straight line split into arbitrary sub waits."""
    async with simulated_oracle() as sim_oracle:
        with Timer(sim_oracle) as sim_timer_overall, Timer(REAL_ORACLE) as real_timer_overall:
            duration_split = duration / splits
            for _ in range(0, splits):
                with Timer(sim_oracle) as sim_timer, Timer(REAL_ORACLE) as real_timer:
                    await sim_oracle.sleep(duration_split)
                    assert math.isclose(sim_timer.elapsed, duration_split, abs_tol=0.01)
                    assert math.isclose(real_timer.elapsed, 0, abs_tol=0.01)
            assert math.isclose(sim_timer_overall.elapsed, duration, abs_tol=0.01)
            assert math.isclose(real_timer_overall.elapsed, 0, abs_tol=0.01)


async def test_simulated_time_cursed():
    """Fast forward across multiple workers with different simulated oracles and different waiting patterns."""
    ...
