import asyncio
from contextvars import ContextVar
from datetime import datetime, timedelta
from typing import Any, Callable, Mapping, Sequence, cast
from uuid import UUID

import structlog
from git import TYPE_CHECKING
from opentelemetry import baggage, context, trace

from bench.language import (
    COMMUNICATION_NODE_TYPES,
    DEFAULT_CHECK_OPTIONS,
    DEFAULT_RESOURCE_TIMEOUT,
    DEFAULT_WAIT_TIMEOUT,
    RUN_STATUS_BY_INTERRUPTION_TYPE,
    RUNTIME_NODE_TYPES,
    Action,
    BenchError,
    BreakpointSite,
    BuiltinObject,
    CheckOptions,
    Claim,
    ClaimStatus,
    ComputedValue,
    ComputedValueKind,
    ComputedValueMode,
    CustomObject,
    Error,
    ErrorKind,
    Field,
    GetConnection,
    Interruption,
    InterruptionType,
    IsRuntime,
    Node,
    NodeGraph,
    NodeMode,
    NodeReference,
    NodeType,
    Package,
    PathElementType,
    PathError,
    PathOptions,
    Run,
    RunStatus,
    RunType,
    Session,
    SessionStatus,
    Span,
    SpanType,
    Thread,
    TypeKind,
    ValidationError,
    WatchGetUpdate,
    capture_span,
    check_value,
    coerce_value,
    evaluate_path,
    on_invalid_raise,
    synchronize_nodes,
)
from bench.language.connection.capture import GraphCapture
from bench.proto.network import Network
from bench.utils.func import group_by
from bench.utils.oracle import Oracle

from .cache import Cache
from .error import InvalidComputedError, NonRetryableError, RetryableError
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


class Runtime:
    """
    The runtime for executing Runs. A Runtime is tied exclusively to one Session.
    A Run is exclusively owned by one Runtime at a time (but may be transferred between Runtimes).
    """

    def __init__(
        self,
        *,
        session: Session,
        network: Network,
        cache: Cache,
        oracle: Oracle,
        process: "RuntimeProcess | None" = None,
        static_glbls: Mapping[str, Any] | None = None,
        dynamic_glbls: Mapping[str, Any] | None = None,
        on_error: Callable[[BaseException], None] | None = None,
    ):
        from bench.runtime.code import DYNAMIC_CODE_GLOBALS, STATIC_CODE_GLOBALS

        assert session.bench is not None, f"{session!r} is not attached"
        self.session = session
        self.session._graph.add_types(*RUNTIME_NODE_TYPES)
        self.session._graph.add_types(*COMMUNICATION_NODE_TYPES)
        self.session_ptr = session.to_ref()
        self.session_id = session.id
        self.network = network
        self.bench = session.bench
        self.cache = cache
        self.oracle = oracle
        self.static_glbls = static_glbls or STATIC_CODE_GLOBALS
        self.dynamic_glbls = dynamic_glbls or DYNAMIC_CODE_GLOBALS
        self.combined_glbls = {**self.static_glbls, **self.dynamic_glbls}
        self.process = process
        self.on_error = on_error
        assert session._runtime is None, f"{session!r} already in runtime {session._runtime!r}"
        self.session._runtime = self

        self._locks_by_run_id: dict[UUID, asyncio.Lock] = {}
        self._locks_by_thread_id: dict[UUID, asyncio.Lock] = {}
        self._threads_by_id: dict[UUID, ThreadHandle] = {}
        self._runners_by_id: dict[UUID, Runner] = {}
        self._runners_by_thread_id: dict[UUID, list[Runner]] = {}
        self._active_runner: ContextVar[Runner | None] = ContextVar("active_runner")
        self._active_runners_by_id: dict[UUID, Runner] = {}
        self._active_root_runners: list[Runner] = []
        self._is_stop_requested: bool = False

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
        assert self.session.status == SessionStatus.PENDING, f"{self.session!r} is not pending"
        self.session.status = SessionStatus.OPEN
        self.session.opened_at = self.oracle.utc()
        self.session._create(self.session)
        await self.session.commit()

    def stop(self):
        """Stop the Runtime (drain active Runs)."""
        # TODO :Robustness: handle Runtime stop better? drain properly ... pause existing Runs?
        #  (maybe auto-interrupt all active Runners so we can transfer them? :HibernateRuns)
        self._is_stop_requested = True

    @tracer.start_as_current_span("runtime.wait_stopped")
    async def wait_stopped(self):
        """Wait for the Runtime to stop (all active Runs to stop + commit)."""
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

    #
    # Context
    #

    def _evaluate_computed_value(self, runner: Runner, computed_value: ComputedValue):
        """Evaluates the ComputedValue in/to the Run/Runner."""
        if computed_value.kind == ComputedValueKind.PATH:
            source_path = computed_value.source_path
            assert source_path is not None, f"no source path for {computed_value!r}"
            return evaluate_path(
                current=runner.node,
                scope=runner.node,
                context=runner.context,
                path=source_path,
                options=PathOptions(missing_is="invalid"),
            )
        else:
            raise RuntimeError(f"unsupported {computed_value!r}")

    def _apply_computed_value(self, runner: Runner, computed_value: ComputedValue) -> bool:
        """Applies the ComputedValue in/to the Run/Runner. Returns True if the value was applied."""
        # get source value
        try:
            source_value = self._evaluate_computed_value(runner, computed_value)
        except (PathError, AttributeError, ValidationError) as e:
            raise InvalidComputedError(computed_value=computed_value) from e
        if source_value is None and computed_value.mode == ComputedValueMode.IF_SOURCE_SET:
            logger.trace(
                "runtime.apply_computed_value.skip_if_source_set", computed_value=computed_value
            )
            return False

        # get target site
        assert computed_value.target_path is not None, f"no target for {computed_value!r}"
        target_obj = evaluate_path(
            current=runner.node,
            scope=runner.node,
            context=runner.context,
            path=computed_value.target_path.elements[:-1],
            options=PathOptions(missing_is="none"),
        )
        target_key = computed_value.target_path.elements[-1]
        if target_obj is None:
            logger.trace(
                "runtime.apply_computed_value.missing", runner=runner, computed_value=computed_value
            )
            return False
        if target_key.type != PathElementType.ATTRIBUTE:
            raise InvalidComputedError(computed_value=computed_value)

        # skip if target is already set (if configured)
        if computed_value.mode == ComputedValueMode.IF_TARGET_UNSET:
            if isinstance(field := target_key.node, Field):
                assert isinstance(
                    target_obj, CustomObject
                ), f"bad target {target_obj!r} in {computed_value!r}"
                is_set = target_obj.is_set(field)
            elif (prop := target_key.property) is not None:
                assert isinstance(
                    target_obj, (BuiltinObject, CustomObject)
                ), f"bad target {target_obj!r} in {computed_value!r}"
                is_set = target_obj.is_set(prop)
            else:
                raise InvalidComputedError(computed_value=computed_value)
            if is_set:
                logger.trace(
                    "runtime.apply_computed_value.skip_if_target_unset",
                    runner=runner,
                    computed_value=computed_value,
                )
                return False

        # coerce & set value
        if isinstance(field := target_key.node, Field):
            assert isinstance(
                target_obj, CustomObject
            ), f"bad target {target_obj!r} in {computed_value!r}"
            mapped_value = coerce_value(source_value, field)
            target_obj._do_set(field, mapped_value, coerce=False)
        elif (prop := target_key.property) is not None:
            assert isinstance(
                target_obj, (BuiltinObject, CustomObject)
            ), f"bad target {target_obj!r} in {computed_value!r}"
            mapped_value = coerce_value(source_value, prop.type_info)
            target_obj._do_set(prop.name, mapped_value)
        else:
            raise InvalidComputedError(computed_value=computed_value)
        logger.debug(
            "runtime.apply_computed_value",
            computed_value=computed_value,
            target=target_obj,
            value=mapped_value,
        )
        return True

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

    def _set_context(self, node: IsRuntime):
        """Sets the current context on a Span."""
        if node.session_id != self.session.id:
            node.session_ptr = self.session_ptr
        if node.client_id != self.session.client_id:
            node.client_ptr = self.session.client_ptr
        if node.computer_id != self.session.computer_id:
            node.computer_ptr = self.session.computer_ptr
        if node.user_id != self.session.user_id:
            node.user_ptr = self.session.user_ptr

    #
    # Running
    #

    @tracer.start_as_current_span("runtime.run.attempt")
    async def _attempt_run(self, runner: Runner, span: Span, attempt: int):
        """
        Perform a single attempt of a Runner in a given Span.
        If the Runner is is just the Span, this is the only attempt.
        """
        trace_span = trace.get_current_span()
        trace_span.set_attribute("runner", repr(runner))
        trace_span.set_attribute("attempt", attempt)

        self._set_context(span)
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
            span.status = RunStatus.COMPLETED
            log.trace("runtime.attempt.completed", attempt=span, span="current")
        except asyncio.CancelledError as e:
            # cancelled
            span.status = RunStatus.ABORTED
            error = Error.from_exception(ErrorKind.RUNTIME, e)
            span.error = error
            log.debug("runtime.attempt.aborted", attempt=span, span="current")
            raise
        except Interrupted as e:
            # interrupted
            status = RUN_STATUS_BY_INTERRUPTION_TYPE[e.interruption.type]
            span.status = status
            span.interrupted_at = self.oracle.utc()
            span.interruption = e.interruption
            log.debug("runtime.attempt.interrupted", attempt=span, span="current")
            raise
        except BaseException as e:
            # error
            span.status = RunStatus.FAILED
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

    @tracer.start_as_current_span("runtime.prefetch_context")
    async def _prefetch_context(self, runner: Runner):
        """
        Load remote Nodes that are (probably) required for the given Runner.
        TODO :Architecture :Incomplete: unclear which remote Nodes to load for Runs and how
         (should we only load top level references? Text mentions? expand Messages into Threads?
           entire Run trees? this seems related to the context/projection stuff in model instruct)
        NOTE :Robustness: isn't there a race condition in checking & loading Nodes across Runners?
         (and what if a Node is loaded only because it's loaded by a different concurrent Run?,
          but then that Run completes, so we unload it again while the other dependent Run runs?)
        """

        # gather inputs, resources & outputs
        nodes_ptr_by_id: dict[UUID, NodeReference] = {}
        for obj in (runner.inputs, runner.outputs):
            if obj is None:
                continue
            # fields
            for field in obj.fields:
                if field.kind != TypeKind.NODE and field.kind != TypeKind.BASED_NODE:
                    continue
                field_value = cast(Any, obj._do_get(field, _raw=True))
                if field_value is not None:
                    if not field.is_list:
                        nodes_ptr_by_id[field_value.id] = field_value
                    else:
                        for node_ptr in field_value:
                            nodes_ptr_by_id[node_ptr.id] = node_ptr
        if not nodes_ptr_by_id:
            return

        # filter missing nodes
        supergraph = self.session._supergraph
        missing_nodes_ptr: list[NodeReference] = []
        for node_ptr in nodes_ptr_by_id.values():
            node = supergraph.get(node_ptr)
            if node is None:
                missing_nodes_ptr.append(node_ptr)
        if not missing_nodes_ptr:
            return

        # load missing nodes
        missing_links = await synchronize_nodes(missing_nodes_ptr)
        logger.debug("runtime.load_run.missing", nodes=missing_nodes_ptr, links=missing_links)

    @tracer.start_as_current_span("runtime.prepare")
    async def _prepare_run(self, runner: Runner):
        """
        Prepare the Run for execution (only for Runs, not Spans).
        A Run is prepared before the first time it's attempted, so only once.
        """
        run = runner.tracked_run
        assert run is not None, f"missing tracked run for {runner!r}"

        # compute resources/inputs/options from context
        if isinstance(runner.node, Action):
            # init inputs from action
            if runner.inputs is not None and runner.node.inputs_packed is not None:
                runner.inputs.set_default(runner.node.inputs, _skip_validate=True)
        # apply computed values
        try:
            for computed_value in runner.node.computed_values:
                if computed_value.target_path is not None and computed_value.is_active:
                    self._apply_computed_value(runner, computed_value)
        except Exception as e:
            runner.status = RunStatus.FAILED
            runner.error = Error.from_exception(ErrorKind.RUNTIME, e)
            raise

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
                    runner.status = RunStatus.FAILED
                    runner.error = Error.from_exception(ErrorKind.RUNTIME, e)
                    raise

    @tracer.start_as_current_span("runtime.run.run")
    async def _run_run(self, runner: Runner):
        """Runs a Runner, retrying automatically for Runs if needed."""
        # Runner = Run, retry with attempts & breakpoints
        runner.status = RunStatus.RUNNING
        # recover run
        run = runner.tracked_run
        assert run is not None, f"missing tracked run for {runner!r}"
        retry = runner.options.to_retry().new(self.oracle)
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
        if last_attempt is None:
            runner._trap_breakpoint(BreakpointSite.RUN_BEFORE)
        # core loop
        active_runner_token = self._active_runner.set(runner)
        try:
            # make new attempts if we can/should
            while retry.should_retry and not (
                last_attempt is not None
                and (last_attempt.status == RunStatus.COMPLETED or not last_attempt.is_retryable)
            ):
                if run.attempt != retry.attempt:
                    run.attempt = retry.attempt
                retry.on_attempt()
                if last_attempt is not None and last_attempt.status.is_interrupted:
                    # resume interrupted attempt
                    current_attempt = last_attempt
                else:
                    # create new attempt
                    current_attempt = Span(
                        parent=run,
                        type=SpanType.ATTEMPT,
                        status=RunStatus.RUNNING,
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
            if last_attempt is not None and last_attempt.status != RunStatus.COMPLETED:
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
            # breakpoint after
            if last_attempt.status.is_terminal:
                if last_attempt.status == RunStatus.FAILED:
                    runner._trap_breakpoint(
                        BreakpointSite.RUN_AFTER, BreakpointSite.RUN_AFTER_FAILED
                    )
                elif last_attempt.status == RunStatus.COMPLETED:
                    runner._trap_breakpoint(
                        BreakpointSite.RUN_AFTER, BreakpointSite.RUN_AFTER_COMPLETED
                    )
                else:
                    runner._trap_breakpoint(BreakpointSite.RUN_AFTER)
            # runner status = last attempt status
            runner.status = last_attempt.status
            runner.error = last_attempt.error

    @tracer.start_as_current_span("runtime.run.span")
    async def _run_span(self, runner: Runner):
        """Runs a Span Runner."""
        # Runner = Span, attempt only once (no breakpoints)
        runner.status = RunStatus.RUNNING
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
        """Runs a Runner, retrying automatically and updating the tracked Run along the way."""
        if runner.status.is_terminal:
            return  # already terminated
        if runner.parent is None:
            self._active_root_runners.append(runner)
        self._active_runners_by_id[runner.id] = runner

        span = runner.tracked
        self._set_context(span)
        context.attach(baggage.set_baggage("run_id", str(span.id)))

        # mark started
        if span.started_at is None:
            span.started_at = self.oracle.utc()
        span.status = RunStatus.RUNNING
        self.session.commit_optimistic()

        # actually attempt Run
        try:
            assert (
                runner.parent is None or not runner.parent.status.is_terminal
            ), f"parent {runner.parent!r} was terminated"
            if runner.tracked_span is not None:
                await self._run_span(runner)
            elif runner.tracked_run is not None:
                await self._prefetch_context(runner)
                if runner.status < RunStatus.RUNNING:
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
                runner.status = RUN_STATUS_BY_INTERRUPTION_TYPE[e.interruption.type]
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
                # update terminal status
                last_attempt = runner.current_attempt
                if last_attempt is not None:
                    # made an attempt
                    span.terminated_at = last_attempt.terminated_at
                    if last_attempt.duration is not None:
                        span.duration = last_attempt.duration
                elif span.terminated_at is not None:
                    # didn't make an attempt, but we have a terminated_at
                    span.duration = span.terminated_at - span.started_at  # type: ignore
                else:
                    # didn't make an attempt
                    span.terminated_at = self.oracle.utc()
                    span.duration = span.terminated_at - span.started_at  # type: ignore

            # commit intermediate session edits
            self.session.commit_optimistic(runtime=runner.is_root)

            # notify
            self._active_runners_by_id.pop(runner.id, None)
            if runner.parent is None:
                self._active_root_runners.remove(runner)
            if runner.status.is_interrupted:
                assert runner.interruption is not None, f"missing interruption for {runner!r}"
                runner.fire_event(RunnerInterruptedEvent(runner, interruption=runner.interruption))
            elif runner.status == RunStatus.COMPLETED:
                runner.fire_event(RunnerCompletedEvent(runner, outputs=runner.outputs))
            elif runner.status == RunStatus.FAILED:
                assert runner.error is not None, f"missing error for {runner!r}"
                runner.fire_event(RunnerFailedEvent(runner, error=runner.error))
            elif runner.status == RunStatus.ABORTED:
                runner.fire_event(RunnerAbortedEvent(runner))
            elif runner.status == RunStatus.CANCELLED:
                runner.fire_event(RunnerCancelledEvent(runner))

    async def _wrap_run_runner(self, runner: Runner):
        """Run the runner at the top-level, handling any exceptions."""
        try:
            await self.run_runner(runner)
        except (Interrupted, asyncio.CancelledError):
            pass  # already handled, not a top-level error
        except Exception as e:
            logger.error("runtime.run_runner.error", runner=runner, exc_info=e)

    def run_runner_soon(self, runner: Runner) -> Runner:
        """Schedule a Runner to run asynchronously."""
        runner.outer_task = asyncio.create_task(self._wrap_run_runner(runner))
        return runner

    def _try_mark_failed(self, run: Run, kind: ErrorKind, error: BaseException):
        """Mark a Run as failed (but ignore if we can't, perhaps because the Session is closing)."""
        try:
            if run.status != RunStatus.FAILED:
                run.status = RunStatus.FAILED
                run.error = Error.from_exception(kind, error)
        except BaseException as e:
            logger.error("runtime.mark_failed.error", run=run, exc_info=e)

    #
    # Orchestration
    #

    def get_thread(self, thread_id: UUID) -> ThreadHandle | None:
        """Get a Thread."""
        return self._threads_by_id.get(thread_id)

    @tracer.start_as_current_span("runtime.load_thread")
    async def _load_thread(self, thread_ptr: NodeReference) -> ThreadHandle:
        """Load a Thread."""
        trace.get_current_span().set_attribute("thread_id", str(thread_ptr.id))

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
                logger.debug("runtime.load_thread.skip", thread=thread, span="current")
                return thread

            # load thread
            handle = ThreadHandle(runtime=self, thread_ptr=thread_ptr)
            self._threads_by_id[thread_ptr.id] = handle
            await handle.open()
            logger.debug("runtime.load_thread", thread=handle, span="current")
            return handle

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
        """Load the top-level Run."""
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
                logger.debug("runtime.load_runner.skip", runner=runner, run=runner.tracked_run)
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
            thread_id = run.thread_id
            assert thread_id is not None, f"{run!r} has no Thread"
            if thread_id not in self._runners_by_thread_id:
                self._runners_by_thread_id[thread_id] = []
            self._runners_by_thread_id[thread_id].append(runner)
            run._connection.on_update(lambda _, update: self._on_updated(runner, update))
            logger.debug("runtime.load_runner", runner=runner, run=run, span="current")
            return runner, run

    def _on_run_updated(self, runner: Runner, run: Run):
        """React to updates on a Run from outside this Runtime."""
        if run.status.is_terminal:
            return  # nothing to do anymore
        elif run.stopped_at:
            self.stop_run(run)
        elif run.should_pause:
            pass  # nothing to do (pause is trapped automatically)
        elif run.should_resume:
            # close open Interruption, resume affected Runs
            interruption = run.interruption
            if (
                interruption
                and interruption.type == InterruptionType.PAUSE
                and not interruption.status.is_closed
            ):
                interruption.complete(_trigger_runtime=False)
            self.resume_run(run, *run.ancestors)

    def _on_interrupt_updated(self, runner: Runner, interruption: Interruption):
        """React to updates on an Interruption from outside this Runtime."""
        if interruption.status.is_closed:
            self.resume_run(*self.get_interrupted_runs(self.session._graph, interruption))

    def _on_updated(self, runner: Runner, update: WatchGetUpdate):
        """React to updates on Runtime nodes from outside this Runtime."""
        for node in update.updated.values():
            if isinstance(node, Run):
                self._on_run_updated(runner, node)
            elif isinstance(node, Interruption):
                self._on_interrupt_updated(runner, node)

    async def run(
        self,
        run: Run | NodeReference,
        *,
        _return_error: bool = True,
    ) -> Runner | None:
        """
        Run a top-level Run in this Runtime. This Runtime will assume ownership of the Run.
        Automatically lifts/joins the Run with a higher or existing Run if needed.
        """
        from bench.runtime.flow import FlowRunner

        if isinstance(run, Run):
            run = run.to_ref()

        # can't run if stopped
        if self._is_stop_requested:
            raise RuntimeError(f"{self!r} was stopped")

        # bail if we already have this runner as active
        if run.id in self._active_runners_by_id:
            return self._active_runners_by_id[run.id]

        async with self.session.active():
            # get runner
            runner, run = await self._load_runner(run)
            assert runner.tracked.is_attached, f"{runner!r}'s {runner.tracked!r} is not attached"
            assert runner.is_root, f"{runner!r} is not a root runner"
            inner_runner = runner

            # get thread
            thread_ptr = run.thread_ptr
            assert thread_ptr is not None, f"{run!r} has no Thread"
            thread = await self._load_thread(thread_ptr)

            # lift run into existing / higher flow
            if (
                run.type == RunType.ACTION
                and (action := run.action) is not None
                and (flow := action.flow) is not None
            ):
                # find existing Flow to lift into
                identity_id = run.agent_id
                self.session.commit_optimistic()
                for existing_runner in self._runners_by_id.values():
                    if (
                        (existing_run := existing_runner.tracked_run) is not None
                        and existing_run.type == RunType.FLOW
                        and existing_run.flow_id == flow.id
                        and existing_run.agent_id == identity_id
                    ):
                        target_runner = existing_runner
                        assert isinstance(
                            target_runner, FlowRunner
                        ), f"{target_runner!r} is not a FlowRunner"
                        break
                else:
                    target_runner = None

                if target_runner is not None:
                    # lift into existing run
                    outer_run = target_runner.tracked_run
                    assert outer_run is not None, f"{target_runner!r} has no tracked run"
                    run.move(to=outer_run)
                    self.session.commit_optimistic(runtime=True)
                    runner.close()  # we're moving the runner to an existing runner

                    # bail if target runner is already active
                    if target_runner.id in self._active_runners_by_id:
                        target_runner.run_inner((run,))
                        runner = self._active_runners_by_id[runner.id]
                        return runner
                    runner = target_runner
                else:
                    # create new outer run
                    parent_node = run.parent or run.thread
                    assert parent_node is not None, f"no parent for {run!r}"
                    outer_run, _ = create_run(
                        flow,
                        parent=parent_node,
                        mode=run.mode,
                        status=RunStatus.QUEUED,
                        thread=run.thread,
                        graph=parent_node._graph,
                    )
                    run.move(to=outer_run)
                    self.session.commit_optimistic(runtime=True)
                    runner.close()  # we're loading a new runner to cover the outer run
                    await self.session.commit()  # wait for Run to actually exist
                    runner, run = await self._load_runner(outer_run.to_ref())
                    logger.debug(
                        "runtime.lift_flow", runner=runner, inner_run=run, outer_run=outer_run
                    )

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
                self._try_mark_failed(run, ErrorKind.RUNTIME, e)
                self.session.commit_optimistic()
                logger.info("runtime.run.error", thread=thread, run=run, exc_info=e, span="current")
                if not _return_error:
                    raise
            except BaseException as e:
                # some unexpected internal error
                self._try_mark_failed(run, ErrorKind.INTERNAL, e)
                self.session.commit_optimistic()
                logger.error(
                    "runtime.run.internal_error", thread=thread, run=run, exc_info=e, span="current"
                )
                if self.on_error is not None:
                    self.on_error(e)
                if not _return_error:
                    raise

        # get inner runner from actual runner (may have been lifted)
        if inner_runner is not runner:
            inner_runner = runner.get_latest_runner(inner_runner.node)
            if inner_runner is None:
                raise RuntimeError(f"missing lifted inner {inner_runner!r} in {runner!r}")

        # close runner once we're done
        if runner.is_root and runner.status.is_terminal:
            runner.close()
            if runner.id in self._runners_by_thread_id:
                self._runners_by_thread_id[runner.id].remove(runner)
                # also close thread if it's no longer needed
                if not self._runners_by_thread_id[runner.id]:
                    thread = runner.thread
                    assert thread is not None, f"{runner!r} has no thread"
                    thread.close()
                    del self._runners_by_thread_id[thread.id]

        return inner_runner

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

        if self._is_stop_requested:
            raise RuntimeError(f"{self!r} was stopped")

        runs_by_parent: dict[Package | Thread | Run | None, list[Run]] = group_by(
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
        """Kill a Run that is currently active in this Runtime (and any inside it)."""
        root_runner = self._runners_by_id.get(run.id)
        if root_runner is None:
            raise RuntimeError(f"no active runner for {run!r} in {self!r}")
        for runner in reversed(list(root_runner.walk())):
            if not runner.status.is_terminal:
                logger.debug("runtime.stop_run", runner=runner)
                runner.stop()
