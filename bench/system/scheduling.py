import asyncio
from typing import TYPE_CHECKING, override

import structlog

from bench.language.bench import Bench
from bench.language.const import NodeType, RunStatus
from bench.language.run import Run
from bench.system.core import GraphDiff, HostPlugin
from bench.utils.func import bittuple
from bench.utils.task import TaskManager

if TYPE_CHECKING:
    from bench.system.host import Host

logger = structlog.get_logger(__name__)


class RunPlugin(HostPlugin[Run]):
    node_types = bittuple(NodeType.RUN)

    def __init__(self, host: "Host", bench: Bench):
        super().__init__(bench)
        self._host = host
        self._runs_to_queue: asyncio.Queue[Run] = asyncio.Queue()

    def __str__(self):
        return f"queue={self._runs_to_queue.qsize()}"

    @override
    async def start(self, tasks: TaskManager) -> None:
        # TODO :Robustness: cancel/re-queue Runs stuck on dead Machines
        tasks.start_queue(self._runs_to_queue, self._process_run)

    @override
    async def on_graph_commit(self, diff: GraphDiff[Run]) -> None:
        # queue any new runs
        for run in diff.added:
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
        package = self._host._packages.get(run.package_id)
        assert package is not None, f"missing package for run {run!r}"
        raise NotImplementedError("nocheckin")
