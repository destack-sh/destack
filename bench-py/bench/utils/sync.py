import asyncio
import traceback
from contextlib import asynccontextmanager
from time import time_ns
from typing import Literal, assert_never

import structlog
from opentelemetry import trace

from bench.utils.env import IS_DEV, IS_TEST, get_from_env

TRACE_LOCKS = get_from_env(
    "TRACE_LOCKS",
    typ=bool,
    default=False,
    description="Whether to trace critical lock acquisition/release",
)
CRITICAL_LOCK_TIMEOUT = get_from_env(
    "CRITICAL_LOCK_TIMEOUT",
    typ=int,
    default=10,
    description="Timeout for critical locks in seconds",
)

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class TracedLock(asyncio.Lock):
    def __init__(self, name: str):
        super().__init__()
        self._name = name

    async def acquire(self):
        with tracer.start_as_current_span(self._name):
            return await super().acquire()


class RWLock:
    """
    An asynchronous read-write lock allowing multiple readers or one writer.
    Writers are given preference to prevent writer starvation.
    """

    def __init__(self):
        self._lock = asyncio.Lock()
        self._readers = 0
        self._writers = 0
        self._write_requests = 0
        self._readers_ok = asyncio.Condition(self._lock)
        self._writers_ok = asyncio.Condition(self._lock)

    @asynccontextmanager
    async def hold(self, mode: Literal["read", "write"]):
        if mode == "read":
            await self.acquire_read()
            try:
                yield
            finally:
                await self.release_read()
        elif mode == "write":
            await self.acquire_write()
            try:
                yield
            finally:
                await self.release_write()
        else:
            assert_never(mode)

    @asynccontextmanager
    async def read(self):
        await self.acquire_read()
        try:
            yield
        finally:
            await self.release_read()

    @asynccontextmanager
    async def write(self):
        await self.acquire_write()
        try:
            yield
        finally:
            await self.release_write()

    async def acquire_read(self):
        async with self._lock:
            while self._writers > 0 or self._write_requests > 0:
                await self._readers_ok.wait()
            self._readers += 1

    async def release_read(self):
        async with self._lock:
            self._readers -= 1
            if self._readers == 0 and self._write_requests > 0:
                self._writers_ok.notify()

    async def acquire_write(self):
        async with self._lock:
            self._write_requests += 1
            while self._readers > 0 or self._writers > 0:
                await self._writers_ok.wait()
            self._write_requests -= 1
            self._writers += 1

    async def release_write(self):
        async with self._lock:
            self._writers -= 1
            if self._write_requests > 0:
                self._writers_ok.notify()
            else:
                self._readers_ok.notify_all()


class CriticalLock(asyncio.Lock):
    """
    A smarter asyncio.Lock that remembers who acquired it & supports timeouts for critical sections.
    """

    def __init__(
        self,
        name: str,
        track_acquirer: bool = IS_DEV or IS_TEST,
        timeout: float = CRITICAL_LOCK_TIMEOUT,
    ):
        super().__init__()
        self._name = f"{name}_{id(self):x}"
        self._track_acquirer = track_acquirer
        self._timeout = timeout
        self._acquired_by = None
        self._acquired_at: float | None = None

    async def acquire(self):
        if TRACE_LOCKS:
            logger.trace("lock.acquire.wait", name=self._name)

        try:
            await asyncio.wait_for(super().acquire(), timeout=self._timeout)
        except asyncio.TimeoutError as e:
            if self._acquired_by:
                # prune _pytest, pluggy, asyncio from traceback
                filtered_tb = [
                    frame
                    for frame in self._acquired_by
                    if not any(m in frame.filename for m in ["pytest", "pluggy", "asyncio"])
                ]
                pretty_tb = "\n" + "\n".join(traceback.format_list(filtered_tb))
                logger.error(
                    "lock.timeout",
                    name=self._name,
                    acquirer=pretty_tb,
                    acquired_at=self._acquired_at,
                )
            raise TimeoutError(f"lock {self._name} timed out after {self._timeout}s") from e

        if self._track_acquirer:
            self._acquired_by = traceback.extract_stack()[:-1]
        self._acquired_at = time_ns()
        if TRACE_LOCKS:
            logger.trace("lock.acquire.success", name=self._name, acquired_at=self._acquired_at)
        return True

    def release(self):
        super().release()
        if TRACE_LOCKS:
            logger.trace("lock.release", name=self._name, acquired_at=self._acquired_at)
        if self._track_acquirer:
            self._acquired_by = None
            self._acquired_at = None
