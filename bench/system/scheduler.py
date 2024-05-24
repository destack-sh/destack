import asyncio
from dataclasses import dataclass
from typing import TYPE_CHECKING, override

import structlog

from bench.language.bench import Bench, ResourceStatus
from bench.language.const import NodeType, RunStatus
from bench.language.run import Run, RunError, RunErrorKind, RunErrorType
from bench.language.session import Session
from bench.proto.services import get_channel_cached
from bench.proto.wire import QueueRunRequest, RuntimeStub
from bench.system.core import Commit, HostPlugin, HostSpec
from bench.utils.dt import monotime
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
    """Distribute new (and forlorn) unassigned Runs to Runtimes (on Machines)."""

    watch_types = bittuple(NodeType.RUN)

    def __init__(self, host: HostSpec, bench: Bench, retry: RetryOptions = RetryOptions()):
        super().__init__(host, bench)
        self._retry = retry
        self._runs_to_queue: asyncio.Queue[QueueAttempt] = asyncio.Queue()

    def __str__(self):
        return f"queue={self._runs_to_queue.qsize()}"

    @override
    async def start(self) -> None:
        # TODO :Robustness: cancel/re-queue Runs stuck on dead Machines
        self._tasks.start_queue(self._runs_to_queue, self._queue_run, skip_errors=True)

    @override
    async def on_commit(self, session: Session, commit: Commit[Run]) -> None:
        # queue any new runs
        for run in commit.added:
            if run.parent_type == NodeType.PACKAGE and run.status == RunStatus.SCHEDULED:
                logger.trace("scheduler.add", host=self, run=run)
                attempt = QueueAttempt(run=run, no=0)
                self._runs_to_queue.put_nowait(attempt)

    async def _queue_run(self, attempt: QueueAttempt) -> None:
        """Distributes Runs to be queued in Runtimes."""
        attempt.no += 1
        start = monotime()
        run = attempt.run
        assert run.package_id is not None, f"missing package id for run {run!r}"
        assert self._bench.main_environment, f"missing main environment for bench {self._bench!r}"

        # find machine to queue run on
        error = None
        for machine in self._bench.main_environment.server.machines:
            if machine.status != ResourceStatus.HEALTHY:
                continue
            assert machine.connection_uri, f"missing connection uri for machine {machine!r}"
            channel = get_channel_cached(machine.connection_uri)
            runtime = RuntimeStub(channel)
            request = QueueRunRequest(run=run._to_data())
            try:
                _ = await runtime.queue_run(request)
                logger.debug("scheduler.queue", host=self, run=run, duration=monotime() - start)
                return  # success
            except Exception as error:
                logger.error("scheduler.queue.error", host=self, run=run, error=error)
                continue

        # failed to queue run
        if self._retry.max_attempts > 0 and attempt.no >= self._retry.max_attempts:
            # give up and mark run as failed
            error = RunError(kind=RunErrorKind.INTERNAL, type=RunErrorType.NO_RUNTIME_AVAILABLE)
            async with self._host.session(autocommit=True):
                run.fail(error)
            logger.error("scheduler.queue.failed", host=self, run=run, error=error)
        else:
            # retry run later
            interval = self._retry.get_interval(attempt.no)
            asyncio.get_event_loop().call_later(interval, self._runs_to_queue.put_nowait, attempt)
            logger.debug(
                "scheduler.queue.retry", host=self, run=run, interval=interval, error=error
            )
