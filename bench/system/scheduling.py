import asyncio
from typing import TYPE_CHECKING, override

import structlog

from bench.language.bench import Bench
from bench.language.const import NodeType, RunStatus
from bench.language.run import Run
from bench.language.session import Session
from bench.system.core import Commit, HostPlugin, HostSpec
from bench.utils.func import bittuple

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)


class RunPlugin(HostPlugin[Run]):
    """Distribute new (and forgotten) unassigned Runs to Runtimes (on Machines)."""

    watch_types = bittuple(NodeType.RUN)

    def __init__(self, host: HostSpec, bench: Bench):
        super().__init__(host, bench)
        self._runs_to_queue: asyncio.Queue[Run] = asyncio.Queue()

    def __str__(self):
        return f"queue={self._runs_to_queue.qsize()}"

    @override
    async def start(self, session: Session) -> None:
        # TODO :Robustness: cancel/re-queue Runs stuck on dead Machines
        self._tasks.start_queue(self._runs_to_queue, self._process_run, skip_errors=True)

    @override
    def on_commit(self, commit: Commit[Run]) -> None:
        # queue any new runs
        for run in commit.added:
            if run.parent_type == NodeType.PACKAGE and run.status == RunStatus.SCHEDULED:
                self._enqueue_run(run)

    def _enqueue_run(self, run: Run) -> None:
        """Adds a run to the distribution queue"""
        logger.trace("host.enqueue_run", host=self, run=run)
        self._runs_to_queue.put_nowait(run)

    async def _process_run(self, run: Run) -> None:
        """Distributes runs to be queued in Runtimes. If no Machine is available, we start one."""
        # find machine for run
        assert run.package_id is not None, f"missing package id for run {run!r}"
        package = self._host.get_package(run.package_id)
        assert package is not None, f"missing package for run {run!r}"
        raise NotImplementedError("nocheckin")
