import asyncio
import random
from dataclasses import dataclass
from datetime import datetime, timedelta
from typing import TYPE_CHECKING, override

import structlog
from more_itertools import first
from opentelemetry import trace

from bench.language import (
    Bench,
    Error,
    ErrorKind,
    ErrorType,
    Machine,
    MachineType,
    NodeType,
    ResourceStatus,
    Run,
    RunStatus,
    Session,
    Text,
    bittuple,
    isolated_graph,
)
from bench.proto import RunRequest, RuntimeClient
from bench.system.host.core import Commit, HostPlugin
from bench.utils.tenacity import RETRY_GRPC, RetryOptions, RetryState

if TYPE_CHECKING:
    from bench.system.host import HostService

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass(slots=True)
class _RunHandle:
    run: Run
    retry: RetryState
    retry_at: datetime | None = None
    is_cancelled: bool = False


class RunPlugin(HostPlugin[Run]):
    """Process Runs in appropriate Runtimes."""

    watch_types = bittuple(NodeType.RUN)

    def __init__(self, host: "HostService", bench: Bench, retry: RetryOptions = RETRY_GRPC):
        super().__init__(host, bench)
        self._retry = retry
        self._run_queue: asyncio.Queue[_RunHandle] = asyncio.Queue()

    def __str__(self):
        return f"queue={self._run_queue.qsize()}"

    @override
    async def start(self) -> None:
        # TODO :Robustness: kill/re-queue abandoned Runs
        #  (like Runs 'stuck' on dead or since restarted Machines)
        self.tasks.start_queue(self._run_queue, self._process_run, skip_errors=True)

    def _queue_run(self, run: Run) -> _RunHandle:
        """Queues a Run operation."""
        # queue new operation
        pending_op = _RunHandle(run=run, retry=self._retry.new(self.host.oracle))
        self._run_queue.put_nowait(pending_op)
        logger.trace("run_plugin.run.queue", host=self, run=run)
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

    @tracer.start_as_current_span("run_plugin.process_run")
    @isolated_graph()
    async def _process_run(self, op: _RunHandle) -> None:
        """Push Runs to relevant Machines."""
        op.retry.on_attempt()
        run = op.run
        log = logger.bind(host=self, run=run, retry=op.retry)

        # select machines to process run on
        # NOTE :Performance: maybe not re-load available Machines in RunPlugin every time?
        available_machines = (
            await Machine.where(
                Machine.get_property("parent").eq(self.bench)
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
                runtime = RuntimeClient(
                    await self.network.get_channel(machine.connection_uri, source_id=self.host.id)
                )
                request = RunRequest(run_ptr=run._to_ref_data(), is_blocking=False)
                _ = await runtime.run(request)
                log.debug("run_plugin.run", machine=machine, span="current")
                return  # success
            except Exception as e:
                log.error("run_plugin.run.error", machine=machine, error=e)
                op.retry.on_error(e)
                continue

        # failed to process run
        if not op.retry.should_retry:
            # give up
            error = Error(
                kind=ErrorKind.RUNTIME,
                type=ErrorType.RUNTIME_UNAVAILABLE,
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
                "run_plugin.run.failed", machines=available_machines, error=error, span="current"
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
                "run_plugin.run.retry",
                machines=available_machines,
                interval=op.retry.get_wait_interval,
                retry=op.retry,
                span="current",
            )
