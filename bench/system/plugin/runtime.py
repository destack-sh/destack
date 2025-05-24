import abc
import asyncio
import dataclasses
from dataclasses import dataclass
from datetime import datetime, timedelta
from typing import TYPE_CHECKING, override

import structlog
from opentelemetry import trace

from bench.language import (
    Bench,
    Error,
    ErrorType,
    Machine,
    MachineType,
    Message,
    Node,
    NodeType,
    ProcessStatus,
    ResourceStatus,
    Run,
    Scope,
    Session,
    Thread,
    bittuple,
)
from bench.pb2 import RunRequest, WakeRequest
from bench.proto import RuntimeClient
from bench.system.host import Commit, HostPlugin
from bench.utils.tenacity import RETRY_GRPC, RetryOptions, RetryState

if TYPE_CHECKING:
    from bench.system.host import HostService

# ruff: noqa: RUF009

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


def raise_if_none():
    _field = dataclasses.field()

    def _raise():
        raise ValueError(f"{_field.name} must be set")

    _field.default_factory = _raise
    return _field


@dataclass
class RuntimeOp:
    retry: RetryState
    retry_at: datetime | None = None
    is_cancelled: bool = False


class RuntimePlugin[N: Node, O: RuntimeOp = RuntimeOp](HostPlugin[N], abc.ABC):
    """
    Do stuff in appropriate Runtimes.
    """

    def __init__(self, host: "HostService", bench: Bench, retry: RetryOptions = RETRY_GRPC):
        super().__init__(host, bench)
        self._retry = retry
        self._queue: asyncio.Queue[O] = asyncio.Queue()

    def __str__(self):
        return f"queue={self._queue.qsize()}"

    @override
    async def start(self) -> None:
        # TODO :Cleanup: mark abandoned Runs as dead
        #  (like Runs 'stuck' on dead or since restarted Machines)
        self.tasks.start_queue(self._queue, self._send_op, skip_errors=True)

    @abc.abstractmethod
    async def _send_in_runtime(self, op: O, machine: Machine, runtime: RuntimeClient) -> None:
        """Do stuff in a Runtime."""
        ...

    @abc.abstractmethod
    async def _on_failed(self, op: O, error: Error) -> None:
        """Fail an operation."""
        ...

    @tracer.start_as_current_span("runtime_plugin.send")
    async def _send_op(self, op: O) -> None:
        """Send an operation to relevant Runtimes."""
        op.retry.on_attempt()
        log = logger.bind(host=self, op=op, retry=op.retry)

        # select machines to send run on :RuntimeRouting
        available_machines = await Machine.search(
            where=Machine.property("type").eq(MachineType.RUNTIME)
            & Machine.property("status").eq(ResourceStatus.AVAILABLE)
        ).execute_list()

        # contact machines
        for machine in available_machines:
            try:
                assert machine.grpc_url, f"missing GRPC URL for {machine!r}"
                channel = await self.network.get_channel(machine.grpc_url, source_id=self.host.id)
                runtime = RuntimeClient(channel)
                await self._send_in_runtime(op, machine, runtime)
                log.trace(f"{self.name}.send", machine=machine, channel=channel, span="current")
                return  # success
            except Exception as e:
                log.error(
                    f"{self.name}.send.error",
                    machine=machine,
                    grpc_url=machine.grpc_url,
                    exc_info=e,
                )
                op.retry.on_error(e)
                self.host.on_error(e)
                continue

        # failed to send run
        if not op.retry.should_retry:
            # give up
            error = Error(
                type=ErrorType.RUNTIME_UNAVAILABLE,
                title="Failed to send",
                text="Could not reach any available Machine.",
            )
            await self._on_failed(op, error)
            log.error(
                f"{self.name}.send.failed",
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
                callback=lambda: op.is_cancelled or self._queue.put_nowait(op),
            )
            log.trace(
                "runtime_plugin.process.retry",
                machines=available_machines,
                interval=op.retry.get_wait_interval,
                retry=op.retry,
                span="current",
            )


#
# Run
#


@dataclass
class RunOp(RuntimeOp):
    run: Run = raise_if_none()


class RunPlugin(RuntimePlugin[Run, RunOp]):
    """
    Process Runs in appropriate Runtimes.
    """

    watch_types = bittuple(NodeType.RUN)

    def _queue_run(self, run: Run):
        """Queues a Run."""
        # queue new operation
        pending_op = RunOp(run=run, retry=self._retry.new(self.host.oracle))
        self._queue.put_nowait(pending_op)
        logger.trace("run_plugin.queue", host=self, run=run)

    @override
    async def post_commit(self, session: Session, commit: Commit[Run]) -> None:
        for run in commit.added:
            # start new scheduled runs
            if run.status <= ProcessStatus.SCHEDULED and run.parent_type != NodeType.RUN:
                self._queue_run(run)
        for run in commit.updated:
            # resume active runs (at root)
            if run.status.is_interrupted and run.should_resume:
                self._queue_run(run.root or run)

    @override
    async def _send_in_runtime(self, op: RunOp, machine: Machine, runtime: RuntimeClient) -> None:
        # should be batched and routed per Thread :RuntimeRouting
        assert op.run.thread_ptr, f"missing thread for run {op.run!r}"
        request = RunRequest(
            scope=Scope(bench_id=self.bench.id)._to_data(),
            machine_ptr=machine._to_ref_data(),
            thread_ptr=op.run.thread_ptr._to_data(),
            run_ptrs=[op.run._to_ref_data()],
        )
        await runtime.run(request)

    @override
    async def _on_failed(self, op: RunOp, error: Error) -> None:
        # mark run as failed
        run = op.run
        async with self.host.session(commit=True):  # :StaleNodes
            run.status = ProcessStatus.ABORTED if run.started_at else ProcessStatus.CANCELLED
            run.terminated_at = self.host.oracle.utc()
            if run.started_at is not None:
                run.duration = run.terminated_at - run.started_at
            run.error = error


#
# Wake
#


@dataclass
class WakeOp(RuntimeOp):
    thread: Thread = raise_if_none()


class WakePlugin(RuntimePlugin[Thread | Message, WakeOp]):
    """
    Notify Threads/Agents in appropriate Runtimes.
    """

    watch_types = bittuple(NodeType.THREAD, NodeType.MESSAGE)

    def _queue_wake(self, thread: Thread) -> WakeOp:
        """Queues a Wake."""
        pending_op = WakeOp(thread=thread, retry=self._retry.new(self.host.oracle))
        self._queue.put_nowait(pending_op)
        logger.trace("wake_plugin.queue", host=self, thread=thread)
        return pending_op

    @override
    async def post_commit(self, session: Session, commit: Commit[Thread | Message]) -> None:
        threads: set[Thread] = set()
        for node in commit.added:
            if isinstance(node, Thread):
                threads.add(node)
            elif isinstance(node, Message) and (thread := node.thread) is not None:
                threads.add(thread)
        for node in commit.updated:
            if isinstance(node, Thread):
                threads.add(node)
            elif isinstance(node, Message) and (thread := node.thread) is not None:
                threads.add(thread)
        for thread in threads:
            self._queue_wake(thread)

    @override
    async def _send_in_runtime(self, op: WakeOp, machine: Machine, runtime: RuntimeClient) -> None:
        request = WakeRequest(
            scope=Scope(bench_id=self.bench.id)._to_data(),
            machine_ptr=machine._to_ref_data(),
            thread_ptrs=[op.thread._to_ref_data()],
        )
        await runtime.wake(request)

    @override
    async def _on_failed(self, op: WakeOp, error: Error) -> None:
        pass  # nothing to do?
