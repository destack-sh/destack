import asyncio
from dataclasses import dataclass
from typing import TYPE_CHECKING, override

import structlog
from opentelemetry import trace

from bench.language.bench import Bench, ResourceStatus
from bench.language.const import NodeType, RunStatus
from bench.language.run import Run, RunError, RunErrorKind, RunErrorType
from bench.language.session import Session
from bench.proto.services import get_channel_cached
from bench.proto.wire import QueueRunRequest, RuntimeStub
from bench.system.core import Commit, HostPlugin, HostSpec
from bench.utils.func import bittuple
from bench.utils.tenacity import RETRY_GRPC, RetryOptions, RetryState

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass(slots=True)
class QueueOperation:
    """Wrapper for Run to track queue attempts."""

    run: Run
    retry: RetryState


class QueueRunPlugin(HostPlugin[Run]):
    """Distribute new (and forlorn) unassigned Runs to Runtimes (on Machines)."""

    watch_types = bittuple(NodeType.RUN)

    def __init__(self, host: HostSpec, bench: Bench, retry: RetryOptions = RETRY_GRPC):
        super().__init__(host, bench)
        self._retry = retry
        self._runs_to_queue: asyncio.Queue[QueueOperation] = asyncio.Queue()

    def __str__(self):
        return f"queue={self._runs_to_queue.qsize()}"

    @override
    async def start(self) -> None:
        # TODO :Robustness: cancel/re-queue forlorn Runs (like those 'stuck' on dead Machines)
        self._tasks.start_queue(self._runs_to_queue, self._queue_run, skip_errors=True)

    @override
    async def on_commit(self, session: Session, commit: Commit[Run]) -> None:
        # queue any new runs
        for run in commit.added:
            if run.parent_type == NodeType.PACKAGE and run.status == RunStatus.SCHEDULED:
                logger.trace("scheduler.add", host=self, run=run)
                attempt = QueueOperation(run=run, retry=self._retry.new())
                self._runs_to_queue.put_nowait(attempt)

    @tracer.start_as_current_span("scheduler.queue_run")
    async def _queue_run(self, op: QueueOperation) -> None:
        """Distributes Runs to be queued in Runtimes."""
        op.retry.on_attempt()
        run = op.run
        assert run.package_id is not None, f"missing package id for run {run!r}"
        environment = self._bench.main_environment
        assert environment, f"missing main environment for bench {self._bench!r}"
        log = logger.bind(host=self, run=run, server=environment.server, retry=op.retry)

        # find machine to queue run on
        for machine in environment.server.machines:
            if machine.status != ResourceStatus.HEALTHY:
                continue
            assert machine.connection_uri, f"missing connection uri for machine {machine!r}"
            channel = get_channel_cached(machine.connection_uri)
            runtime = RuntimeStub(channel)
            request = QueueRunRequest(run=run._to_data())
            try:
                _ = await runtime.queue_run(request)
                log.debug("scheduler.queue", machine=machine)
                return  # success!
            except Exception as e:
                log.error("scheduler.queue.error", machine=machine, error=e)
                op.retry.on_error(e)
                continue

        # failed to queue run
        if not op.retry.should_retry:
            # give up and mark run as failed
            error = RunError(kind=RunErrorKind.INTERNAL, type=RunErrorType.NO_RUNTIME_AVAILABLE)
            async with self._host.session(autocommit=True):
                run.fail(error)
            log.error("scheduler.queue.failed", machines=environment.server.machines, error=error)
        else:
            # retry run later
            asyncio.get_event_loop().call_later(
                op.retry.interval, self._runs_to_queue.put_nowait, op
            )
            log.debug(
                "scheduler.queue.retry",
                machines=environment.server.machines,
                interval=op.retry.interval,
                retry=op.retry,
            )
