import asyncio
import random
from dataclasses import dataclass
from datetime import datetime, timedelta
from typing import TYPE_CHECKING, override

import structlog
from more_itertools import first
from opentelemetry import trace

from bench.language.bench import Bench, ResourceStatus
from bench.language.const import NodeType, RunStatus
from bench.language.machine import Machine, MachineType
from bench.language.message import Message
from bench.language.run import Run, RunError, RunErrorKind, RunErrorType
from bench.language.session import Session
from bench.language.text import Text
from bench.language.trigger import Trigger
from bench.proto.services import get_channel
from bench.proto.wire import RunRequest, RuntimeClient
from bench.system.host.core import Commit, DeferredHostPlugin, Host, HostPlugin
from bench.utils.func import bittuple
from bench.utils.tenacity import RETRY_GRPC, RetryOptions, RetryState

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass(slots=True)
class PendingRunOperation:
    run: Run
    retry: RetryState
    retry_at: datetime | None = None
    is_cancelled: bool = False  # whether this operation was overridden by a more recent one


class RunPlugin(HostPlugin[Run]):
    """Process Runs in appropriate Runtimes."""

    watch_types = bittuple(NodeType.RUN)

    def __init__(self, host: Host, bench: Bench, retry: RetryOptions = RETRY_GRPC):
        super().__init__(host, bench)
        self._retry = retry
        self._run_queue: asyncio.Queue[PendingRunOperation] = asyncio.Queue()

    def __str__(self):
        return f"queue={self._run_queue.qsize()}"

    @override
    async def start(self) -> None:
        # TODO :Robustness: kill/re-queue abandoned Runs
        #  (like Runs 'stuck' on dead or since restarted Machines)
        self.tasks.start_queue(self._run_queue, self._process_queue, skip_errors=True)

    def _queue_run(self, run: Run) -> PendingRunOperation:
        """Queues a Run operation."""
        # queue new operation
        pending_op = PendingRunOperation(run=run, retry=self._retry.new(self.host.oracle))
        self._run_queue.put_nowait(pending_op)
        logger.trace("scheduler.run.queue", host=self, run=run)
        return pending_op

    @override
    async def on_commit(self, session: Session, commit: Commit[Run]) -> None:
        for run in commit.added:
            # start new scheduled runs
            if run.status == RunStatus.SCHEDULED and run.parent_type == NodeType.BENCH:
                self._queue_run(run)
        for run in commit.updated:
            # resume active runs (at root)
            if (
                run.resumed_at is not None
                and run.interrupted_at is not None
                and run.resumed_at > run.interrupted_at
            ):
                self._queue_run(run.root or run)

    @tracer.start_as_current_span("scheduler.process_run")
    async def _process_queue(self, op: PendingRunOperation) -> None:
        """Push Runs to relevant Machines."""
        op.retry.on_attempt()
        run = op.run
        assert self.bench.main_server, f"missing main server for {self.bench!r}"
        log = logger.bind(host=self, run=run, server=self.bench.main_server, retry=op.retry)

        # select machines to process run on
        # NOTE :Performance: maybe not re-load available Machines in RunPlugin every time?
        available_machines = (
            await Machine.where(
                Machine.get_property("parent").eq(self.bench.main_server)
                & Machine.get_property("status").eq(ResourceStatus.UP)
                & Machine.get_property("type").eq(MachineType.RUNTIME)
            )
            .select_all()
            .to_list()
        )
        # if last machine is still available, use that
        existing_machine = first((m for m in available_machines if m.id == run.machine_id), None)
        if existing_machine:
            candidate_machines = [existing_machine]
        else:
            # otherwise, pick any available machine
            candidate_machines = available_machines
            random.shuffle(candidate_machines)

        # contact machines
        for machine in candidate_machines:
            try:
                assert machine.connection_uri, f"missing connection uri for machine {machine!r}"
                runtime = RuntimeClient(get_channel(machine.connection_uri))
                request = RunRequest(run_ptr=run._to_ref_data(), is_blocking=False)
                _ = await runtime.run(request)
                log.debug("scheduler.run", machine=machine, span="current")
                return  # success
            except Exception as e:
                log.error("scheduler.run.error", machine=machine, error=e)
                op.retry.on_error(e)
                continue

        # failed to process run
        if not op.retry.should_retry:
            # give up
            error = RunError(
                kind=RunErrorKind.RUNTIME,
                type=RunErrorType.RUNTIME_UNAVAILABLE,
                title="Failed to queue run",
                text=Text.from_markdown("Could not reach any applicable Machine."),
            )
            async with self.host.session(commit=True):  # :StaleNodes
                run.status = RunStatus.ABORTED if run.started_at else RunStatus.CANCELLED
                run.terminated_at = self.host.oracle.utc()
                if run.started_at is not None:
                    run.duration = run.terminated_at - run.started_at
                run.error = error
            log.error(
                "scheduler.run.failed",
                server=self.bench.main_server,
                machines=available_machines,
                error=error,
                span="current",
            )
        else:
            # retry later
            retry_interval = op.retry.get_wait_interval()
            op.retry_at = self.host.oracle.utc() + timedelta(seconds=retry_interval)
            self.host.oracle.call_later(
                delay=retry_interval,
                callback=lambda: op.is_cancelled or self._run_queue.put_nowait(op),
            )
            log.trace(
                "scheduler.run.retry",
                server=self.bench.main_server,
                machines=available_machines,
                interval=op.retry.get_wait_interval,
                retry=op.retry,
                span="current",
            )


class MessageTriggerPlugin(DeferredHostPlugin[Message | Trigger]):
    """Process active Triggers when they receive Messages."""

    watch_types = bittuple(NodeType.MESSAGE, NodeType.TRIGGER)

    ...


class ScheduleTriggerPlugin(DeferredHostPlugin[Trigger]):
    """Process active Triggers according to their Schedule."""

    watch_types = bittuple(NodeType.TRIGGER)

    ...
