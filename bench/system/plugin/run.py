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
    Computer,
    ComputerType,
    Error,
    ErrorKind,
    ErrorType,
    NodeMode,
    NodeType,
    ResourceStatus,
    Run,
    RunStatus,
    Session,
    Text,
    bittuple,
)
from bench.language.core.object import GraphScope
from bench.proto import RunRequest, RuntimeClient
from bench.system.host import Commit, HostPlugin
from bench.utils.tenacity import RETRY_GRPC, RetryOptions, RetryState

if TYPE_CHECKING:
    from bench.system.host import HostService

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass(slots=True)
class RunHandle:
    run: Run
    retry: RetryState
    retry_at: datetime | None = None
    is_cancelled: bool = False


class RunPlugin(HostPlugin[Run]):
    """
    Process Runs in appropriate Runtimes.
    NOTE :Architecture: turn push-based Run->Runtime into pull from Runtime? :PullRuns
     (would need a way to take exclusive ownership of a Run.. this feels related to
      general taking exclusive ownership of Resources? :ExclusiveOwnership)
    """

    watch_types = bittuple(NodeType.RUN)

    def __init__(self, host: "HostService", bench: Bench, retry: RetryOptions = RETRY_GRPC):
        super().__init__(host, bench)
        self._retry = retry
        self._run_queue: asyncio.Queue[RunHandle] = asyncio.Queue()

    def __str__(self):
        return f"queue={self._run_queue.qsize()}"

    @override
    async def start(self) -> None:
        # TODO :Robustness: kill/re-queue abandoned Runs :SystemRecovery
        #  (like Runs 'stuck' on dead or since restarted Computers)
        self.tasks.start_queue(self._run_queue, self._process_run, skip_errors=True)

    def _queue_run(self, run: Run) -> RunHandle:
        """Queues a Run."""
        # queue new operation
        pending_op = RunHandle(run=run, retry=self._retry.new(self.host.oracle))
        self._run_queue.put_nowait(pending_op)
        logger.trace("run_plugin.queue", host=self, run=run)
        return pending_op

    @override
    async def post_commit(self, session: Session, commit: Commit[Run]) -> None:
        for run in commit.added:
            # start new scheduled runs
            if run.status <= RunStatus.SCHEDULED and run.parent_type != NodeType.RUN:
                self._queue_run(run)
        for run in commit.updated:
            # resume active runs (at root)
            if run.status.is_interrupted and run.should_resume:
                self._queue_run(run.root or run)

    @tracer.start_as_current_span("run_plugin.process_run")
    async def _process_run(self, op: RunHandle) -> None:
        """Push Runs to relevant Computers."""
        op.retry.on_attempt()
        run = op.run
        log = logger.bind(host=self, run=run, retry=op.retry)

        # select computers to process run on
        assert NodeType.COMPUTER in self.bench._graph.node_types, f"not loaded in {self.bench!r}"
        available_computers = [
            computer
            for computer in self.bench._graph.nodes_of_type(Computer)
            if computer.type == ComputerType.RUNTIME
            and computer.status == ResourceStatus.AVAILABLE
            and computer.mode < NodeMode.TEMPLATE
        ]
        # if last computer is still available, use that
        # (should decide based on thread ideally :RunRouting)
        existing_computer = first((m for m in available_computers if m.id == run.computer_id), None)
        if existing_computer:
            candidate_computers = [existing_computer]
        else:
            # otherwise, pick any available computer
            candidate_computers = available_computers
            random.shuffle(candidate_computers)

        # contact computers
        for computer in candidate_computers:
            try:
                assert computer.grpc_url, f"missing GRPC URL for {computer!r}"
                runtime = RuntimeClient(
                    await self.network.get_channel(computer.grpc_url, source_id=self.host.id)
                )
                assert run.thread_ptr, f"missing thread for run {run!r}"
                # should be batched and routed per Thread :RunRouting
                request = RunRequest(
                    scope=GraphScope(bench_id=self.bench.id)._to_data(),
                    computer_ptr=computer._to_ref_data(),
                    thread_ptr=run.thread_ptr._to_data(),
                    run_ptrs=[run._to_ref_data()],
                )
                _ = await runtime.run(request)
                log.info("run_plugin.run", computer=computer, span="current")
                return  # success
            except Exception as e:
                log.error("run_plugin.run.error", computer=computer, error=e)
                op.retry.on_error(e)
                continue

        # failed to process run
        if not op.retry.should_retry:
            # give up
            error = Error(
                kind=ErrorKind.RUNTIME,
                type=ErrorType.RUNTIME_UNAVAILABLE,
                title="Failed to queue run",
                text=Text.from_markdown("Could not reach any available Computer."),
            )
            async with self.host.session(commit=True):  # :StaleNodes
                run.status = RunStatus.ABORTED if run.started_at else RunStatus.CANCELLED
                run.terminated_at = self.host.oracle.utc()
                if run.started_at is not None:
                    run.duration = run.terminated_at - run.started_at
                run.error = error
            log.error(
                "run_plugin.run.failed", computers=available_computers, error=error, span="current"
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
                computers=available_computers,
                interval=op.retry.get_wait_interval,
                retry=op.retry,
                span="current",
            )
