import asyncio
from contextlib import asynccontextmanager
from typing import Literal, assert_never

import structlog
from opentelemetry import trace

from destack.utils.env import get_from_env

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
