import asyncio
import enum
from dataclasses import dataclass
from datetime import datetime, timedelta
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
from bench.proto.services import get_channel
from bench.proto.wire import KillRunRequest, PauseRunRequest, ProcessRunRequest, RuntimeClient
from bench.system.host.core import Commit, DeferredHostPlugin, HostApi, HostPlugin
from bench.utils.func import bittuple
from bench.utils.tenacity import RETRY_GRPC, RetryOptions, RetryState

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class RunOperation(enum.Enum):
    START = 1
    PAUSE = 2
    RESUME = 3
    KILL = 4


@dataclass(slots=True)
class PendingRunOperation:
    op: RunOperation
    run: Run
    retry: RetryState
    retry_at: datetime | None = None
    is_cancelled: bool = False  # whether this operation was cancelled by a more recent one


class ProcessRunPlugin(HostPlugin[Run]):
    """Process Runs in appropriate Runtimes."""

    watch_types = bittuple(NodeType.RUN)

    def __init__(self, host: HostApi, bench: Bench, retry: RetryOptions = RETRY_GRPC):
        super().__init__(host, bench)
        self._retry = retry
        self._run_queue: asyncio.Queue[PendingRunOperation] = asyncio.Queue()

    def __str__(self):
        return f"queue={self._run_queue.qsize()}"

    @override
    async def start(self) -> None:
        # nocheckin: cancel (or re-queue?) forlorn Runs (like those 'stuck' on dead/restarted Machines)
        self.tasks.start_queue(self._run_queue, self._process_queue, skip_errors=True)

    def _queue_operation(self, op: RunOperation, run: Run) -> PendingRunOperation:
        """Queues a Run operation. Cancels any preceding operations for the same Run."""
        # cancel any previous operations for this run
        for pending_op in self._run_queue._queue:  # type: ignore
            if pending_op.run.id == run.id:
                pending_op.is_cancelled = True
                logger.trace(f"scheduler.{op.name.lower()}.cancel", host=self, run=run)

        # add new run
        pending_op = PendingRunOperation(op=op, run=run, retry=self._retry.new(self.host.oracle))
        self._run_queue.put_nowait(pending_op)
        logger.trace(f"scheduler.{op.name.lower()}.queue", host=self, run=run)
        return pending_op

    @override
    async def on_commit(self, session: Session, commit: Commit[Run]) -> None:
        # start new scheduled runs
        for run in commit.added:
            if run.parent_type == NodeType.PACKAGE and run.status == RunStatus.SCHEDULED:
                self._queue_operation(RunOperation.START, run)
        # kill active runs with killed_at
        for run in commit.updated:
            if run.killed_at and not run.status.is_terminal:
                self._queue_operation(RunOperation.KILL, run)

    @tracer.start_as_current_span("scheduler.process_run")
    async def _process_queue(self, op: PendingRunOperation) -> None:
        """Push Runs to relevant Machines."""
        op.retry.on_attempt()
        run = op.run
        assert run.package_id is not None, f"missing package id for run {run!r}"
        assert self.bench.main_server, f"missing main server for {self.bench!r}"
        op_name = op.op.name.lower()
        log = logger.bind(host=self, run=run, server=self.bench.main_server, retry=op.retry)

        # find machine to process run on
        # nocheckin: adapt this for different run ops (only start should select machines freely)
        for machine in self.bench.main_server.machines:
            if machine.current_status != ResourceStatus.UP:
                continue
            assert machine.connection_uri, f"missing connection uri for machine {machine!r}"
            channel = get_channel(machine.connection_uri)
            runtime = RuntimeClient(channel)
            try:
                if op.op == RunOperation.START or op.op == RunOperation.RESUME:
                    request = ProcessRunRequest(run=run._to_data(), is_blocking=False)
                    _ = await runtime.process_run(request)
                elif op.op == RunOperation.PAUSE:
                    request = PauseRunRequest(run=run._to_data())
                    _ = await runtime.pause_run(request)
                elif op.op == RunOperation.KILL:
                    request = KillRunRequest(run=run._to_data())
                    _ = await runtime.kill_run(request)
                log.debug(f"scheduler.{op_name}", machine=machine, span="current")
                return  # success
            except Exception as e:
                log.error(f"scheduler.{op_name}.error", machine=machine, error=e)
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
            async with self.host.session(commit=True):  # :StaleNodes
                run.status = RunStatus.ABORTED if op.op == RunOperation.KILL else RunStatus.FAILED
                run.error = error
            log.error(
                f"scheduler.{op_name}.failed",
                machines=self.bench.main_server.machines,
                error=error,
                span="current",
            )
        else:
            # retry later
            op.retry_at = self.host.oracle.utc() + timedelta(seconds=op.retry.get_wait_interval())
            self.host.oracle.call_at(
                when=op.retry_at.timestamp(),
                callback=lambda: op.is_cancelled or self._run_queue.put_nowait(op),
            )
            log.debug(
                f"scheduler.{op_name}.retry",
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
    """Process active Triggers according to their Schedule."""

    watch_types = bittuple(NodeType.TRIGGER)

    ...
