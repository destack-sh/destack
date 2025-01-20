import abc
import asyncio
from datetime import timedelta
from typing import (
    TYPE_CHECKING,
    Any,
    Callable,
    ClassVar,
    Iterable,
    Literal,
    Sequence,
    assert_never,
    cast,
    final,
)

import structlog
from opentelemetry import trace

from bench.language import (
    Action,
    Bench,
    Block,
    Breakpoint,
    BreakpointScope,
    BreakpointSite,
    Code,
    CustomObject,
    HasContext,
    Interruption,
    InterruptionStatus,
    InterruptionType,
    Log,
    Node,
    NodeMode,
    Pipe,
    Resource,
    ResourceStatus,
    Run,
    RunError,
    RunnableNode,
    RunOptions,
    RunSpan,
    RunSpanType,
    RunStatus,
    RunType,
    Session,
    Type,
    TypeBase,
    TypeIn,
    active_session,
    get_tracing_context,
)

from .error import InterruptionCancelledError, RunImpossibleError
from .options import BASE_RUN_OPTIONS_BY_KIND

if TYPE_CHECKING:
    from .runtime import Runtime


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class Interrupted(Exception):  # noqa: N818
    """A Runner/Run is interrupted."""

    def __init__(self, runner: "Runner", run: Run, interruption: Interruption):
        self.runner = runner
        self.run = run
        self.interruption = interruption


RunIn = Run | RunSpan | RunSpanType | Literal["track"]
RunnerHook = Callable[["Runner", BaseException | None], None]


class Runner[N: RunnableNode = RunnableNode](abc.ABC):
    """
    A Runner to run a Run/RunSpan (every Run has one Runner, some RunSpans have one).
    Runners work similar to asyncio Tasks, making progress until terminated or stopped by an Interruption.
    After an Interruption is handled, we try to run the Runner again - it may progress or raise another Interruption.
    Interruption-capable Runners may be nested, and it's the responsibility of the Runners
     to ensure replay stability in all sub-Runners when resuming after an Interruption.
    """

    __slots__ = (
        "context",
        "error",
        "id",
        "input_type",
        "inputs",
        "is_stopped",
        "mode",
        "node",
        "options",
        "outer_task",
        "output_type",
        "outputs",
        "parent",
        "runners",
        "runtime",
        "status",
        "task",
        "tracked",
        "tracked_run",
        "tracked_span",
        "variable_type",
        "variables",
    )

    runner_type: ClassVar[RunType]

    def __init__(
        self,
        *,
        runtime: "Runtime",
        node: N,
        options: RunOptions,
        context: HasContext,
        run: RunIn,
        parent: "Runner[Any] | None" = None,
        variables: CustomObject | None = None,
        inputs: CustomObject | None = None,
        outputs: TypeBase | CustomObject | None = None,
    ) -> None:
        self.runtime = runtime
        self.node = node
        self.status = RunStatus.QUEUED
        self.options = options
        self.context: HasContext = context
        self.parent = parent or runtime.active_runner
        self.runners: list[Runner] = []
        self.task: asyncio.Task | None = None
        self.outer_task: asyncio.Task | None = None
        self.is_stopped = False

        # variables/inputs/outputs
        self.variables = variables
        self.variable_type = node.variable_type
        assert (
            self.variable_type is None or self.variables is not None
        ), f"{self!r} has no variables"
        self.inputs: CustomObject | None = inputs
        self.input_type = node.input_type
        assert self.input_type is None or self.inputs is not None, f"{self!r} has no inputs"
        if isinstance(outputs, TypeBase):
            self.outputs: CustomObject | None = None
            self.output_type = outputs or node.output_type
        elif isinstance(outputs, CustomObject):
            self.outputs = outputs
            self.output_type = outputs._type
        else:
            self.outputs = None
            self.output_type = node.output_type
        self.error: RunError | None = None

        # context/tracing
        if self.parent is not None:
            self.parent.runners.append(self)  # register with parent Runner
            parent_run = self.parent.closest_tracked_run
        else:
            parent_run = None
        if self.parent is not None:
            self.mode = self.parent.mode
        elif isinstance(run, Node):
            self.mode = run.mode
        else:
            self.mode = NodeMode.PRODUCTION

        # track in Run/RunSpan
        self.tracked_run: Run | None
        self.tracked_span: RunSpan | None
        self.tracked: RunSpan | Run
        if type(run) is Run or run == "track":
            # Runner = Run
            if run == "track":
                tracked_run = Run(
                    parent=parent_run or self.runtime.bench,
                    type=self.runner_type,
                    block=node if isinstance(node, Block) else node.block,
                    action=node if isinstance(node, Action) else None,
                    pipe=node if isinstance(node, Pipe) else None,
                    options=options,
                    status=self.status,
                    mode=self.mode,
                    inputs=self.inputs,
                    variables=self.variables,
                    session=self.session,
                    _skip_validate_self=True,
                )
                self.session._create(tracked_run)
            else:
                tracked_run = cast(Run, run)
            self.id = tracked_run.id
            # Run and Runner *must* share the same objects (so we can apply computed values)
            # get objects from Run (Node may copy them if they're from a different parent,
            #  like when we re-use a Flow's inputs for the StartAction.inputs)
            self.inputs = tracked_run.inputs
            self.variables = tracked_run.variables
            self.options = tracked_run.options
            self.tracked_run = tracked_run
            self.tracked_span = None
            self.tracked = tracked_run
            assert (
                tracked_run.variables is self.variables
            ), f"{tracked_run!r} has other variables than {self!r}"
            assert (
                tracked_run.inputs is self.inputs
            ), f"{tracked_run!r} has other inputs than {self!r}"
            assert (
                tracked_run.options is self.options
            ), f"{tracked_run!r} has other options than {self!r}"
        else:
            # Runner = RunSpan
            if type(run) is RunSpanType:
                assert parent_run is not None, f"{self!r} has no parent Run"
                tracked_span = RunSpan(
                    parent=parent_run,
                    type=run,
                    status=self.status,
                    mode=self.mode,
                    _skip_validate_self=True,
                )
                self.session._create(tracked_span)
            else:
                tracked_span = cast(RunSpan, run)
            self.tracked_run = None
            self.tracked_span = tracked_span
            self.tracked = self.tracked_span
        self.id = self.tracked.id

        # context
        if self.context is None:
            self.context = self.tracked

    def __str__(self):
        str_parts: list[str] = [
            self.status.bench_name,
            f"node={self.node!r}",
            f"options={self.options!r}",
        ]
        if self.runners:
            str_parts.append(f"runs={len(self.runners)}")
        if self.tracked_run:
            str_parts.append(f"run={self.tracked_run}")
        return ", ".join(str_parts)

    def __repr__(self):
        content_str = str(self)
        return (
            f"<{self.__class__.__name__} {content_str}>"
            if content_str
            else f"<{self.__class__.__name__}>"
        )

    def walk(self) -> Iterable["Runner"]:
        yield self
        for run in self.runners:
            yield from run.walk()

    @property
    def session(self):
        return self.runtime.session

    @property
    def is_active(self) -> bool:
        return self.outer_task is not None and not self.outer_task.done()

    @property
    def is_root(self) -> bool:
        return self.parent is None

    @property
    def closest_tracked_run(self) -> Run | None:
        runner = self
        while runner is not None:
            if runner.tracked_run is not None:
                return runner.tracked_run
            runner = runner.parent
        return None

    @property
    def should_pause(self) -> bool:
        if self.options.suppress_pause or self.tracked_run is None:
            return False
        runner = self
        while runner is not None:
            if (
                runner.tracked_run is not None
                and runner.tracked_run.paused_at
                and not runner.options.suppress_pause
                and (
                    not runner.tracked_run.resumed_at
                    or runner.tracked_run.resumed_at < runner.tracked_run.paused_at
                )
            ):
                return True
            runner = runner.parent
        return False

    @property
    def attempts(self) -> Sequence[RunSpan]:
        if self.tracked_run is None:
            return ()
        return tuple(span for span in self.tracked_run.spans if span.type == RunSpanType.ATTEMPT)

    @property
    def current_attempt(self) -> RunSpan | None:
        if self.tracked_run is None:
            return None
        for span in reversed(self.tracked_run.spans):
            if span.type == RunSpanType.ATTEMPT:
                return span

    @property
    def logs(self) -> Sequence[Log]:
        if self.tracked_run is not None:
            return self.tracked_run.logs
        elif (run := self.closest_tracked_run) is not None:
            return run.logs
        else:
            return ()

    @property
    def ancestors(self):
        parent = self
        while parent is not None:
            yield parent
            parent = parent.parent

    #
    # Interruptions
    #

    @property
    def interruption(self) -> Interruption | None:
        assert self.tracked_run is not None, f"{self!r} is not tracked"
        return self.tracked_run.interruption

    @property
    def breakpoints(self) -> Sequence[Breakpoint]:
        if self.tracked_run is None or self.tracked_run.options is None:
            return ()
        else:
            return self.tracked_run.options.breakpoints

    def _has_breakpoint_set(self, *sites: BreakpointSite):
        """Gets all Breakpoints applicable to this Runner."""
        for bp in self.breakpoints:
            if bp.scope == BreakpointScope.SELF and bp.site in sites:
                return True
        if self.parent is not None:
            for bp in self.parent.breakpoints:
                if bp.scope == BreakpointScope.CHILD and bp.site in sites:
                    return True
        return False

    def _get_or_create_interruption(
        self,
        kind: InterruptionType,
        *,
        attempt: int | None = None,
        breakpoint: BreakpointSite | None = None,
    ):
        """Gets an Interrupt in the current Runner of the given shape (or creates one)."""
        assert self.tracked_run is not None, f"{self!r} is not tracked"
        for interruption in self.tracked_run.interruptions:
            if (
                interruption.type == kind
                and interruption.attempt_no == attempt
                and interruption.breakpoint_site == breakpoint
            ):
                return interruption
        interruption = Interruption.from_run(kind, self.tracked_run, breakpoint=breakpoint)
        self.session._create(interruption)
        return interruption

    def _trap_interruption(
        self,
        kind: InterruptionType,
        *,
        attempt: int | None = None,
        breakpoint: BreakpointSite | None = None,
    ):
        """Yield/resume an Interrupt of the given kind if set in this Runner."""
        assert self.tracked_run is not None, f"{self!r} is not tracked"
        interruption = self._get_or_create_interruption(
            kind, attempt=attempt, breakpoint=breakpoint
        )
        if interruption.status == InterruptionStatus.COMPLETED:
            logger.trace(
                f"runtime.{kind.name.lower()}.completed", runner=self, interrupt=interruption
            )
            return interruption  # interrupt already handled
        elif interruption.status == InterruptionStatus.CANCELLED:
            logger.trace(
                f"runtime.{kind.name.lower()}.cancelled", runner=self, interrupt=interruption
            )
            raise InterruptionCancelledError(f"{interruption!r} is cancelled")
        else:
            logger.trace(f"runtime.{kind.name.lower()}", runner=self, interrupt=interruption)
            raise Interrupted(self, self.tracked_run, interruption)

    @final
    def _trap_breakpoint(self, site: BreakpointSite, *alias_sites: BreakpointSite):
        """Yield/resume the given kind of breakpoint if set in this Runner."""
        if self.tracked_run is None:
            return  # can't break into untracked Run
        # handle applicable breakpoint (if any)
        if self._has_breakpoint_set(site, *alias_sites):
            self._trap_interruption(InterruptionType.YIELD, breakpoint=site)

    @final
    def _trap_pause(self):
        """Yield/resume a pause the given Runner if required."""
        if self.tracked_run is None:
            return  # can't break into untracked Run
        if self.should_pause:
            self._trap_interruption(InterruptionType.PAUSE)

    async def _wait_for(
        self, nodes: Sequence[Node], complete_when: Callable[[], bool], timeout: timedelta
    ):
        """Wait for the given nodes to reach a certain state."""
        return await self.runtime._wait_for(nodes, complete_when, timeout)

    def _get_resource[R: Resource = Resource](
        self, resource_type: Type | TypeIn | type[R]
    ) -> R | None:
        """Finds a Resource in the current context of a Runner."""
        return self.runtime._get_resource(self, resource_type)

    def _get_ready_resource_or_error[R: Resource = Resource](
        self, resource_type: Type | TypeIn | type[R]
    ) -> R:
        """Finds a Resource in the current context of a Runner."""
        resource = self._get_resource(resource_type)
        if resource is None:
            raise LookupError(f"no resource like {resource_type} found in {self!r}")
        if resource.status != ResourceStatus.UP:
            raise RuntimeError(f"resource {resource!r} is not ready in {self!r}")
        return resource

    #
    # Run
    #

    @abc.abstractmethod
    async def run(self) -> None:
        """
        Run until the Run stops (for now):
         - On completion, return normally.
         - On error, raise an exception.
         - On cancel, (re)raise the CancelledError.
         - On interrupt, raise Interrupted.
        """
        ...

    def resume(self, runs: Sequence[Run]):  # noqa: B027
        """Resume inner Runs."""
        pass

    def stop(self):
        """Stop this Runner/Run."""
        if self.status.is_terminal:
            return
        self.is_stopped = True
        if self.outer_task is not None:
            self.outer_task.cancel()
        if self.task is not None:
            self.task.cancel()
        if self.tracked_run is not None and not self.is_active:
            self.tracked_run._mark_stopped()
            self.status = self.tracked_run.status


def create_run_from_node(
    node: "RunnableNode",
    *,
    variables: Any | None = None,
    inputs: Any | None = None,
    options: RunOptions | None = None,
    mode: NodeMode | None = None,
    parent: "Run | Bench | None" = None,
    session: "Session | None" = None,
) -> "Run":
    """Creates a Run from a runnable Node."""
    from bench.language import Action, Block, coerce_custom_object_scalar

    # context
    if session is None:
        session = active_session()
    if isinstance(node, Block):
        block = node
        action = None
        pipe = None
        kind = node.run_type
        assert kind is not None, f"no run kind for {node!r}"
    elif isinstance(node, Action):
        action = node
        block = action.block
        pipe = None
        kind = RunType.ACTION
    elif isinstance(node, Pipe):
        pipe = node
        block = pipe.block
        action = None
        kind = RunType.PIPE
    else:
        assert_never(node)

    # build run
    tracing = get_tracing_context()
    run = Run(
        parent=parent or node.bench,
        type=kind,
        block=block,
        action=action,
        pipe=pipe,
        mode=mode or tracing.mode,
        _skip_validate_self=True,
    )

    # variables
    if variables is None:
        variables = {}
    if (variable_type := run.variable_type) is not None:
        variables = coerce_custom_object_scalar(variables, variable_type)
        run.variables = variables

    # inputs
    if inputs is None:
        inputs = {}
    if (input_type := run.input_type) is not None:
        inputs = coerce_custom_object_scalar(inputs, input_type)
        run.inputs = inputs

    # options
    if options is None:
        if isinstance(run_options := getattr(node, "run_options", None), RunOptions):
            options = run_options.clone()
            options.set_default(BASE_RUN_OPTIONS_BY_KIND[kind], copy=False)
        else:
            options = BASE_RUN_OPTIONS_BY_KIND[kind].clone()
    else:
        options.set_default(BASE_RUN_OPTIONS_BY_KIND[kind], copy=False)
    run.options = options

    session._create(run)
    return run


def restore_runner(runtime: "Runtime", run: Run) -> "Runner":
    """Make a Runner from a Run."""
    node = run.runnable
    if node is None:
        raise RunImpossibleError(f"no node for {run!r}")

    # variables
    if run.variables is None and (variable_type := run.variable_type) is not None:
        variables = CustomObject.new({}, typ=variable_type, supergraph=runtime.session._supergraph)
    else:
        variables = run.variables

    # inputs
    if run.inputs is None and (input_type := run.input_type) is not None:
        inputs = CustomObject.new({}, typ=input_type, supergraph=runtime.session._supergraph)
    else:
        inputs = run.inputs

    return make_runner(
        runtime,
        node,
        run=run,
        type=run.type,
        context=run,
        variables=variables,
        inputs=inputs,
    )


def make_runner(
    runtime: "Runtime",
    node: RunnableNode,
    run: RunIn,
    *,
    type: RunType | None = None,
    context: HasContext | None = None,
    variables: CustomObject | None = None,
    inputs: Any | None = None,
    outputs: TypeBase | CustomObject | None = None,
    options: RunOptions | None = None,
    parent: "Runner[Any] | None" = None,
) -> "Runner":
    """Make a Runner from a runnable Node."""

    RUN_TYPE = type or node.run_type
    assert RUN_TYPE is not None, f"no run kind for {node!r}"

    # variables
    variable_type = node.variable_type
    if variables is None and variable_type is not None:
        variables = CustomObject.new({}, typ=variable_type, supergraph=runtime.session._supergraph)

    # inputs
    input_type = node.input_type
    if inputs is None and input_type is not None:
        inputs = CustomObject.new({}, typ=input_type, supergraph=runtime.session._supergraph)

    # options
    if options is None and isinstance(
        run_options := getattr(node, "run_options", None), RunOptions
    ):
        options = run_options.clone()
    if options is None:
        options = BASE_RUN_OPTIONS_BY_KIND[RUN_TYPE].clone()
    else:
        options.set_default(BASE_RUN_OPTIONS_BY_KIND[RUN_TYPE], copy=False)

    # build runner
    base_kwargs: dict[str, Any] = {
        "runtime": runtime,
        "options": options,
        "context": context or run,
        "variables": variables,
        "inputs": inputs,
        "outputs": outputs,
        "run": run,
        "node": node,
        "parent": parent,
    }

    # map to runner
    if RUN_TYPE == RunType.CODE:
        from bench.runtime.code.code import CodeFunctionRunner

        code = getattr(node, "code", None) or Code.empty()
        runner = CodeFunctionRunner(**base_kwargs, code=code)
    elif RUN_TYPE == RunType.FLOW:
        from bench.runtime.flow import FlowRunner

        runner = FlowRunner(**base_kwargs)
    elif RUN_TYPE == RunType.ACTION:
        from bench.runtime.flow import ACTION_RUNNER_BY_ACTION_TYPE

        assert isinstance(node, Action), f"expected Action, got {node!r}"
        runner_cls = ACTION_RUNNER_BY_ACTION_TYPE[node.type]
        runner = runner_cls(**base_kwargs)
    elif RUN_TYPE == RunType.PIPE:
        from bench.runtime.flow import PIPE_RUNNER_BY_PIPE_TYPE

        assert isinstance(node, Pipe), f"expected Pipe, got {node!r}"
        runner_cls = PIPE_RUNNER_BY_PIPE_TYPE[node.type]
        runner = runner_cls(**base_kwargs)
    else:
        assert_never(RUN_TYPE)

    return cast(Runner, runner)
