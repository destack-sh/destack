import asyncio
from collections import defaultdict
from contextvars import ContextVar
from dataclasses import dataclass
from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Any, Callable, Sequence, cast
from uuid import UUID

import structlog
from opentelemetry import baggage, context, trace

from bench.language import (
    COMMUNICATION_NODE_TYPES,
    DEFAULT_CHECK_OPTIONS,
    DEFAULT_RESOURCE_TIMEOUT,
    DEFAULT_WAIT_TIMEOUT,
    PROCESS_STATUS_BY_INTERRUPTION_TYPE,
    RESOURCE_NODE_TYPES,
    RUNTIME_NODE_TYPES,
    Agent,
    BenchError,
    CheckOptions,
    Claim,
    ClaimStatus,
    CursorType,
    CustomObject,
    Error,
    ErrorKind,
    GetConnection,
    GraphCapture,
    Interruption,
    InterruptionType,
    IsRuntime,
    Message,
    Node,
    NodeGraph,
    NodeMode,
    NodeReference,
    NodeType,
    Package,
    ProcessStatus,
    Run,
    Session,
    SessionStatus,
    Span,
    SpanType,
    Thread,
    ValidationError,
    WatchGetUpdate,
    WatchSearchUpdate,
    capture_span,
    check_value,
    on_invalid_raise,
    synchronize_nodes,
)
from bench.proto.network import Network
from bench.utils.func import group_by
from bench.utils.oracle import Oracle
from bench.utils.task import TaskManager

from .error import NonRetryableError, RetryableError
from .runner import (
    Interrupted,
    Runner,
    RunnerAbortedEvent,
    RunnerCancelledEvent,
    RunnerCompletedEvent,
    RunnerFailedEvent,
    RunnerInterruptedEvent,
    create_run,
    restore_runner,
)
from .thread import ThreadHandle

if TYPE_CHECKING:
    from bench.runtime.process import RuntimeProcess


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


RUN_QUERY = Run.include_descendants(
    NodeType.RUN,
    NodeType.SPAN,
    NodeType.PLAN,
    NodeType.TASK,
    NodeType.INTERRUPTION,
)


THREAD_QUERY = Thread.include_descendants(
    *RESOURCE_NODE_TYPES,
    NodeType.MEMBERSHIP,
    NodeType.PLAN,
    NodeType.TASK,
    NodeType.CLAIM,
    NodeType.AGENT,
    NodeType.CURSOR,
    NodeType.RUN,
    NodeType.INTERRUPTION,
)


@dataclass(slots=True)
class ThreadUpdateTick:
    thread: Thread
    nodes: list[Node]


@dataclass(slots=True)
class ThreadWakeTick:
    thread: Thread


ThreadTick = ThreadUpdateTick | ThreadWakeTick


class Runtime:
    """
    The runtime for executing Runs. A Runtime is tied exclusively to one Session.
    A Run is exclusively owned by one Runtime at a time (but may be transferred between Runtimes).
    NOTE :Architecture! …Security: at some point, the bench language & Runtime will be a custom language
     If just for performance, it makes a lot of sense to implement Bench like Godot with GDScript.
      i.e. we could have Bench be a subset of Python with a Rust-based runtime to actually execute everything.
     Could still support Python in some places, but the core stuff - the orchestration - must blaze.
     Would also make everything much more secure since we could control memory access very precisely.
    """

    def __init__(
        self,
        *,
        session: Session,
        network: Network,
        oracle: Oracle,
        process: "RuntimeProcess | None" = None,
        on_error: Callable[[BaseException], None] | None = None,
    ):
        assert session.bench is not None, f"{session!r} is not attached"
        self.session = cast(Session, session)  # NOTE: break Runtime/Session typechecking circle
        self.session._graph.add_types(*RUNTIME_NODE_TYPES)
        self.session._graph.add_types(*COMMUNICATION_NODE_TYPES)
        self.session_ptr = session.to_ref()
        self.session_id = session.id
        self.supergraph = session._supergraph
        self.network = network
        self.bench = session.bench
        self.oracle = oracle
        self.process = process
        self.on_error = on_error
        assert session._runtime is None, f"{session!r} already in runtime {session._runtime!r}"
        self.session._runtime = self
        self.tasks = TaskManager(
            owner=self, logger=logger, oracle=self.oracle, on_error=self.on_error
        )

        self._locks_by_run_id: dict[UUID, asyncio.Lock] = {}
        self._locks_by_thread_id: dict[UUID, asyncio.Lock] = {}
        self._threads_by_id: dict[UUID, ThreadHandle] = {}
        self._runners_by_id: dict[UUID, Runner] = {}
        self._runners_by_runnable_id: dict[UUID, list[Runner]] = {}
        self._active_runner: ContextVar[Runner | None] = ContextVar("active_runner")
        self._active_runners_by_id: dict[UUID, Runner] = {}
        self._active_root_runners: list[Runner] = []
        self._thread_tick_queue: asyncio.Queue[ThreadTick] = asyncio.Queue()
        self._is_draining: bool = False

    def __str__(self):
        return f"{len(self._active_runners_by_id)} active, {len(self._runners_by_id)} loaded, {self.session!r}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def is_idle(self) -> bool:
        """Check if the Runtime is idle (no pending requests or processing)."""
        return len(self._active_runners_by_id) == 0

    @property
    def active_runner(self) -> Runner | None:
        return self._active_runner.get(None)

    @property
    def active_run(self) -> Run | None:
        runner = self._active_runner.get(None)
        while runner is not None:
            if runner.tracked_run is not None:
                return runner.tracked_run
            runner = runner.parent
        return None

    @property
    def active_mode(self) -> NodeMode:
        runner = self._active_runner.get(None)
        return runner.mode if runner else self.session.mode

    @tracer.start_as_current_span("runtime.start")
    async def start(self):
        """Start the Runtime."""
        # open session
        assert self.session.status == SessionStatus.PENDING, f"{self.session!r} is not pending"
        self.session.status = SessionStatus.OPEN
        self.session.opened_at = self.oracle.utc()
        self.session._create(self.session)
        await self.session.commit()

        # start tasks
        self.tasks.start_queue(self._thread_tick_queue, self._tick_thread, skip_errors=True)

    def stop(self):
        """Stop the Runtime (drain active Runs)."""
        # NOTE :Robustness: handle Runtime stop better? pause, transfer existing Runs?
        #  (maybe auto-interrupt all active Runners so we can transfer them? :HibernateRuns)
        self.tasks.close()
        self._is_draining = True

    @tracer.start_as_current_span("runtime.wait_stopped")
    async def wait_stopped(self):
        """Wait for the Runtime to stop (all active Runs to stop + commit)."""
        await self.tasks.wait_closed()
        active_runners = tuple(runner for runner in self._active_root_runners if runner.task)
        if active_runners:
            logger.debug("runtime.wait_stopped.runners", runners=active_runners)
            await asyncio.gather(
                *(runner.task for runner in active_runners if runner.task), return_exceptions=True
            )
        self.session.closed_at = self.oracle.utc()
        self.session.status = SessionStatus.CLOSED
        await self.session.commit(_ignore_open=True)
        await self.session.close()

    @tracer.start_as_current_span("runtime.run.attempt")
    async def _attempt_run(self, runner: Runner, span: Span, attempt: int):
        """
        Perform a single attempt of a Runner in a given Span.
        If the Runner is is just the Span, this is the only attempt.
        """
        trace_span = trace.get_current_span()
        trace_span.set_attribute("runner", repr(runner))
        trace_span.set_attribute("attempt", attempt)

        self._set_session_context(span)
        log = logger.bind(runner=runner, span=span, attempt=attempt)
        started_at: datetime | None = None
        terminated_at: datetime | None = None
        try:
            if runner.is_stopped:
                raise asyncio.CancelledError()
            with tracer.start_as_current_span("runtime.attempt.run"):
                if span.started_at is None:
                    started_at = self.oracle.utc()
                    span.started_at = started_at
                runner.task = asyncio.create_task(runner.run())
                await runner.task
                terminated_at = self.oracle.utc()

            # check outputs
            if runner.output_type is not None:
                with tracer.start_as_current_span("runtime.check_outputs"):
                    if runner.outputs is None:
                        runner.outputs = CustomObject.new(
                            {},
                            runner.output_type,
                            supergraph=self.session._supergraph,
                        )
                    check_value(
                        runner.outputs,
                        runner.output_type,
                        # raise on detached nodes
                        options=CheckOptions(detached_is="invalid"),
                        invalid=on_invalid_raise,
                    )
            span.status = ProcessStatus.COMPLETED
            log.trace("runtime.attempt.completed", attempt=span, span="current")
        except asyncio.CancelledError as e:
            # cancelled
            span.status = ProcessStatus.ABORTED
            error = Error.from_exception(ErrorKind.RUNTIME, e)
            span.error = error
            log.debug("runtime.attempt.aborted", attempt=span, span="current")
            raise
        except Interrupted as e:
            # interrupted
            status = PROCESS_STATUS_BY_INTERRUPTION_TYPE[e.interruption.type]
            span.status = status
            span.interrupted_at = self.oracle.utc()
            span.interruption = e.interruption
            log.debug("runtime.attempt.interrupted", attempt=span, span="current")
            raise
        except BaseException as e:
            # error
            span.status = ProcessStatus.FAILED
            error = Error.from_exception(ErrorKind.RUNTIME, e)
            span.error = error
            log.debug("runtime.attempt.failed", attempt=span, exc_info=e, span="current")
            raise
        finally:
            runner.task = None
            if span.status.is_terminal:
                if terminated_at is None:
                    terminated_at = self.oracle.utc()
                span.terminated_at = terminated_at
                if started_at is not None:
                    span.duration = terminated_at - started_at

    @tracer.start_as_current_span("runtime.prepare")
    async def _prepare_run(self, runner: Runner):
        """
        Prepare the Run for execution (only for Runs, not Spans).
        A Run is prepared before the first time it's attempted, so only once.
        """
        run = runner.tracked_run
        assert run is not None, f"missing tracked run for {runner!r}"

        # prepare resources
        open_claims: list[Claim] = []
        for claim in runner.node.claims:
            # create new claim
            claim = claim.instance(recursive=False, detach=True)
            claim.parent = runner.thread.thread
            self.session._create(claim)
            open_claims.append(claim)
        if open_claims:
            with capture_span(
                tracer, "runtime.acquire_resources", SpanType.ACQUIRE, runner=runner
            ) as span:
                if span:
                    span.nodes = cast(list["Node"], open_claims)
                await self.wait_for(
                    nodes=open_claims,
                    condition=lambda: all(
                        claim.status == ClaimStatus.OPEN for claim in open_claims
                    ),
                    timeout=DEFAULT_RESOURCE_TIMEOUT,
                )

        # check inputs
        if runner.input_type is not None:
            with tracer.start_as_current_span("runtime.check_inputs"):
                inputs = runner.inputs or CustomObject.new(
                    {}, runner.input_type, supergraph=self.session._supergraph
                )
                try:
                    check_value(
                        inputs,
                        runner.input_type,
                        options=DEFAULT_CHECK_OPTIONS,
                        invalid=on_invalid_raise,
                    )
                except ValidationError as e:
                    runner.status = ProcessStatus.FAILED
                    runner.error = Error.from_exception(ErrorKind.RUNTIME, e)
                    raise

    @tracer.start_as_current_span("runtime.run.run")
    async def _run_run(self, runner: Runner):
        """Runs a Runner, retrying automatically for Runs if needed."""
        # Runner = Run, retry with attempts & breakpoints
        runner.status = ProcessStatus.RUNNING
        # recover run
        run = runner.tracked_run
        assert run is not None, f"missing tracked run for {runner!r}"
        retry = runner.node.to_retry().new(self.oracle)
        attempts = runner.attempts
        retry.attempt = len(attempts)
        last_attempt = attempts[-1] if attempts else None
        for attempt in attempts:  # 'restore' errors
            if attempt.error is not None:
                retry.on_error(attempt.error)
        if last_attempt is not None:
            if last_attempt.status.is_interrupted:
                # resume interrupted attempt
                retry.attempt -= 1  # don't count interrupted attempt (see above)
        # breakpoint before
        runner._trap_pause()
        # core loop
        active_runner_token = self._active_runner.set(runner)
        try:
            # make new attempts if we can/should
            while retry.should_retry and not (
                last_attempt is not None
                and (
                    last_attempt.status == ProcessStatus.COMPLETED or not last_attempt.is_retryable
                )
            ):
                retry.on_attempt()
                if last_attempt is not None and last_attempt.status.is_interrupted:
                    # resume interrupted attempt
                    current_attempt = last_attempt
                else:
                    # create new attempt
                    current_attempt = Span(
                        parent=run,
                        type=SpanType.ATTEMPT,
                        status=ProcessStatus.RUNNING,
                        _skip_validate_self=True,
                    )
                    self.session._create(current_attempt)
                last_attempt = current_attempt
                try:
                    await self._attempt_run(
                        runner=runner, span=current_attempt, attempt=retry.attempt
                    )
                except (Interrupted, asyncio.CancelledError):
                    raise
                except BaseException:
                    if (error := last_attempt.error) and not retry.on_error(error):
                        raise
            # give up if retry exhausted
            if last_attempt is not None and last_attempt.status != ProcessStatus.COMPLETED:
                if last_attempt.error:
                    # re-raise last error
                    raise RetryableError(title=last_attempt.error.title, error=last_attempt.error)
                else:
                    # no attempts, shouldn't actually get here if RetryOptions.max_attempts > 0
                    raise NonRetryableError(title="retry exhausted")
            logger.debug("runtime.run_run.completed", runtime=self, runner=runner, span="current")
        finally:
            # reset active run
            self._active_runner.reset(active_runner_token)
            # update from last attempt
            assert last_attempt is not None, f"missing last attempt for run {runner!r}"
            # runner status = last attempt status
            runner.status = last_attempt.status
            runner.error = last_attempt.error

    @tracer.start_as_current_span("runtime.run.span")
    async def _run_span(self, runner: Runner):
        """Runs a Span Runner."""
        # Runner = Span, attempt only once (no breakpoints)
        runner.status = ProcessStatus.RUNNING
        span = runner.tracked_span
        assert span is not None, f"missing tracked span for {runner!r}"
        active_runner_token = self._active_runner.set(runner)
        try:
            await self._attempt_run(runner=runner, span=span, attempt=0)
        finally:
            # reset active run
            self._active_runner.reset(active_runner_token)
            # update runner from span
            runner.status = span.status
            runner.error = span.error

    @tracer.start_as_current_span("runtime.run")
    async def run_runner(self, runner: Runner[Any]):
        """Run a Runner, retrying automatically and updating the tracked Run along the way."""
        if runner.status.is_terminal:
            return  # already terminated
        if runner.is_root:
            self._active_root_runners.append(runner)
        self._active_runners_by_id[runner.id] = runner

        span = runner.tracked
        self._set_session_context(span)
        context.attach(baggage.set_baggage("run_id", str(span.id)))

        # mark started
        if span.started_at is None:
            span.started_at = self.oracle.utc()
        span.status = ProcessStatus.RUNNING
        if runner.is_root:
            assert type(span) is Run, f"unexpected non-Run root: {span!r}"
            thread = runner.thread.thread
            thread.update_status_from(span, *thread.runs)
            self.session.stage(include_runtime=True)
        else:
            self.session.stage()

        # actually attempt Run
        try:
            if runner.tracked_span is not None:
                await self._run_span(runner)
            elif runner.tracked_run is not None:
                if runner.status < ProcessStatus.RUNNING:
                    await self._prepare_run(runner)
                await self._run_run(runner)
            else:
                raise RuntimeError(f"missing tracked for {runner!r}")
        except Interrupted as e:
            if not runner.status.is_interrupted:
                # interruption not handled in attempt loop (probably from a breakpoint)
                last_attempt = runner.current_attempt
                interrupted_at = (
                    last_attempt.interrupted_at if last_attempt is not None else self.oracle.utc()
                )
                runner.status = PROCESS_STATUS_BY_INTERRUPTION_TYPE[e.interruption.type]
                span.interrupted_at = interrupted_at
            else:
                span.interrupted_at = self.oracle.utc()
            span.interruption = e.interruption
            raise
        except BaseException:
            raise
        finally:
            # update span from runner
            if span.error is not runner.error:
                span.error = runner.error
            if span.status != runner.status:
                span.status = runner.status
            if type(span) is Run:
                if span.inputs is not runner.inputs:
                    span.inputs = runner.inputs
                if span.outputs is not runner.outputs:
                    span.outputs = runner.outputs
            if runner.status.is_terminal:
                last_attempt = runner.current_attempt
                if last_attempt is not None:
                    # made an attempt
                    span.terminated_at = last_attempt.terminated_at
                    if last_attempt.duration is not None:
                        span.duration = last_attempt.duration
                elif span.terminated_at is not None:
                    # didn't make an attempt, but we have a terminated_at (?)
                    span.duration = span.terminated_at - span.started_at  # type: ignore
                else:
                    # didn't make an attempt
                    span.terminated_at = self.oracle.utc()
                    span.duration = span.terminated_at - span.started_at  # type: ignore

            # notify
            self._active_runners_by_id.pop(runner.id, None)
            if runner.parent is None:
                self._active_root_runners.remove(runner)
            if runner.status.is_interrupted:
                assert runner.interruption is not None, f"missing interruption for {runner!r}"
                runner.fire_event(RunnerInterruptedEvent(runner, interruption=runner.interruption))
            elif runner.status == ProcessStatus.COMPLETED:
                runner.fire_event(RunnerCompletedEvent(runner, outputs=runner.outputs))
            elif runner.status == ProcessStatus.FAILED:
                assert runner.error is not None, f"missing error for {runner!r}"
                runner.fire_event(RunnerFailedEvent(runner, error=runner.error))
            elif runner.status == ProcessStatus.ABORTED:
                runner.fire_event(RunnerAbortedEvent(runner))
            elif runner.status == ProcessStatus.CANCELLED:
                runner.fire_event(RunnerCancelledEvent(runner))

            # commit intermediate session edits
            if runner.is_root:
                assert type(span) is Run, f"unexpected non-Run root: {span!r}"
                thread = runner.thread.thread
                thread.update_status_from(span, *thread.runs)
                self.session.stage(include_runtime=True)
            else:
                self.session.stage(include_runtime=False)

            # close runner once we're done
            if runner.is_root and runner.status.is_terminal:
                runner.close()

    async def _wrap_run_runner(self, runner: Runner):
        """Run the runner at the top-level, handling any exceptions."""
        try:
            await self.run_runner(runner)
        except (Interrupted, asyncio.CancelledError):
            pass  # already handled, not a top-level error
        except (BenchError, ValueError, TypeError) as e:
            logger.error("runtime.run_runner.error", runner=runner, exc_info=e)
        except BaseException as e:
            logger.error("runtime.run_runner.internal_error", runner=runner, exc_info=e)
            if self.on_error is not None:
                self.on_error(e)

    def run_runner_soon(self, runner: Runner) -> Runner:
        """Schedule a Runner to run asynchronously."""
        runner.outer_task = asyncio.create_task(self._wrap_run_runner(runner))
        return runner

    def get_interrupted_runs(self, graph: NodeGraph, *interruptions: Interruption) -> list[Run]:
        """Gets all Runs that were directly interrupted by the given Interruptions."""
        interrupted_runs: list[Run] = []
        for run in graph.nodes_of_type(Run):
            interruption = run.interruption
            if (
                run.status.is_interrupted
                and interruption is not None
                and interruption in interruptions
            ):
                interrupted_runs.append(run)
        return interrupted_runs

    def resume_run(self, *runs: Run):
        """Resume interrupted Runs. Does *not* mark the Run or close open Interruptions."""
        from bench.runtime.flow import FlowRunner

        if self._is_draining:
            raise RuntimeError(f"{self!r} was stopped")

        runs_by_parent: dict[Package | Thread | Run | Agent | None, list[Run]] = group_by(
            runs, key=lambda run: run.parent
        )
        for parent, child_runs in runs_by_parent.items():
            assert parent is not None, f"missing parent for {child_runs!r}"
            if isinstance(parent, (Package, Thread)):
                # resume root runs
                for root_run in child_runs:
                    runner = self.restore_runner(root_run)
                    logger.debug("runtime.resume_run", run=root_run, runner=runner)
                    self.run_runner_soon(runner)
            elif isinstance(parent, Run):
                # resume child runs within their parent
                parent_runner = self._active_runners_by_id.get(parent.id)
                if isinstance(parent_runner, FlowRunner):
                    parent_runner.run_inner(child_runs)
                    logger.debug(
                        "runtime.resume_run",
                        run=parent,
                        runner=parent_runner,
                        inner_runs=child_runs,
                    )
                else:
                    root_run = parent.root or parent
                    root_runner = self.restore_runner(root_run)
                    logger.debug("runtime.resume_run.inner", run=parent, runner=root_runner)
                    self.run_runner_soon(root_runner)

    def stop_run(self, run: Run):
        """Stop a Run that is currently active in this Runtime (and any inside it)."""
        root_runner = self._runners_by_id.get(run.id)
        if root_runner is None:
            raise RuntimeError(f"no active runner for {run!r} in {self!r}")
        for runner in reversed(list(root_runner.walk())):
            if not runner.status.is_terminal:
                logger.debug("runtime.stop_run", runner=runner)
                runner.stop()

    #
    # Loading
    #

    @tracer.start_as_current_span("runtime.wait_for")
    async def wait_for(
        self,
        nodes: Sequence[Node],
        condition: Callable[[], bool],
        timeout: timedelta | None = None,
    ):
        """
        Wait for the given nodes to reach a certain state.
        NOTE :Architecture: use Triggers/Interruptions instead of 'busy' (async) wait in Runtime?
        """
        if condition():
            return  # already good
        if timeout is None:
            timeout = DEFAULT_WAIT_TIMEOUT

        # ensure nodes are in global graph
        assert not any(node.is_deleted for node in nodes), f"cannot watch deleted: {nodes!r}"
        if any(node._is_new for node in nodes):
            await self.session.commit()

        log = logger.bind(runtime=self, nodes=nodes, condition=condition)
        subs: list[Callable[[], None]] = []
        links = None
        complete_signal = asyncio.Event()

        def _check():
            """Check if the condition is met, stop if so."""
            if condition():
                log.debug("runtime.wait_for.complete")
                complete_signal.set()

        try:
            # subscribe
            stale_nodes = [node for node in nodes if not node._is_live]
            links = await synchronize_nodes(stale_nodes)
            for link in links:
                subs.append(link.on_update(_check))
            for node in nodes:
                subs.append(self.session.on_edit(node, lambda _: _check()))
            log.trace("runtime.wait_for.subscribe", links=links)

            # wait for condition
            log.debug("runtime.wait_for")
            _check()
            await asyncio.wait_for(complete_signal.wait(), timeout=timeout.total_seconds())
            log.debug("runtime.wait_for.complete")
        except asyncio.TimeoutError as e:
            raise TimeoutError(f"timed out waiting for {nodes!r}") from e
        except BaseException as e:
            log.error("runtime.wait_for.error", exc_info=e)
            raise
        finally:
            if links is not None:
                for link in links:
                    link.close()
            for sub in subs:
                sub()

    def _set_session_context(self, node: IsRuntime):
        """Sets the Session context on a runtime Node."""
        if node.session_id != self.session.id:
            node.session_ptr = self.session_ptr
        if node.client_id != self.session.client_id:
            node.client_ptr = self.session.client_ptr
        if node.computer_id != self.session.computer_id:
            node.computer_ptr = self.session.computer_ptr

    def on_external_update(self, update: WatchGetUpdate | WatchSearchUpdate):
        """React to updates on Runtime nodes from outside this Runtime."""
        touched_nodes_by_thread: dict[Thread, list[Node]] = defaultdict(list)
        # added
        for node in update.added.values():
            if isinstance(node, Message) and (thread := node.thread) is not None:
                touched_nodes_by_thread[thread].append(node)
        # updated
        for node in update.updated.values():
            if isinstance(node, Run):
                # handle requested state changes
                if node.should_stop:
                    self.stop_run(node)
                elif node.should_pause:
                    pass  # nothing to do (pause is trapped automatically)
                elif node.should_resume:
                    # close open Interruption, resume affected Runs
                    interruption = node.interruption
                    if (
                        interruption
                        and interruption.type == InterruptionType.PAUSE
                        and not interruption.status.is_closed
                    ):
                        interruption.complete(_trigger_runtime=False)
                    self.resume_run(node, *node.ancestors)
            elif isinstance(node, Interruption):
                if node.status.is_closed:
                    self.resume_run(*self.get_interrupted_runs(self.session._graph, node))
            elif isinstance(node, Thread):
                touched_nodes_by_thread[node].append(node)
            elif isinstance(node, Message) and (thread := node.thread) is not None:
                touched_nodes_by_thread[thread].append(node)
        # removed
        for node in update.removed.values():
            if isinstance(node, Thread):
                touched_nodes_by_thread.pop(node, None)
            elif isinstance(node, Message) and (thread := node.thread) is not None:
                touched_nodes_by_thread.pop(thread, None)
        # tick threads
        for thread, nodes in touched_nodes_by_thread.items():
            self._thread_tick_queue.put_nowait(ThreadUpdateTick(thread=thread, nodes=nodes))

    def get_thread(self, thread_id: UUID) -> ThreadHandle | None:
        """Get a Thread."""
        return self._threads_by_id.get(thread_id)

    @tracer.start_as_current_span("runtime.load_thread")
    async def _load_thread(self, thread_ptr: NodeReference) -> ThreadHandle:
        """Load a Thread."""
        trace.get_current_span().set_attribute("thread_id", str(thread_ptr.id))

        # bail if we already have this thread
        if thread_ptr.id in self._threads_by_id:
            thread = self._threads_by_id[thread_ptr.id]
            logger.trace("runtime.load_thread.skip", thread=thread, span="current")
            return thread

        # synchronize
        if thread_ptr.id in self._locks_by_thread_id:
            thread_lock = self._locks_by_thread_id[thread_ptr.id]
        else:
            thread_lock = asyncio.Lock()
            self._locks_by_thread_id[thread_ptr.id] = thread_lock

        async with thread_lock:
            # bail if we already have this thread
            if thread_ptr.id in self._threads_by_id:
                thread = self._threads_by_id[thread_ptr.id]
                logger.trace("runtime.load_thread.skip", thread=thread, span="current")
                return thread

            # load thread
            capture = GraphCapture()
            async with capture.capture():
                thread, thread_connection = await THREAD_QUERY.get_connection(thread_ptr, live=True)
                thread._graph.add_types(*RUNTIME_NODE_TYPES)
                thread._graph.add_types(*COMMUNICATION_NODE_TYPES)

                _, messages_connection = (
                    await Message.where(Message.get_property("thread").eq(thread_ptr))
                    .order_by(Message.get_property("created_at").asc())
                    .search_connection(live=True)
                )
                messages_connection.on_update(lambda _, update: self.on_external_update(update))

            handle = ThreadHandle(
                runtime=self,
                capture=capture,
                thread_ptr=thread_ptr,
                thread_connection=thread_connection,
                messages_connection=messages_connection,
            )
            self._threads_by_id[thread_ptr.id] = handle
            logger.debug("runtime.load_thread", thread=handle, span="current")
            return handle

    async def _tick_thread(self, tick: ThreadTick):
        """Tick a Thread on some update, waking Agents as needed."""
        from bench.runtime import AgentRunner

        thread = tick.thread
        handle = self._threads_by_id.get(thread.id)
        assert handle is not None, f"missing thread handle for {thread!r} in {self!r}"

        # ensure all Agent runs are active if they should be
        new_runs: list[Run] = []
        woke_agents: list[Agent] = []
        for membership in thread.memberships:
            if not isinstance(agent := membership.member, Agent):
                continue
            cursor = thread.get_cursor(type=CursorType.THREAD, owned_by=agent)
            if handle.has_new_messages_for(agent, cursor):
                # try to resume, otherwise create new run
                agent_runners = self._runners_by_runnable_id.get(agent.id, ())
                resumed = False
                for runner in agent_runners:  # (should only be at most one per now)
                    if isinstance(runner, AgentRunner) and not runner.status.is_terminal:
                        resumed = True
                        runner.wake()
                        woke_agents.append(agent)
                if not resumed:
                    # create new agent run
                    run, _ = create_run(
                        node=agent,
                        parent=thread,
                        status=ProcessStatus.QUEUED,
                        thread=thread,
                        agent=agent,
                    )
                    new_runs.append(run)
        logger.trace(
            "runtime.tick_thread", tick=tick, thread=thread, runs=new_runs, woke_agents=woke_agents
        )

        # kick off new runs
        if new_runs:
            await self.session.commit()
            for run in new_runs:
                runner, run = await self._load_runner(run.to_ref())
                self.run_runner_soon(runner)

    def get_runner(self, span: Run | Span) -> Runner | None:
        """Get a Runner for a Run or Span."""
        return self._runners_by_id.get(span.id)

    def restore_runner(self, run: Run) -> Runner:
        """Restore a Runner from a Run."""
        if (runner := self._runners_by_id.get(run.id)) is not None:
            return runner
        else:
            runner = restore_runner(runtime=self, run=run)
            return runner

    @tracer.start_as_current_span("runtime.load_runner")
    async def _load_runner(self, run_ptr: NodeReference) -> tuple[Runner, Run]:
        """
        Load a top-level Run.
        TODO :Performance: having to commit before loading Run/Runner is inefficient
         (but need to synchronize the Run somehow?)
        """
        # NOTE :Architecture: split Runner into Runner & RunHandle/RunLoader (like ThreadHandle)?
        trace.get_current_span().set_attribute("run_id", str(run_ptr.id))

        # synchronize
        if run_ptr.id in self._locks_by_run_id:
            run_lock = self._locks_by_run_id[run_ptr.id]
        else:
            run_lock = asyncio.Lock()
            self._locks_by_run_id[run_ptr.id] = run_lock

        async with run_lock:
            # already loaded
            if run_ptr.id in self._runners_by_id:
                runner = self._runners_by_id[run_ptr.id]
                assert runner.tracked_run is not None, f"missing tracked run for {runner!r}"
                logger.trace("runtime.load_runner.skip", runner=runner, run=runner.tracked_run)
                return runner, runner.tracked_run

            # load run
            capture = GraphCapture()
            async with capture.capture():
                run = await RUN_QUERY.get(run_ptr, live=True)
                run._graph.add_types(*RUNTIME_NODE_TYPES)
                run._graph.add_types(*COMMUNICATION_NODE_TYPES)
                assert run.root_ptr is None, f"{run!r} is not a root Run"
                assert isinstance(run._connection, GetConnection), f"{run!r} has no connection"

            # make runner
            runner = restore_runner(runtime=self, run=run)
            runner.connection = run._connection
            runner.capture = capture
            self._runners_by_id[run_ptr.id] = runner
            run._connection.on_update(lambda _, update: self.on_external_update(update))
            logger.debug("runtime.load_runner", runner=runner, run=run, span="current")
            return runner, run

    #
    # Service
    #

    async def run(
        self,
        run: Run | NodeReference,
        *,
        _return_error: bool = True,
    ) -> Runner | None:
        """Run a top-level Run in this Runtime."""

        if isinstance(run, Run):
            run = run.to_ref()

        # bail if we already have this runner as active
        if run.id in self._active_runners_by_id:
            return self._active_runners_by_id[run.id]

        # can't run if stopped
        if self._is_draining:
            # NOTE :Robustness: shouldn't Runtime error if draining only for new *root* Runs?
            raise RuntimeError(f"{self!r} is draining")

        async with self.session.active():
            # get runner
            runner, run = await self._load_runner(run)
            assert runner.tracked.is_attached, f"{runner!r}'s {runner.tracked!r} is not attached"
            assert runner.is_root, f"{runner!r} is not a root runner"

            # get thread
            thread_ptr = run.thread_ptr
            assert thread_ptr is not None, f"{run!r} has no Thread"
            thread = await self._load_thread(thread_ptr)

            # actually run
            try:
                assert runner.capture is not None, f"{runner!r} has no graph capture"
                async with runner.capture.capture():
                    await self.run_runner(runner)
                logger.info("runtime.run", thread=thread, run=run, runner=runner, span="current")
            except (Interrupted, asyncio.CancelledError):
                pass  # already handled, not a top-level error
            except (BenchError, ValueError, TypeError) as e:
                # re-raised inner user error
                logger.info("runtime.run.error", thread=thread, run=run, exc_info=e, span="current")
                if not _return_error:
                    raise
            except BaseException as e:
                # some unexpected internal error
                logger.error(
                    "runtime.run.internal_error", thread=thread, run=run, exc_info=e, span="current"
                )
                if self.on_error is not None:
                    self.on_error(e)
                if not _return_error:
                    raise

        return runner

    async def wake(self, thread_ptr: NodeReference) -> None:
        """Wake a Thread (if inactive)."""
        if thread_ptr.id in self._threads_by_id:
            return  # already alive (we don't need to wake it)
        async with self.session.active():
            thread = await self._load_thread(thread_ptr)
            self._thread_tick_queue.put_nowait(ThreadWakeTick(thread=thread.thread))
