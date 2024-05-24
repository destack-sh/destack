import asyncio
from typing import TYPE_CHECKING

import structlog

from bench.language import Bench, Package
from bench.language.run import Run
from bench.utils.task import TaskManager

if TYPE_CHECKING:
    from bench.runtime.runtime import Runtime

logger = structlog.get_logger(__name__)


class RuntimeProcess:
    """A 'process' for actually executing Runs in a Runtime. May be an actual process."""

    def __init__(
        self,
        id: int,
        runtime: "Runtime",
        bench: "Bench",
        package: "Package",
        queue: asyncio.Queue[Run],
    ):
        self.id = id
        self._runtime = runtime
        self._bench = bench
        self._package = package
        self._queue = queue
        self._tasks = TaskManager(owner=self, logger=logger)

    def __str__(self):
        return f"{self.id} in {self._runtime!r}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    async def start(self):
        self._tasks.start_queue(self._queue, self._process_run, f"run{self.id}", skip_errors=True)

    async def _process_run(self, run: Run):
        raise NotImplementedError(f"nocheckin: _process_run {run!r}")

    def close(self):
        self._tasks.close()

    async def wait_closed(self):
        await self._tasks.wait_closed()
