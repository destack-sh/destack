import asyncio
from contextvars import ContextVar
from datetime import datetime, timedelta
from typing import Any, Callable, Mapping, Sequence, cast
from uuid import UUID

import structlog
from git import TYPE_CHECKING
from opentelemetry import baggage, context, trace

from bench.language import (
    DEFAULT_CHECK_OPTIONS,
    NODE_CLASS_BY_TYPE,
    RUN_STATUS_BY_INTERRUPTION_TYPE,
    Action,
    BenchError,
    BreakpointSite,
    BuiltinObject,
    CheckOptions,
    ComputedValue,
    ComputedValueKind,
    ComputedValueMode,
    Connection,
    CustomObject,
    Error,
    ErrorKind,
    Field,
    HasContext,
    Interruption,
    Node,
    NodeGraph,
    NodeMode,
    NodeType,
    PathElementType,
    PathError,
    PathOptions,
    Resource,
    ResourceStatus,
    Run,
    RunnableNode,
    RunOptions,
    RunSpanType,
    RunStatus,
    Session,
    Type,
    TypeIn,
    ValidationError,
    WatchGetUpdate,
    check_value,
    coerce_value,
    evaluate_path,
    is_node_type,
    is_value,
    on_invalid_raise,
    run_span,
    to_type_scalar,
)
from bench.language.runtime.run import RunSpan
from bench.runtime.core import Cache, InvalidComputedError, NonRetryableError, RetryableError
from bench.utils.func import group_by
from bench.utils.naming import generate_random_name
from bench.utils.oracle import Oracle

from .runner import (
    Interrupted,
    Runner,
    RunnerAbortedEvent,
    RunnerCancelledEvent,
    RunnerCompletedEvent,
    RunnerFailedEvent,
    RunnerInterruptedEvent,
    create_run_from_node,
    restore_runner,
)

if TYPE_CHECKING:
    from bench.runtime.thread import RuntimeThread


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

DEFAULT_WAIT_TIMEOUT = timedelta(seconds=30)
DEFAULT_RESOURCE_TIMEOUT = timedelta(seconds=30)


class Runtime:
    """
    The runtime for executing Runs.
    One top-level Run/Runner is processed at a time.
    Inner runs may run in parallel in some cases.
    A Runtime is exclusively associated with one Session.

    NOTE :Robustness :Architecture: separate transactions for session and other edits?
    NOTE :Incomplete: respect RunOptions.max_concurrency,...
    NOTE :Architecture: Run started/terminated/... epochs are relative to session
        (meaning if we make local edits during a Run, the terminated_epoch is still the same)
    """

    def __init__(
        self,
        *,
        session: Session,
        cache: Cache,
        oracle: Oracle,
        thread: "RuntimeThread | None" = None,
        static_glbls: Mapping[str, Any] | None = None,
        dynamic_glbls: Mapping[str, Any] | None = None,
    ):
        from bench.runtime.browser.playwright import PlaywrightClient
        from bench.runtime.code.context import DYNAMIC_CODE_GLOBALS, STATIC_CODE_GLOBALS

        assert session.bench is not None, f"{session!r} is not attached"
        assert session.bench is not None, f"{session!r} is not attached"
        self.session = session
        self.session_ptr = session.to_ref()
        self.bench = session.bench
        self.cache = cache
        self.oracle = oracle
        self.static_glbls = static_glbls or STATIC_CODE_GLOBALS
        self.dynamic_glbls = dynamic_glbls or DYNAMIC_CODE_GLOBALS
        self.combined_glbls = {**self.static_glbls, **self.dynamic_glbls}
        self.thread = thread
        self.playwright = PlaywrightClient()

        assert session._runtime is None, f"{session!r} already in runtime {session._runtime!r}"
        self.session._runtime = self
        self._active_runner: ContextVar[Runner | None] = ContextVar("active_runner")
        self._active_runners_by_id: dict[UUID, Runner] = {}

    def __str__(self):
        return f"{len(self._active_runners_by_id)} active, {self.session!r}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

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

    def get_runs(self, runnable: RunnableNode) -> list[Run]:
        """Find all Runs of a Node in this Runtime."""
        matching_runs: list[Run] = []
        for graph in self.session._supergraph._graphs_by_node_type.get(NodeType.RUN, ()):
            for node in graph.nodes:
                if type(node) is Run and node.base == runnable:
                    matching_runs.append(node)
        matching_runs.sort(
            key=lambda r: r.terminated_at or r.interrupted_at or r.started_at or r.created_at,
            reverse=True,
        )
        return matching_runs

    def get_latest_run(self, runnable: RunnableNode) -> Run | None:
        """Find the latest Run of a Node in this Runtime."""
        matching_runs = self.get_runs(runnable)
        return matching_runs[0] if matching_runs else None

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
            target_obj._do_set(prop.name, mapped_value, coerce=False)
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
    async def _wait_for(
        self,
        nodes: Sequence[Node],
        condition: Callable[[], bool],
        timeout: timedelta = DEFAULT_WAIT_TIMEOUT,
    ):
        """
        Wait for the given nodes to reach a certain state.
        NOTE :Architecture: use Interruptions instead of 'busy' (async) wait in Runtime?
        """
        if condition():
            return  # already good

        # ensure nodes are in global graph
        assert not any(node.is_deleted for node in nodes), f"cannot watch deleted: {nodes!r}"
        if any(node._is_new for node in nodes):
            await self.session.commit()

        log = logger.bind(runtime=self, nodes=nodes, condition=condition)
        nodes_by_id: dict[UUID, Node] = {node.id: node for node in nodes}
        connections: list[Connection] = []
        subs: list[Callable[[], None]] = []
        complete_signal = asyncio.Event()

        def _stop():
            """Stop waiting."""
            for connection in connections:
                connection.close()
            connections.clear()
            for sub in subs:
                sub()
            subs.clear()

        def _check():
            """Check if the condition is met, stop if so."""
            if condition():
                log.debug("runtime.wait_for.complete")
                complete_signal.set()
                _stop()

        def _apply_update(update: WatchGetUpdate):
            """'Apply' the updates from a live connection to our graphs (patching nodes in place)."""
            touched_any = False
            for live_node in update.updated.values():
                our_node = nodes_by_id.get(live_node.id)
                if our_node is not None:
                    touched_any = True
                    our_node._patch_from(live_node)
            if touched_any:
                _check()

        try:
            # create live connections if needed
            # NOTE :Architecture: auto-update entire supergraph from connections? :SupergraphWatch
            stale_nodes = [node for node in nodes if not node._is_live]
            stale_nodes_by_type = group_by(stale_nodes, lambda node: node.metatype)
            for node_type, stale_nodes in stale_nodes_by_type.items():
                node_cls = NODE_CLASS_BY_TYPE[node_type]
                live_nodes = await node_cls.select_all().get(
                    tuple(n.to_ref() for n in stale_nodes), live=True
                )
                for live_node in live_nodes:  # also patch immediately
                    our_node = nodes_by_id.get(live_node.id)
                    if our_node is not None:
                        our_node._patch_from(live_node)
                connection = live_nodes[0]._connection
                assert live_nodes and connection, f"no live connection for {stale_nodes!r}"
                connections.append(connection)
                connection.subscribe_on_update(lambda c, u: _apply_update(cast(WatchGetUpdate, u)))
                log.trace("runtime.wait_for.subscribe", connection=connection)

            # subscribe
            for node in nodes:
                subs.append(self.session._subscribe_on_edit(node, lambda _: _check()))

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
            _stop()

    def _get_resource[R: Resource = Resource](
        self, runner: Runner, resource_type: Type | TypeIn | type[R]
    ) -> R | None:
        """Finds a Resource in the current context of a Runner."""
        resource_type = to_type_scalar(resource_type)
        assert resource_type.bench_type is not None, f"no bench_type for {resource_type!r}"

        # traverse variables up
        parent = runner
        while parent is not None:
            if parent.variables is not None:
                for field in parent.variables.fields:
                    if field.bench_type == resource_type.bench_type:
                        field_value = parent.variables._do_get(field)
                        if is_value(field_value, resource_type):
                            return cast(R, field_value)
            parent = parent.parent
        return None

    def _create_resource(
        self, resource_type: Type, title: str | None = None, **kwargs: Any
    ) -> Resource:
        """Create a new Resource of the given type."""
        # get class
        node_type = resource_type.bench_type
        assert is_node_type(node_type), f"invalid resource type {resource_type!r}"
        node_type = NodeType(node_type)
        resource_cls = NODE_CLASS_BY_TYPE[node_type]
        assert issubclass(resource_cls, Resource), f"{resource_cls} in {resource_type!r}"
        # create resource
        resource_kwargs: dict[str, Any] = {"name": generate_random_name()}
        resource_kwargs.update(kwargs)
        resource = resource_cls(**resource_kwargs)
        self.bench.append(resource)
        return resource

    async def _get_or_create_resources(
        self,
        runner: Runner,
        resources: Sequence[Resource | Type | TypeIn],
    ) -> list[Resource]:
        """Get or create the available Resources for the given Runner."""
        # TODO :Incomplete: reuse resources across (unrelated) Runs?
        resource_types = [to_type_scalar(t) for t in resources if not isinstance(t, Resource)]
        resources_to_acquire: list[Resource] = [r for r in resources if isinstance(r, Resource)]
        for resource_type in resource_types:
            node_type = resource_type.bench_type
            assert is_node_type(node_type), f"invalid resource type {resource_type!r}"
            node_type = NodeType(node_type)
            assert node_type.is_resource, f"expected resource type, got {node_type!r}"

            # check context for matching resource or create new one
            resource = self._get_resource(runner, resource_type)
            if resource is not None:
                resources_to_acquire.append(resource)
                logger.trace("runtime.acquire_resources.reuse", resource=resource)
            else:
                resource = self._create_resource(resource_type)
                resources_to_acquire.append(resource)
                logger.debug("runtime.acquire_resources.new", resource=resource)
        return resources_to_acquire

    def _set_context(self, node: HasContext):
        """Sets the current context on a RunSpan."""
        if node.session_id != self.session.id:
            node._do_set("session_ptr", self.session_ptr, validate=False)
        if node.client_id != self.session.client_id:
            node._do_set("client_ptr", self.session.client_ptr, validate=False)
        if node.machine_id != self.session.machine_id:
            node._do_set("machine_ptr", self.session.machine_ptr, validate=False)
        if node.user_id != self.session.user_id:
            node._do_set("user_ptr", self.session.user_ptr, validate=False)

    @tracer.start_as_current_span("runtime.run.attempt")
    async def _do_attempt(self, runner: Runner, span: RunSpan, attempt: int):
        """
        Perform a single attempt of a Runner in a given RunSpan.
        If the Runner is is just the RunSpan, this is the only attempt.
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
                    span._do_set("started_at", started_at, validate=False)
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
            span._do_set("status", RunStatus.COMPLETED, validate=False)
            log.debug("runtime.attempt.completed", attempt=span, span="current")
        except asyncio.CancelledError as e:
            # cancelled
            span._do_set("status", RunStatus.ABORTED, validate=False)
            error = Error.from_exception(ErrorKind.RUNTIME, e)
            span._do_set("error", error, validate=False)
            log.debug("runtime.attempt.aborted", attempt=span, span="current")
            raise
        except Interrupted as e:
            # interrupted
            status = RUN_STATUS_BY_INTERRUPTION_TYPE[e.interruption.type]
            span._do_set("status", status, validate=False)
            span._do_set("interrupted_at", self.oracle.utc(), validate=False)
            span._do_set("interruption", e.interruption, validate=False)
            log.debug("runtime.attempt.interrupted", attempt=span, span="current")
            raise
        except BaseException as e:
            # error
            span._do_set("status", RunStatus.FAILED, validate=False)
            error = Error.from_exception(ErrorKind.RUNTIME, e)
            span._do_set("error", error, validate=False)
            log.debug("runtime.attempt.failed", attempt=span, exc_info=e, span="current")
            raise
        finally:
            runner.task = None
            if span.status.is_terminal:
                if terminated_at is None:
                    terminated_at = self.oracle.utc()
                span._do_set("terminated_at", terminated_at, validate=False)
                if started_at is not None:
                    span._do_set("duration", terminated_at - started_at, validate=False)

    @tracer.start_as_current_span("runtime.prepare")
    async def _prepare_run(self, runner: Runner):
        """Prepare the Run for execution (only for Runs, not RunSpans)"""
        assert type(runner.tracked) is Run, f"expected Run, got {runner.tracked!r}"

        # compute variables/inputs/options from context (on initial attempt)
        # init variables/inputs from node
        if isinstance(runner.node, Action):
            if runner.variables is not None and runner.node.variables_packed is not None:
                runner.variables.set_default(runner.node.variables, _skip_validate=True)
            assert runner.inputs is not None, f"missing inputs in {runner!r}"
            runner.inputs.set_default(runner.node, _skip_validate=True)
            if runner.node.inputs_packed is not None:
                runner.inputs.set_default(runner.node.inputs, _skip_validate=True)
        # apply computed values
        if runner.tracked_run is not None:  # (only in tracked runs)
            try:
                for computed_value in runner.node.computed_values:
                    if computed_value.target_path is not None and computed_value.is_active:
                        self._apply_computed_value(runner, computed_value)
            except Exception as e:
                runner.status = RunStatus.FAILED
                runner.error = Error.from_exception(ErrorKind.RUNTIME, e)
                raise

        # check variables
        if runner.variable_type is not None and (variable_fields := runner.variable_type._fields):
            with tracer.start_as_current_span("runtime.check_variables"):
                variables = runner.variables
                assert variables is not None, f"missing variables in {runner!r}"
                try:
                    missing_resource_slots: list[Field] = []
                    for field in variable_fields:
                        variable_value = variables._do_get(field)
                        if (
                            variable_value is None
                            and is_node_type(field.bench_type)
                            and NodeType(field.bench_type).is_resource
                        ):
                            missing_resource_slots.append(field)
                            continue
                        else:
                            check_value(
                                variable_value,
                                field,
                                options=DEFAULT_CHECK_OPTIONS,
                                invalid=on_invalid_raise,
                            )
                except ValidationError as e:
                    runner.status = RunStatus.FAILED
                    runner.error = Error.from_exception(ErrorKind.RUNTIME, e)
                    raise

            # acquire missing resource variables
            if missing_resource_slots:
                with run_span(
                    tracer, "runtime.acquire_resources", RunSpanType.ACQUIRE, runner=runner
                ) as span:
                    resources = await self._get_or_create_resources(
                        runner=runner, resources=missing_resource_slots
                    )
                    if span is not None:
                        span.nodes = cast(list["Node"], resources)
                    for field, resource in zip(missing_resource_slots, resources):
                        variables._do_set(field, resource, validate=False)
                    await self._wait_for(
                        nodes=resources,
                        condition=lambda: all(
                            resource.status == ResourceStatus.UP for resource in resources
                        ),
                        timeout=DEFAULT_RESOURCE_TIMEOUT,
                    )
                    # nocheckin: free or decommission resourcers on Run termination?
                    #  (use Resource.active_at for timeout/keepalive as backup in Host)

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
    async def _do_run_run(self, runner: Runner):
        """Runs a Runner, retrying automatically for Runs if needed."""
        # NOTE :Performance: track attempt as efficiently as possible :RuntimeHotPath

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
                    run._do_set("attempt", retry.attempt, validate=False)
                retry.on_attempt()
                if last_attempt is not None and last_attempt.status.is_interrupted:
                    # resume interrupted attempt
                    current_attempt = last_attempt
                else:
                    # create new attempt
                    current_attempt = RunSpan(
                        parent=run,
                        type=RunSpanType.ATTEMPT,
                        status=RunStatus.RUNNING,
                        _skip_validate_self=True,
                    )
                    self.session._create(current_attempt)
                last_attempt = current_attempt
                try:
                    await self._do_attempt(
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
        finally:
            # reset active run
            self._active_runner.reset(active_runner_token)
            # update from last attempt
            last_attempt = runner.current_attempt
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
    async def _do_run_span(self, runner: Runner):
        """Runs a RunSpan Runner."""
        # Runner = RunSpan, attempt only once (no breakpoints)
        runner.status = RunStatus.RUNNING
        span = runner.tracked_span
        assert span is not None, f"missing tracked span for {runner!r}"
        active_runner_token = self._active_runner.set(runner)
        try:
            await self._do_attempt(runner=runner, span=span, attempt=0)
        finally:
            # reset active run
            self._active_runner.reset(active_runner_token)
            # update runner from span
            runner.status = span.status
            runner.error = span.error

    @tracer.start_as_current_span("runtime.run")
    async def run_runner(self, runner: Runner[Any]):
        """Runs a Runner, retrying automatically and updating the tracked Run along the way."""
        # NOTE :Performance: update the Run as efficiently as possible :RuntimeHotPath
        async with self.session.active():
            self._active_runners_by_id[runner.id] = runner
            span = runner.tracked
            self._set_context(span)
            context.attach(baggage.set_baggage("run_id", str(span.id)))

            # mark started
            if span.started_at is None:
                span._do_set("started_at", self.oracle.utc(), validate=False)
            span._do_set("status", RunStatus.RUNNING, validate=False)

            # commit intermediate session edits
            self.session.commit_optimistic()

            # actually attempt Run
            try:
                if runner.tracked_span is not None:
                    await self._do_run_span(runner)
                elif runner.tracked_run is not None:
                    if runner.status < RunStatus.RUNNING:
                        await self._prepare_run(runner)
                    await self._do_run_run(runner)
                else:
                    raise RuntimeError(f"missing tracked for {runner!r}")
            except Interrupted as e:
                if not runner.status.is_interrupted:
                    # interruption not handled in attempt loop (probably from a breakpoint)
                    last_attempt = runner.current_attempt
                    interrupted_at = (
                        last_attempt.interrupted_at
                        if last_attempt is not None
                        else self.oracle.utc()
                    )
                    runner.status = RUN_STATUS_BY_INTERRUPTION_TYPE[e.interruption.type]
                    span._do_set("interrupted_at", interrupted_at, validate=False)
                else:
                    span._do_set("interrupted_at", self.oracle.utc(), validate=False)
                span._do_set("interruption", e.interruption, validate=False)
                raise
            except BaseException:
                raise
            finally:
                # update span from runner
                if type(span) is Run:
                    if span.variables is not runner.variables:
                        span._do_set("variables", runner.variables, validate=False)
                    if span.inputs is not runner.inputs:
                        span._do_set("inputs", runner.inputs, validate=False)
                    if span.outputs is not runner.outputs:
                        span._do_set("outputs", runner.outputs, validate=False)
                if span.error is not runner.error:
                    span._do_set("error", runner.error, validate=False)
                if span.status != runner.status:
                    span._do_set("status", runner.status, validate=False)
                if runner.status.is_terminal:
                    # update terminal status
                    last_attempt = runner.current_attempt
                    if last_attempt is not None:
                        # made an attempt
                        span._do_set("terminated_at", last_attempt.terminated_at, validate=False)
                        if last_attempt.duration is not None:
                            span._do_set("duration", last_attempt.duration, validate=False)
                    elif span.terminated_at is not None:
                        # didn't make an attempt, but we have a terminated_at
                        span._do_set("duration", span.terminated_at - span.started_at)  # type: ignore
                    else:
                        # didn't make an attempt
                        span._do_set("terminated_at", self.oracle.utc(), validate=False)
                        span._do_set("duration", span.terminated_at - span.started_at)  # type: ignore
                    # close any remaining (directly) contained open Interruptions
                    self.close(span)

                # commit intermediate session edits
                self.session.commit_optimistic()

                # notify
                self._active_runners_by_id.pop(runner.id, None)
                if runner.status.is_interrupted:
                    assert runner.interruption is not None, f"missing interruption for {runner!r}"
                    runner.fire_event(
                        RunnerInterruptedEvent(runner, interruption=runner.interruption)
                    )
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

    def schedule_runner(self, runner: Runner) -> Runner:
        """Schedule a Runner to run asynchronously."""
        runner.outer_task = asyncio.create_task(self._wrap_run_runner(runner))
        return runner

    async def run(
        self,
        run: Run | RunnableNode,
        *,
        variables: Any | None = None,
        inputs: Any | None = None,
        options: RunOptions | None = None,
        mode: NodeMode | None = None,
        return_error: bool = False,
        optimistic: bool = False,
    ) -> Runner | None:
        """Start or resume a top-level Run in this Runtime until termination/interruption."""
        runner = None
        async with self.session.active():
            if not isinstance(run, Run):
                run = create_run_from_node(
                    run,
                    variables=variables,
                    inputs=inputs,
                    options=options,
                    mode=mode,
                    parent=self.active_run,
                    session=self.session,
                )
            try:
                runner = restore_runner(runtime=self, run=run)
                await self.run_runner(runner)
                logger.info("runtime.run", run=run, runner=runner, span="current")
            except (Interrupted, asyncio.CancelledError):
                pass  # already handled, not a top-level error
            except (BenchError, ValueError, TypeError) as e:
                # re-raised inner user error
                if run.status != RunStatus.FAILED:
                    run.status = RunStatus.FAILED
                    run.error = Error.from_exception(ErrorKind.RUNTIME, e)
                self.session.commit_optimistic()
                logger.info("runtime.run.error", run=run, exc_info=e, span="current")
                if not return_error:
                    raise
            except BaseException as e:
                # some unexpected internal error
                if run.status != RunStatus.FAILED:
                    run.status = RunStatus.FAILED
                    run.error = Error.from_exception(ErrorKind.INTERNAL, e)
                self.session.commit_optimistic()
                logger.error("runtime.run.internal_error", run=run, exc_info=e, span="current")
                if not return_error:
                    raise
            finally:
                if not optimistic:
                    await self.session.commit()
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

    def resume(self, *runs: Run):
        """Resume interrupted Runs. Does *not* mark the Run or close open Interruptions."""
        runs_by_parent_id: dict[UUID | None, list[Run]] = group_by(
            runs, key=lambda run: run.parent_id
        )
        for parent_id, child_runs in runs_by_parent_id.items():
            if parent_id is None:
                continue  # can't resume top-level Run
            parent_runner = self._active_runners_by_id.get(parent_id)
            if parent_runner is not None:
                parent_runner.resume(child_runs)

    def stop(self, run: Run):
        """Kill a Run that is currently active in this Runtime (and any inside it)."""
        root_runner = self._active_runners_by_id.get(run.id)
        if root_runner is None:
            raise RuntimeError(f"no active runner for {run!r} in {self!r}")
        for runner in reversed(list(root_runner.walk())):
            if not runner.status.is_terminal:
                runner.stop()
                if runner.tracked_run is not None:
                    self.close(runner.tracked_run, resume=not runner.is_root)
                logger.debug("runtime.run.stop", runner=runner)

    def close(self, span: Run | RunSpan, resume: bool = True):
        """Close the Interruptions in a Run."""
        closed_interruptions: list[Interruption] | None = None
        for interruption in span._graph.iter_descendants(span, NodeType.INTERRUPTION):
            interruption = cast(Interruption, interruption)
            if interruption.status.is_open:
                interruption.cancel(_trigger_runtime=False)
                if closed_interruptions is None:
                    closed_interruptions = []
                closed_interruptions.append(interruption)

        # trigger resume for Interruptions (if we can still run, i.e. not at root)
        if resume and span.parent_ptr is not None and closed_interruptions:
            runs_to_resume = self.get_interrupted_runs(span._graph, *closed_interruptions)
            self.resume(*runs_to_resume)
