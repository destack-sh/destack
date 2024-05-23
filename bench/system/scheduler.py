import asyncio
from dataclasses import dataclass
from typing import TYPE_CHECKING, override

import structlog
from more_itertools import first

from bench.language.bench import Bench, ResourceStatus
from bench.language.const import NodeType, RunStatus
from bench.language.run import Run, RunError, RunErrorKind, RunErrorType
from bench.language.session import Session
from bench.system.core import Commit, HostPlugin, HostSpec
from bench.utils.func import bittuple
from bench.utils.tenacity import RetryOptions

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)


@dataclass(slots=True)
class QueueAttempt:
    """Wrapper for Run to track queue attempts."""

    run: Run
    no: int


class QueueRunPlugin(HostPlugin[Run]):
    """Distribute new (and forgotten) unassigned Runs to Runtimes (on Machines)."""

    watch_types = bittuple(NodeType.RUN)

    def __init__(self, host: HostSpec, bench: Bench, retry: RetryOptions = RetryOptions()):
        super().__init__(host, bench)
        self._retry = retry
        self._runs_to_queue: asyncio.Queue[QueueAttempt] = asyncio.Queue()

    def __str__(self):
        return f"queue={self._runs_to_queue.qsize()}"

    @override
    async def start(self, session: Session) -> None:
        # TODO :Robustness: cancel/re-queue Runs stuck on dead Machines
        self._tasks.start_queue(self._runs_to_queue, self._queue_run, skip_errors=True)

    @override
    async def on_commit(self, session: Session, commit: Commit[Run]) -> None:
        # queue any new runs
        for run in commit.added:
            if run.parent_type == NodeType.PACKAGE and run.status == RunStatus.SCHEDULED:
                logger.trace("host.enqueue_run", host=self, run=run)
                attempt = QueueAttempt(run=run, no=0)
                self._runs_to_queue.put_nowait(attempt)

    async def _queue_run(self, attempt: QueueAttempt) -> None:
        """Distributes Runs to be queued in Runtimes."""
        attempt.no += 1
        run = attempt.run
        assert run.package_id is not None, f"missing package id for run {run!r}"

        # find machine to run on
        package = self._host.get_package(run.package_id)
        assert package is not None, f"missing package for run {run!r}"
        machine = first(
            (m for m in package.environment.server.machines if m.status == ResourceStatus.HEALTHY),
            None,
        )
        if machine is None:
            if self._retry.max_attempts > 0 and attempt.no >= self._retry.max_attempts:
                # nocheckin: fail run
                error = RunError(kind=RunErrorKind.INTERNAL, type=RunErrorType.NO_RUNTIME_AVAILABLE)
                run.fail(error)
            else:
                # retry run later
                interval = self._retry.get_interval(attempt.no)
                asyncio.get_event_loop().call_later(
                    interval, self._runs_to_queue.put_nowait, attempt
                )
                logger.debug("host.retry_queue", host=self, run=run, interval=interval)
        else:
            # queue run
            # nocheckin: send run to machine (cache channel?)
            pass
