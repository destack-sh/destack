import asyncio
import contextlib
import dataclasses
import math
import time
from dataclasses import dataclass
from datetime import datetime, timedelta
from random import Random
from typing import Callable, Union, override

import pytest
import pytz
import uvloop
from hypothesis import example, given, reject
from hypothesis import strategies as st

from bench.test.strategies import DURATION_STRATEGY
from bench.utils.oracle import MAX_SCHEDULE_DURATION, REAL_ORACLE, Oracle, Timer

TimeBaseNs = Union[int, Callable[[], int]]


@dataclass(order=True, slots=True)
class _ScheduledCallable:
    when: float
    when_ns: int
    callback: Callable = dataclasses.field(compare=False)
    args: tuple = dataclasses.field(default_factory=tuple, compare=False)


class SimulatedLoop:
    """Simulated loop to progress the 'dynamic' offsets of oracles for fast-forwarding."""

    def __init__(self, base_time_ns: TimeBaseNs):
        self._scheduled_callbacks: asyncio.Queue[_ScheduledCallable] = asyncio.PriorityQueue()
        self._loop_task: asyncio.Task | None = None
        self._base_time_ns = base_time_ns
        self._dynamic_offset_ns = 0

    def time_ns(self) -> int:
        base_time_ns = (
            self._base_time_ns if isinstance(self._base_time_ns, int) else self._base_time_ns()
        )
        return base_time_ns + self._dynamic_offset_ns

    async def start(self):
        self._loop_task = asyncio.create_task(self._tick_forever())

    async def _tick_forever(self):
        while True:
            scheduled = await self._scheduled_callbacks.get()
            now_ns = self.time_ns()
            if scheduled.when_ns > now_ns:
                # jump forward in time
                self._dynamic_offset_ns += scheduled.when_ns - now_ns
            scheduled.callback(*scheduled.args)
            # yield to allow other tasks to run
            #  (otherwise we work through all scheduled callbacks at once,
            #    unexpectedly forwarding time more than calling code expects)
            await asyncio.sleep(0)

    def close(self):
        if self._loop_task is not None:
            self._loop_task.cancel()

    async def wait_closed(self):
        if self._loop_task is not None:
            with contextlib.suppress(asyncio.CancelledError):
                await self._loop_task
            self._loop_task = None

    def schedule(self, when: float, callback: Callable, *args):
        scheduled = _ScheduledCallable(when, int(when * 1e9), callback, args)
        self._scheduled_callbacks.put_nowait(scheduled)


class SimulatedOracle(Oracle):
    """A simulated oracle for deterministic 'randomness' and fast-forwardable timing."""

    __slots__ = ("_base_ns", "_loop", "_random", "_static_offset_ns")

    def __init__(
        self,
        *,
        random: Random,
        static_offset: timedelta | int | None,
        loop: SimulatedLoop,
    ):
        self._random = random
        self._base_ns = loop._base_time_ns
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
        base_ns = self._base_ns if isinstance(self._base_ns, int) else self._base_ns()
        return base_ns + self._static_offset_ns + self._loop._dynamic_offset_ns

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
        # time includes oracle's static offset, so normalize to loop time
        when_loop = when - (self._static_offset_ns / 1e9)
        self._loop.schedule(when_loop, callback, *args)

    @override
    def call_soon(self, callback: Callable, *args) -> None:
        # add to loop directly
        asyncio.get_event_loop().call_soon(callback, *args)


#
# Tests (testing the timing simulation)
# NOTE: we care so much about this being right that we run it in all event loops.
#


@pytest.fixture(
    params=(
        uvloop.EventLoopPolicy(),
        asyncio.DefaultEventLoopPolicy(),
    )
)
def event_loop_policy(request: pytest.FixtureRequest):
    # see https://pytest-asyncio.readthedocs.io/en/latest/how-to-guides/multiple_loops.html
    return request.param


@contextlib.asynccontextmanager
async def simulated_loop():
    loop = SimulatedLoop(base_time_ns=time.time_ns)
    await loop.start()
    yield loop
    loop.close()
    await loop.wait_closed()


@given(duration=DURATION_STRATEGY)
async def test_simulated_time_linear(duration: float):
    """Fast forward in a straight line."""
    async with simulated_loop() as sim_loop:
        sim_oracle = SimulatedOracle(random=Random(), static_offset=0, loop=sim_loop)
        with Timer(sim_oracle) as sim_timer, Timer(REAL_ORACLE) as real_timer:
            await sim_oracle.sleep(duration)
            assert math.isclose(sim_timer.elapsed, duration, abs_tol=0.01)
            assert math.isclose(real_timer.elapsed, 0, abs_tol=0.01)


@given(data=st.data(), duration=DURATION_STRATEGY, splits=st.integers(min_value=1, max_value=100))
async def test_simulated_time_linear_cumulative(data: st.DataObject, duration: float, splits: int):
    """Fast forward in a straight line split into arbitrary sub waits."""
    duration_split = duration / splits
    async with simulated_loop() as sim_loop:
        sim_oracle = SimulatedOracle(random=Random(), static_offset=0, loop=sim_loop)
        with Timer(sim_oracle) as sim_timer_overall, Timer(REAL_ORACLE) as real_timer_overall:
            for _ in range(0, splits):
                with Timer(sim_oracle) as sim_timer, Timer(REAL_ORACLE) as real_timer:
                    await sim_oracle.sleep(duration_split)
                assert math.isclose(sim_timer.elapsed, duration_split, abs_tol=0.01)
                assert math.isclose(real_timer.elapsed, 0, abs_tol=0.01)
        assert math.isclose(sim_timer_overall.elapsed, duration, abs_tol=0.01)
        assert math.isclose(real_timer_overall.elapsed, 0, abs_tol=0.01)


@dataclass
class _WorkerSpec:
    duration_splits: list[float]
    static_offset_ns: int


@st.composite
def _worker_specs(draw: Callable) -> _WorkerSpec:
    splits = draw(st.lists(DURATION_STRATEGY, min_size=1, max_size=10))
    if sum(splits) >= MAX_SCHEDULE_DURATION:
        reject()
    static_offset_ns = draw(st.integers(min_value=0, max_value=10**6))
    return _WorkerSpec(duration_splits=splits, static_offset_ns=static_offset_ns)


@given(worker_specs=st.lists(_worker_specs(), min_size=1, max_size=10))
@example(
    worker_specs=[
        _WorkerSpec(duration_splits=[0.0], static_offset_ns=0),
        _WorkerSpec(duration_splits=[1.0], static_offset_ns=0),
    ]
)
async def test_simulated_time_concurrent(worker_specs: list[_WorkerSpec]):
    """
    Fast forward across multiple workers with different simulated oracles and different waiting patterns.
    Also checks that this is deterministic by running it multiple times.
    """

    class _Worker:
        def __init__(self, oracle: Oracle, duration_splits: list[float]) -> None:
            self.oracle = oracle
            self.duration_splits = duration_splits

        async def run(self):
            for split_duration in self.duration_splits:
                with Timer(self.oracle) as sim_timer, Timer(REAL_ORACLE) as real_timer:
                    await self.oracle.sleep(split_duration)
                assert math.isclose(sim_timer.elapsed, split_duration, abs_tol=0.01)
                assert math.isclose(real_timer.elapsed, 0, abs_tol=0.01)

    async with simulated_loop() as sim_loop:
        # setup workers
        workers: list[_Worker] = []
        for worker_spec in worker_specs:
            sim_oracle = SimulatedOracle(
                random=Random(), static_offset=worker_spec.static_offset_ns, loop=sim_loop
            )
            worker = _Worker(sim_oracle, worker_spec.duration_splits)
            workers.append(worker)

        # run workers
        await asyncio.gather(*(worker.run() for worker in workers))
