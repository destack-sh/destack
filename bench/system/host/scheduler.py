import asyncio
from dataclasses import dataclass
from typing import TYPE_CHECKING, override

import structlog
from opentelemetry import trace

from bench.language.bench import Bench, ResourceStatus
from bench.language.const import NodeType, RunStatus
from bench.language.run import Run, RunError, RunErrorKind, RunErrorType
from bench.language.session import Session
from bench.language.signal import Signal
from bench.language.text import Text
from bench.language.trigger import Trigger
from bench.proto.services import get_channel_cached
from bench.proto.wire import QueueRunRequest, RuntimeClient
from bench.system.host.core import Commit, DeferredHostPlugin, HostApi, HostPlugin
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
    """Queue new (and forlorn) unassigned Runs in Runtimes (on Machines)."""

    watch_types = bittuple(NodeType.RUN)

    def __init__(self, host: HostApi, bench: Bench, retry: RetryOptions = RETRY_GRPC):
        super().__init__(host, bench)
        self._retry = retry
        self._runs_to_queue: asyncio.Queue[QueueOperation] = asyncio.Queue()

    def __str__(self):
        return f"queue={self._runs_to_queue.qsize()}"

    @override
    async def start(self) -> None:
        # TODO :Robustness: cancel/re-queue forlorn Runs (like those 'stuck' on dead Machines)
        self.tasks.start_queue(self._runs_to_queue, self._queue_run, skip_errors=True)

    @override
    async def on_commit(self, session: Session, commit: Commit[Run]) -> None:
        # queue any new runs
        for run in commit.added:
            if run.parent_type == NodeType.PACKAGE and run.status == RunStatus.SCHEDULED:
                logger.trace("scheduler.add", host=self, run=run)
                attempt = QueueOperation(run=run, retry=self._retry.new(self.host.oracle))
                self._runs_to_queue.put_nowait(attempt)

    @tracer.start_as_current_span("scheduler.queue_run")
    async def _queue_run(self, op: QueueOperation) -> None:
        """Distributes Runs to be queued in Runtimes."""
        op.retry.on_attempt()
        run = op.run
        assert run.package_id is not None, f"missing package id for run {run!r}"
        assert self.bench.main_server, f"missing main server for {self.bench!r}"
        log = logger.bind(host=self, run=run, server=self.bench.main_server, retry=op.retry)

        # find machine to queue run on
        for machine in self.bench.main_server.machines:
            if machine.status != ResourceStatus.READY:
                continue
            assert machine.connection_uri, f"missing connection uri for machine {machine!r}"
            channel = get_channel_cached(machine.connection_uri)
            runtime = RuntimeClient(channel)
            request = QueueRunRequest(run=run._to_data())
            try:
                _ = await runtime.queue_run(request)
                log.debug("scheduler.queue", machine=machine, span="current")
                return  # success!
            except Exception as e:
                log.error("scheduler.queue.error", machine=machine, error=e)
                op.retry.on_error(e)
                continue

        # failed to queue run
        if not op.retry.should_retry:
            # give up and mark run as failed
            error = RunError(
                kind=RunErrorKind.RUNTIME,
                type=RunErrorType.RUNTIME_UNAVAILABLE,
                title="Failed to queue run",
                text=Text.from_markdown("Could not reach any currently available machine."),
            )
            async with self.host.session(commit=True):
                run.status = RunStatus.FAILED
                run.error = error
            log.error(
                "scheduler.queue.failed",
                machines=self.bench.main_server.machines,
                error=error,
                span="current",
            )
        else:
            # retry run later
            asyncio.get_event_loop().call_later(
                op.retry.get_wait_interval(), self._runs_to_queue.put_nowait, op
            )
            log.debug(
                "scheduler.queue.retry",
                machines=self.bench.main_server.machines,
                interval=op.retry.get_wait_interval,
                retry=op.retry,
                span="current",
            )


class SignalTriggerPlugin(DeferredHostPlugin[Signal | Trigger]):
    """Process active Triggers when they receive Signals."""

    watch_types = bittuple(NodeType.SIGNAL, NodeType.TRIGGER)

    ...


class ScheduleTriggerPlugin(DeferredHostPlugin[Trigger]):
    """Process active Triggers when their Schedule fires."""

    watch_types = bittuple(NodeType.TRIGGER)

    ...
