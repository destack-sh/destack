import asyncio
from contextvars import ContextVar
from datetime import datetime, timedelta
from typing import Any, Callable, Mapping, Sequence, cast
from uuid import UUID

import structlog
from git import TYPE_CHECKING
from opentelemetry import baggage, context, trace

from bench.language.bench import Resource, ResourceStatus
from bench.language.connection import Connection, WatchGetUpdate
from bench.language.const import (
    BenchError,
    NodeMode,
    NodeType,
    ObjectKind,
    RunErrorKind,
    RunStatus,
    is_node_type,
)
from bench.language.field import Field, TypeIn, TypeInfo, to_type_scalar
from bench.language.graph import NodeGraph
from bench.language.interrupt import RUN_STATUS_BY_INTERRUPT_TYPE, BreakpointSite, Interrupt
from bench.language.node import Node
from bench.language.registry import NODE_CLASS_BY_TYPE
from bench.language.run import Run, RunAttempt, RunError, RunnableNode
from bench.language.session import Session
from bench.language.validation import ValidationError, on_invalid_raise
from bench.language.value import (
    DEFAULT_CHECK_OPTIONS,
    CheckOptions,
    CustomObject,
    check_value,
    is_value,
)
from bench.runtime.cache import Cache
from bench.runtime.core import (
    DYNAMIC_CODE_GLOBALS,
    STATIC_CODE_GLOBALS,
    NonRetryableError,
    RetryableError,
)
from bench.runtime.runner import (
    Interrupted,
    Runner,
    RunnerHook,
    make_run_from_node,
    restore_runner,
)
from bench.utils.func import group_by
from bench.utils.oracle import Oracle
from bench.utils.tenacity import RetryState

if TYPE_CHECKING:
    from bench.runtime.thread import RuntimeThread


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

# nocheckin: adjust timeouts
DEFAULT_TIMEOUT = timedelta(seconds=10)
DEFAULT_RESOURCE_TIMEOUT = timedelta(seconds=10)


class Runtime:
    """
    The runtime for executing Runs.
    One top-level Run/Runner is processed at a time.
    Inner runs may run in parallel in some cases.
    A Runtime is exclusively associated with one Session.

    NOTE :Robustness :Architecture: separate transactions for session and other edits?
    NOTE :Incomplete: respect RunOptions.max_concurrency
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
        static_glbls: Mapping[str, Any] = STATIC_CODE_GLOBALS,
        dynamic_glbls: Mapping[str, Any] = DYNAMIC_CODE_GLOBALS,
    ):
        assert session.bench is not None, f"{session!r} is not attached"
        self.session = session
        self.bench = session.bench
        self.cache = cache
        self.oracle = oracle
        self.static_glbls = static_glbls
        self.thread = thread
        self.dynamic_glbls = dynamic_glbls
        self.combined_glbls = {**static_glbls, **dynamic_glbls}

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

    @tracer.start_as_current_span("runtime.wait_for")
    async def _wait_for(
        self,
        nodes: Sequence[Node],
        condition: Callable[[], bool],
        timeout: timedelta = DEFAULT_TIMEOUT,
    ):
        """
        Wait for the given nodes to reach a certain state.
        NOTE :Architecture: use Interrupts instead of 'busy' (async) wait in Runtime?
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
                live_nodes = await node_cls.get(tuple(n.to_ref() for n in stale_nodes), live=True)
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
        except BaseException as e:
            log.error("runtime.wait_for.error", exc_info=e)
            raise
        finally:
            _stop()

    def _get_resource[R: Resource = Resource](
        self, runner: Runner, resource_type: TypeInfo | TypeIn | type[R]
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

    @tracer.start_as_current_span("runtime.acquire_resources")
    async def _acquire_resources(
        self, runner: Runner, resource_types: Sequence[TypeInfo | TypeIn]
    ) -> list[Resource]:
        """Acquires the relevant Resources for the given Runner."""
        # TODO :Incomplete: reuse resources across (unrelated) Runs?
        resource_types = [to_type_scalar(t) for t in resource_types]
        resources: list[Resource] = []
        now = self.oracle.utc()
        for resource_type in resource_types:
            node_type = resource_type.bench_type
            assert is_node_type(node_type) is not None, f"invalid resource type {resource_type!r}"
            node_type = NodeType(node_type)
            assert node_type.is_resource, f"expected resource type, got {node_type!r}"

            # check context for matching resource
            resource = self._get_resource(runner, resource_type)
            if resource is not None:
                resources.append(resource)
                continue

            # make new resource
            resource_cls = NODE_CLASS_BY_TYPE[node_type]
            assert issubclass(resource_cls, Resource), f"{resource_cls} in {resource_type!r}"
            resource_kwargs: dict[str, Any] = {}
            if "title" in resource_cls.__properties__:
                resource_kwargs["title"] = f"{resource_cls.__name__} {now.strftime('%Y-%m-%d')}"
            resource = resource_cls(**resource_kwargs)
            self.bench.append(resource)
            resources.append(resource)
            logger.debug("runtime.acquire_resources.new", resource=resource)

        # commit and wait for resources to become ready
        await self._wait_for(
            nodes=resources,
            condition=lambda: all(resource.status == ResourceStatus.UP for resource in resources),
            timeout=DEFAULT_RESOURCE_TIMEOUT,
        )

        return resources

    @tracer.start_as_current_span("runtime.run_runner.attempt")
    async def _do_attempt(self, runner: Runner, retry: RetryState, attempt: RunAttempt):
        trace.get_current_span().set_attribute("runner", repr(runner))
        trace.get_current_span().set_attribute("attempt", retry.attempt)
        log = logger.bind(runner=runner, attempt=attempt, retry=retry)
        started_at: datetime | None = None
        terminated_at: datetime | None = None
        try:
            if runner.is_stopped:
                raise asyncio.CancelledError()
            with tracer.start_as_current_span("runtime.attempt.run"):
                if attempt.started_at is None:
                    started_at = self.oracle.utc()
                    attempt._do_set("started_at", started_at, validate=False)
                runner.task = asyncio.create_task(runner.run())
                await runner.task
                terminated_at = self.oracle.utc()

            # check outputs
            if runner.output_type is not None:
                with tracer.start_as_current_span("runtime.check_outputs"):
                    if runner.outputs is None:
                        runner.outputs = CustomObject.new(
                            ObjectKind.OUTPUT,
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
            attempt._do_set("status", RunStatus.COMPLETED, validate=False)
            log.debug("runtime.attempt.completed", attempt=attempt, span="current")
        except asyncio.CancelledError as e:
            # cancelled
            error = RunError.from_exception(RunErrorKind.RUNTIME, e)
            attempt._do_set("status", RunStatus.ABORTED, validate=False)
            attempt._do_set("error", error, validate=False)
            log.debug("runtime.attempt.aborted", attempt=attempt, span="current")
            raise
        except Interrupted as e:
            # interrupted
            status = RUN_STATUS_BY_INTERRUPT_TYPE[e.interrupt.type]
            attempt._do_set("status", status, validate=False)
            attempt._do_set("interrupted_at", self.oracle.utc(), validate=False)
            attempt._do_set("interrupt", e.interrupt, validate=False)
            log.debug("runtime.attempt.interrupted", attempt=attempt, span="current")
            raise
        except BaseException as e:
            # error
            error = RunError.from_exception(RunErrorKind.RUNTIME, e)
            attempt._do_set("error", error, validate=False)
            attempt._do_set("status", RunStatus.FAILED, validate=False)
            log.debug("runtime.attempt.failed", attempt=attempt, exc_info=e, span="current")
            if not error.is_retryable or (
                not retry.on_error(e) and not (error.type and error.type in runner.options.retry_on)
            ):
                raise  # give up if not retryable (anymore)
        finally:
            runner.task = None
            if attempt.status.is_terminal:
                if terminated_at is None:
                    terminated_at = self.oracle.utc()
                attempt._do_set("terminated_at", terminated_at, validate=False)
                attempt._do_set("terminated_epoch", self.session.epoch, validate=False)
                if started_at is not None:
                    attempt._do_set("duration", terminated_at - started_at, validate=False)

    @tracer.start_as_current_span("runtime.run_runner.run")
    async def _do_run(self, runner: Runner):
        """Runs a a Runner, retrying automatically and updating the Runner along the way."""
        # NOTE :Performance: track attempt as efficiently as possible :RuntimeHotPath

        # check variables
        if runner.variable_type is not None:
            with tracer.start_as_current_span("runtime.check_variables"):
                variables = runner.variables
                assert variables is not None, f"missing variables in {runner!r}"
                try:
                    missing_resource_slots: list[Field] = []
                    for field in runner.variable_type._fields:
                        variable_value = variables._do_get(field)
                        if (
                            variable_value is None
                            and is_node_type(field.bench_type) is not None
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
                    runner.error = RunError.from_exception(RunErrorKind.RUNTIME, e)
                    return

            # acquire missing resource variables
            if missing_resource_slots:
                resources = await self._acquire_resources(runner, missing_resource_slots)
                for field, resource in zip(missing_resource_slots, resources):
                    variables._do_set(field, resource, validate=False)

        # check inputs
        if runner.input_type is not None:
            with tracer.start_as_current_span("runtime.check_inputs"):
                inputs = runner.inputs or CustomObject.new(
                    ObjectKind.INPUT, {}, runner.input_type, supergraph=self.session._supergraph
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
                    runner.error = RunError.from_exception(RunErrorKind.RUNTIME, e)
                    return

        # set status
        runner.status = RunStatus.RUNNING
        retry = runner.options.to_retry().new(self.oracle, attempt=len(runner.attempts))
        last_attempt = runner.current_attempt
        for attempt in runner.attempts:  # 'restore' errors
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

        # run
        active_run_runner_token = self._active_runner.set(runner)
        try:
            # make new attempts if we can/should
            while retry.should_retry and not (
                last_attempt is not None and last_attempt.status == RunStatus.COMPLETED
            ):
                retry.on_attempt()
                if last_attempt is not None and last_attempt.status.is_interrupted:
                    current_attempt = last_attempt
                else:
                    current_attempt = RunAttempt(
                        status=RunStatus.RUNNING,
                        started_epoch=self.session.epoch,
                        _skip_validate_self=True,
                    )
                    runner.attempts.append(current_attempt)
                last_attempt = current_attempt
                await self._do_attempt(runner=runner, retry=retry, attempt=current_attempt)
                if last_attempt.error is not None:
                    retry.on_error(last_attempt.error)

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
            self._active_runner.reset(active_run_runner_token)
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

            # run status = last attempt
            runner.status = last_attempt.status
            runner.error = last_attempt.error

    @tracer.start_as_current_span("runtime.run_runner.track")
    async def _do_run_tracked(self, runner: Runner):
        """Runs a Runner, retrying automatically and updating the tracked Run along the way."""
        # NOTE :Performance: update the Run as efficiently as possible :RuntimeHotPath
        # NOTE :UX :Performance: commit optimistically ideally only while inside user code
        #  (while we're inside a leaf Runner.run, but not while updating/creating Runs,
        #   so for instance inside a Flow we should wait for all initial Steps to start somehow)

        run = runner.tracked_run
        assert run is not None, f"missing run in {runner!r}"
        context.attach(baggage.set_baggage("run_id", str(run.id)))

        # update context
        if run.session_id != self.session.id:
            run._do_set("session", self.session, validate=False)
        run._do_set("client_ptr", self.session.client_ptr, validate=False)
        run._do_set("machine_ptr", self.session.machine_ptr, validate=False)
        run._do_set("server_ptr", self.session.server_ptr, validate=False)
        run._do_set("user_ptr", self.session.user_ptr, validate=False)

        # mark started
        if run.started_at is None:
            run._do_set("started_epoch", self.session.epoch, validate=False)
            run._do_set("started_at", self.oracle.utc(), validate=False)
        run._do_set("status", RunStatus.RUNNING, validate=False)

        # commit intermediate session edits
        self.session.commit_optimistic()

        # actually attempt Run
        try:
            await self._do_run(runner)
        except Interrupted as e:
            if not runner.status.is_interrupted:
                # interrupt not handled in attempt loop (probably from a breakpoint)
                last_attempt = runner.current_attempt
                interrupted_at = (
                    last_attempt.interrupted_at if last_attempt is not None else self.oracle.utc()
                )
                runner.status = RUN_STATUS_BY_INTERRUPT_TYPE[e.interrupt.type]
                run._do_set("interrupted_at", interrupted_at, validate=False)
            else:
                run._do_set("interrupted_at", self.oracle.utc(), validate=False)
            run._do_set("interrupt", e.interrupt, validate=False)
            raise
        finally:
            run._do_set("attempts", runner.attempts, validate=False)
            run._do_set("logs", runner.logs, validate=False)
            run._do_set("inputs", runner.inputs, validate=False)
            run._do_set("outputs", runner.outputs, validate=False)
            run._do_set("error", runner.error, validate=False)
            run._do_set("status", runner.status, validate=False)
            if runner.status.is_terminal:
                # update terminal status
                last_attempt = runner.current_attempt
                if last_attempt is not None:
                    # made an attempt
                    run._do_set("terminated_at", last_attempt.terminated_at, validate=False)
                    run._do_set("terminated_epoch", last_attempt.terminated_epoch, validate=False)
                    if last_attempt.duration is not None:
                        run._do_set("duration", last_attempt.duration, validate=False)
                elif run.terminated_at is not None:
                    # didn't make an attempt, but we have a terminated_at
                    run._do_set("duration", run.terminated_at - run.started_at)  # type: ignore
                else:
                    # didn't make an attempt
                    run._do_set("terminated_at", self.oracle.utc(), validate=False)
                    run._do_set("terminated_epoch", self.session.epoch, validate=False)
                    run._do_set("duration", run.terminated_at - run.started_at)  # type: ignore

                # close any remaining (directly) contained open Interrupts
                self.close(run)

            # commit intermediate session edits
            self.session.commit_optimistic()

    @tracer.start_as_current_span("runtime.run_runner")
    async def run_runner(self, runner: Runner, hook: RunnerHook | None = None):
        """Runs a Runner until termination/interruption."""
        async with self.session.active():
            self._active_runners_by_id[runner.id] = runner
            exc = None
            try:
                trace.get_current_span().set_attribute("runner", repr(runner))
                if runner.tracked_run is not None:
                    await self._do_run_tracked(runner)
                else:
                    await self._do_run(runner)
            except BaseException as e:
                exc = e
                raise
            finally:
                self._active_runners_by_id.pop(runner.id, None)
                if not runner.is_tracked and runner.parent is not None:
                    # add inner spans/logs/events to parent
                    runner.parent.logs.extend(runner.logs)
                    runner.parent.events.extend(runner.events)
                    runner.parent.spans.extend(runner.spans)
                if hook is not None:
                    hook(runner, exc)

    async def _wrap_run_runner(self, runner: Runner, hook: RunnerHook | None = None):
        """Run the runner at the top-level, handling any exceptions."""
        try:
            await self.run_runner(runner, hook=hook)
        except (Interrupted, asyncio.CancelledError):
            pass  # already handled, not a top-level error
        except Exception as e:
            logger.error("runtime.run_runner.error", runner=runner, exc_info=e)

    def schedule_runner(self, runner: Runner, on_stop: RunnerHook | None = None) -> Runner:
        """Schedule a Runner to run asynchronously."""
        runner.outer_task = asyncio.create_task(self._wrap_run_runner(runner, hook=on_stop))
        return runner

    @tracer.start_as_current_span("runtime.run")
    async def run(
        self,
        run: Run | RunnableNode,
        *,
        variables: Any | None = None,
        inputs: Any | None = None,
        mode: NodeMode | None = None,
        return_error: bool = False,
        optimistic: bool = False,
    ) -> Runner | None:
        """Start or resume a top-level Run in this Runtime until termination/interruption."""
        if not isinstance(run, Run):
            run = make_run_from_node(
                run, variables=variables, inputs=inputs, mode=mode, parent=self.active_run
            )
            self.session._create(run)
        runner = None
        async with self.session.active():
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
                    run.error = RunError.from_exception(RunErrorKind.RUNTIME, e)
                self.session.commit_optimistic()
                logger.info("runtime.run.error", run=run, exc_info=e, span="current")
                if not return_error:
                    raise
            except BaseException as e:
                # some unexpected internal error
                if run.status != RunStatus.FAILED:
                    run.status = RunStatus.FAILED
                    run.error = RunError.from_exception(RunErrorKind.INTERNAL, e)
                self.session.commit_optimistic()
                logger.error("runtime.run.internal_error", run=run, exc_info=e, span="current")
                if not return_error:
                    raise
            finally:
                if not optimistic:
                    await self.session.commit()
        return runner

    def get_interrupted_runs(self, graph: NodeGraph, *interrupts: Interrupt) -> list[Run]:
        """Gets all Runs that were directly interrupted by the given Interrupts."""
        interrupted_runs: list[Run] = []
        for run in graph.nodes_of_type(Run):
            interrupt = run.interrupt
            if run.status.is_interrupted and interrupt is not None and interrupt in interrupts:
                interrupted_runs.append(run)
        return interrupted_runs

    def resume(self, *runs: Run):
        """Resume interrupted Runs. Does *not* mark the Run or close open Interrupts."""
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

    def close(self, run: Run, resume: bool = True):
        """Close the Interrupts in a Run."""
        closed_interrupts: list[Interrupt] | None = None
        for interrupt in run._graph.iter_descendants(run, NodeType.INTERRUPT):
            interrupt = cast(Interrupt, interrupt)
            if interrupt.status.is_open:
                interrupt.cancel(_trigger_runtime=False)
                if closed_interrupts is None:
                    closed_interrupts = []
                closed_interrupts.append(interrupt)

        # trigger resume for Interrupts (if we can still run, i.e. not at root)
        if resume and run.parent_ptr is not None and closed_interrupts:
            runs_to_resume = self.get_interrupted_runs(run._graph, *closed_interrupts)
            self.resume(*runs_to_resume)
