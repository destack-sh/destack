import abc
import asyncio
from datetime import timedelta
from typing import (
    TYPE_CHECKING,
    Any,
    Callable,
    ClassVar,
    Iterable,
    Sequence,
    assert_never,
    cast,
    final,
)

import structlog
from opentelemetry import trace

from bench.language import (
    Action,
    Block,
    Breakpoint,
    BreakpointScope,
    BreakpointSite,
    Code,
    Context,
    CustomObject,
    Interruption,
    InterruptionStatus,
    InterruptionType,
    LogInfo,
    Node,
    NodeMode,
    ObjectKind,
    Pipe,
    Resource,
    ResourceStatus,
    Run,
    RunAttempt,
    RunError,
    RunEvent,
    RunnableNode,
    RunOptions,
    RunSpan,
    RunStatus,
    RunType,
    TypeBase,
    TypeIn,
    TypeInfo,
    get_tracing_context,
)
from bench.utils.uuidt import UUIDT

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


RunnerHook = Callable[["Runner", BaseException | None], None]


class Runner[N: RunnableNode = RunnableNode](abc.ABC):
    """
    A runner for a single Run (tracked Run or untracked RunSpan).
    Runners work similar to asyncio Tasks, making progress until terminated or stopped by an Interruption.
    Once an Interruption is handled, we try to run the Runner again - it may progress or raise another Interruption.
    Interruption-capable Runners may be nested, and it's the responsibility of the Runners
     to ensure replay stability in all sub-Runners when resuming after an Interruption.
    """

    __slots__ = (
        "attempts",
        "context",
        "error",
        "events",
        "id",
        "input_type",
        "inputs",
        "is_stopped",
        "kind",
        "logs",
        "mode",
        "node",
        "options",
        "outer_task",
        "output_type",
        "outputs",
        "parent",
        "runners",
        "runs",
        "runtime",
        "spans",
        "status",
        "task",
        "tracked_run",
        "variable_type",
        "variables",
    )

    kind: ClassVar[RunType]

    def __init__(
        self,
        *,
        runtime: "Runtime",
        node: N,
        track: bool,
        options: RunOptions,
        context: Context,
        parent: "Runner | None" = None,
        variables: CustomObject | None = None,
        inputs: CustomObject | None = None,
        output_type: TypeBase | None = None,
        mode: NodeMode | None = None,
        run: Run | None = None,
    ) -> None:
        self.id = run.id if run is not None else UUIDT()
        self.runtime = runtime
        self.node = node
        self.status = RunStatus.QUEUED
        self.options = options

        self.context = context
        self.variables = variables
        self.variable_type = node.variable_type
        self.inputs: CustomObject | None = inputs
        self.input_type = node.input_type
        self.outputs: CustomObject | None = None
        self.output_type = output_type or node.output_type
        self.error: RunError | None = None
        assert (
            self.variable_type is None or self.variables is not None
        ), f"{self!r} has no variables"
        assert self.input_type is None or self.inputs is not None, f"{self!r} has no inputs"

        self.parent = parent or runtime.active_runner
        if run is not None:
            self.attempts = list(run.attempts)
            self.logs = list(run.logs)
            self.spans = list(run.spans)
            self.events = list(run.events)
        else:
            self.attempts: list[RunAttempt] = []
            self.logs: list[LogInfo] = []
            self.spans: list[RunSpan] = []
            self.events: list[RunEvent] = []
        self.runners: list[Runner] = []
        self.tracked_run = run
        self.task: asyncio.Task | None = None
        self.outer_task: asyncio.Task | None = None
        self.is_stopped = False

        # determine mode
        if mode is not None:
            self.mode = mode
        elif self.parent is not None:
            self.mode = self.parent.mode
        elif run is not None:
            self.mode = run.mode
        else:
            self.mode = NodeMode.PRODUCTION

        # nest active Runners/Runs
        if self.parent is not None:
            self.parent.runners.append(self)
        if track and run is None:
            if self.parent is not None:
                assert self.parent.tracked_run is not None, f"{self.parent!r} has no Run"
                parent_run = self.parent.tracked_run
            else:
                parent_run = None
            with tracer.start_as_current_span("runtime.create_run"):
                run = Run(
                    parent=parent_run or self.runtime.bench,
                    type=self.kind,
                    block=node if isinstance(node, Block) else node.block,
                    action=node if isinstance(node, Action) else None,
                    pipe=node if isinstance(node, Pipe) else None,
                    options=options,
                    status=self.status,
                    mode=self.mode,
                    inputs=self.inputs,
                    session=self.session,
                    _skip_validate_self=True,
                )
                self.id = run.id
                self.session._create(run)
            self.tracked_run = run

    def __str__(self):
        str_parts: list[str] = [
            self.status.bench_name,
            f"node={self.node!r}",
            f"options={self.options!r}",
        ]
        if self.attempts:
            str_parts.append(f"attempts={self.attempts}")
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
    def is_tracked(self) -> bool:
        return self.tracked_run is not None

    @property
    def is_root(self) -> bool:
        return self.parent is None

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
    def current_attempt(self) -> RunAttempt | None:
        return self.attempts[-1] if self.attempts else None

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
        self, resource_type: TypeInfo | TypeIn | type[R]
    ) -> R | None:
        """Finds a Resource in the current context of a Runner."""
        return self.runtime._get_resource(self, resource_type)

    def _get_ready_resource_or_error[R: Resource = Resource](
        self, resource_type: TypeInfo | TypeIn | type[R]
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
        Run until the Run stops:
         - On completion, return normally.
         - On error, raise an exception.
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


def get_run_options(kind: RunType, options: RunOptions | None):
    """Get the combined run options for a node."""
    base_options = BASE_RUN_OPTIONS_BY_KIND[kind]
    return base_options.override(options)


def make_run_from_node(
    node: "RunnableNode",
    *,
    variables: Any | None = None,
    inputs: Any | None = None,
    mode: NodeMode | None = None,
    parent: "Run | None" = None,
) -> "Run":
    """Creates a Run from a runnable Node."""
    from bench.language import Action, Block, coerce_custom_object_scalar

    # context
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
    options = get_run_options(kind, node.run_options)
    run = Run(
        parent=parent or node.bench,
        type=kind,
        block=block,
        action=action,
        pipe=pipe,
        options=options,
        mode=mode or get_tracing_context(),
    )

    # variables
    if variables is None:
        variables = {}
    if run.variable_type is not None:
        variables = coerce_custom_object_scalar(ObjectKind.VARIABLE, variables, run.variable_type)
        if type(node) is Action and node.inputs_packed is not None:
            variables.set_default(node.inputs, _skip_validate=True)
        run.variables = variables

    # inputs
    if inputs is None:
        inputs = {}
    if run.input_type is not None:
        inputs = coerce_custom_object_scalar(ObjectKind.INPUT, inputs, run.input_type)
        if type(node) is Action and node.inputs_packed is not None:
            inputs.set_default(node.inputs, _skip_validate=True)
        run.inputs = inputs

    return run


def restore_runner(runtime: "Runtime", run: Run) -> "Runner":
    """Make a Runner from a Run."""
    node = run.runnable
    if node is None:
        raise RunImpossibleError(f"no node for {run!r}")

    # variables
    if run.variables is None and run.variable_type is not None:
        variables = CustomObject.new(
            ObjectKind.VARIABLE, {}, typ=run.variable_type, supergraph=runtime.session._supergraph
        )
    else:
        variables = run.variables

    # inputs
    if run.inputs is None and run.input_type is not None:
        inputs = CustomObject.new(
            ObjectKind.INPUT, {}, typ=run.input_type, supergraph=runtime.session._supergraph
        )
    else:
        inputs = run.inputs

    return make_runner(
        runtime,
        node,
        kind=run.type,
        context=run.context,
        variables=variables,
        inputs=inputs,
        run=run,
        track=True,
    )


def make_runner(
    runtime: "Runtime",
    node: RunnableNode,
    track: bool,
    *,
    kind: RunType | None = None,
    context: Context | None = None,
    variables: CustomObject | None = None,
    inputs: Any | None = None,
    output_type: TypeBase | None = None,
    parent: "Runner | None" = None,
    run: Run | None = None,
) -> "Runner":
    """Make a Runner from a runnable Node."""

    RUN_TYPE = kind or node.run_type
    assert RUN_TYPE is not None, f"no run kind for {node!r}"

    # variables
    variable_type = node.variable_type
    if variables is None and variable_type is not None:
        variables = CustomObject.new(
            ObjectKind.VARIABLE, {}, typ=variable_type, supergraph=runtime.session._supergraph
        )

    # inputs
    input_type = node.input_type
    if inputs is None and input_type is not None:
        inputs = CustomObject.new(
            ObjectKind.INPUT, {}, typ=input_type, supergraph=runtime.session._supergraph
        )

    # build runner
    options = get_run_options(RUN_TYPE, run.options if run is not None else node.run_options)
    base_kwargs: dict[str, Any] = {
        "runtime": runtime,
        "options": options,
        "context": context,
        "inputs": inputs,
        "variables": variables,
        "output_type": output_type,
        "run": run,
        "track": track,
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
