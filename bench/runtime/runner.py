import abc
import asyncio
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

from bench.language import Block, Step
from bench.language.code import Code
from bench.language.const import ObjectKind, RunStatus
from bench.language.field import TypeBase
from bench.language.flow import Pipe
from bench.language.interrupt import (
    Breakpoint,
    BreakpointScope,
    BreakpointSite,
    Interrupt,
    InterruptKind,
)
from bench.language.log import LogInfo
from bench.language.run import (
    Context,
    Run,
    RunAttempt,
    RunError,
    RunEvent,
    RunKind,
    RunnableNode,
    RunOptions,
    RunSpan,
)
from bench.language.value import CustomObject
from bench.runtime.core import BASE_RUN_OPTIONS_BY_KIND, RunImpossibleError
from bench.utils.uuidt import UUIDT

if TYPE_CHECKING:
    from bench.runtime.runtime import Runtime


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class Interrupted(Exception):  # noqa: N818
    """A Runner/Run is interrupted."""

    def __init__(self, runner: "Runner", run: Run, interrupt: Interrupt):
        self.runner = runner
        self.run = run
        self.interrupt = interrupt


RunnerHook = Callable[["Runner", Exception | None], None]


class Runner[N: RunnableNode = RunnableNode](abc.ABC):
    """
    A runner for a single Run (tracked Run or untracked RunSpan).
    Runners work similar to asyncio Tasks, making progress until terminated or stopped by an Interrupt.
    Once an Interrupt is handled, we try to run the Runner again - it may progress or raise another Interrupt.
    Interruptible Runners may be nested, and it's the responsibility of the Runners
     to ensure replay stability in all sub-Runners when resuming after an Interrupt.
    """

    __slots__ = (
        "attempts",
        "context",
        "error",
        "events",
        "id",
        "input_type",
        "inputs",
        "is_cancelled",
        "kind",
        "logs",
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
    )

    kind: ClassVar[RunKind]

    def __init__(
        self,
        *,
        runtime: "Runtime",
        node: N,
        track: bool,
        options: RunOptions,
        context: Context,
        parent: "Runner | None" = None,
        inputs: CustomObject | None = None,
        output_type: TypeBase | None = None,
        run: Run | None = None,
    ) -> None:
        self.id = run.id if run is not None else UUIDT()
        self.runtime = runtime
        self.node = node
        self.status = RunStatus.QUEUED
        self.options = options

        self.context = context
        self.inputs: CustomObject | None = inputs
        self.input_type = node.input_type
        self.outputs: CustomObject | None = None
        self.output_type = output_type or node.output_type
        self.error: RunError | None = None

        self.parent = parent or runtime.active_runner
        self.attempts: list[RunAttempt] = list(run.attempts) if run is not None else []
        self.logs: list[LogInfo] = []
        self.spans: list[RunSpan] = []
        self.events: list[RunEvent] = []
        self.runners: list[Runner] = []
        self.tracked_run = run
        self.task: asyncio.Task | None = None
        self.outer_task: asyncio.Task | None = None
        self.is_cancelled = False

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
                    parent=parent_run or self.session.package,
                    kind=self.kind,
                    block=node if isinstance(node, Block) else node.block,
                    step=node if isinstance(node, Step) else None,
                    pipe=node if isinstance(node, Pipe) else None,
                    options=options,
                    status=self.status,
                    inputs=self.inputs,
                    session=self.session,
                    _skip_validate_self=True,
                )
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
    def is_tracked(self) -> bool:
        return self.tracked_run is not None

    @property
    def is_nested(self) -> bool:
        return self.parent is not None

    @property
    def current_attempt(self) -> RunAttempt | None:
        return self.attempts[-1] if self.attempts else None

    @property
    def ancestors(self):
        parent = self
        while parent is not None:
            yield parent
            parent = parent.parent

    @property
    def interrupt(self) -> Interrupt | None:
        assert self.tracked_run is not None, f"{self!r} is not tracked"
        return self.tracked_run.interrupt

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

    def _get_or_create_interrupt(
        self, kind: InterruptKind, *, breakpoint: BreakpointSite | None = None
    ):
        """Gets an Interrupt in the current Runner of the given shape (or creates one)."""
        assert self.tracked_run is not None, f"{self!r} is not tracked"
        for interrupt in self.tracked_run.interrupts:
            if interrupt.kind == kind and interrupt.breakpoint_site == breakpoint:
                return interrupt
        interrupt = Interrupt.from_run(kind, self.tracked_run, breakpoint=breakpoint)
        self.session._create(interrupt)
        return interrupt

    @final
    def _trap_breakpoint(self, site: BreakpointSite, *alias_sites: BreakpointSite):
        """
        Yield/resume the given kind of breakpoint if set in this Runner.
        """
        if self.tracked_run is None:
            return  # can't break into untracked Run
        # handle applicable breakpoint (if any)
        if self._has_breakpoint_set(site, *alias_sites):
            interrupt = self._get_or_create_interrupt(InterruptKind.YIELD, breakpoint=site)
            if interrupt.is_closed:
                logger.trace(
                    "runtime.breakpoint.closed", runner=self, interrupt=interrupt, site=site
                )
                return  # breakpoint already handled
            else:
                logger.trace("runtime.breakpoint", runner=self, interrupt=interrupt, site=site)
                raise Interrupted(self, self.tracked_run, interrupt)

    @final
    def _trap_pause(self):
        """Yield/resume a pause the given Runner if required."""
        if self.tracked_run is None:
            return

    def cancel(self):
        self.is_cancelled = True
        if self.outer_task is not None:
            self.outer_task.cancel()
        if self.task is not None:
            self.task.cancel()

    @abc.abstractmethod
    async def run(self) -> None:
        """
        Run until the Run stops:
         - On completion, return normally.
         - On error, raise an exception.
         - On interrupt, raise Interrupted.
        """
        ...


def get_run_options(kind: RunKind, options: RunOptions | None):
    """Get the combined run options for a node."""
    base_options = BASE_RUN_OPTIONS_BY_KIND[kind]
    return base_options.override(options)


def make_run_from_node(
    node: "RunnableNode",
    *,
    inputs: Any | None = None,
    parent: "Run | None" = None,
) -> "Run":
    """Creates a Run from a runnable Node."""
    from bench.language import Block, Step
    from bench.language.value import coerce_custom_object

    if isinstance(node, Block):
        block = node
        step = None
        pipe = None
        kind = node.run_kind
        assert kind is not None, f"no run kind for {node!r}"
    elif isinstance(node, Step):
        step = node
        block = step.block
        pipe = None
        kind = RunKind.STEP
    elif isinstance(node, Pipe):
        pipe = node
        block = pipe.block
        step = None
        kind = RunKind.PIPE
    else:
        assert_never(node)

    options = get_run_options(kind, node.run_options)
    run = Run(
        parent=parent or node.package,
        kind=kind,
        block=block,
        step=step,
        pipe=pipe,
        options=options,
    )
    if inputs is None:
        inputs = {}
    if run.input_type is not None:
        inputs = coerce_custom_object(ObjectKind.INPUT, run.input_type, inputs)
        run.inputs = inputs
    return run


def restore_runner(runtime: "Runtime", run: Run) -> "Runner":
    """Make a Runner from a Run."""
    node = run.runnable
    if node is None:
        raise RunImpossibleError(f"no node for {run!r}")
    if run.inputs is None and run.input_type is not None:
        inputs = CustomObject.new(ObjectKind.INPUT, {}, run.input_type)
    else:
        inputs = run.inputs

    return make_runner(
        runtime,
        node,
        kind=run.kind,
        options=run.options,
        context=run.context,
        inputs=inputs,
        run=run,
        track=True,
    )


def make_runner(
    runtime: "Runtime",
    node: RunnableNode,
    track: bool,
    *,
    kind: RunKind | None = None,
    context: Context | None = None,
    inputs: Any | None = None,
    parent: "Run | None" = None,
    run: Run | None = None,
    **kwargs,
) -> "Runner":
    """Make a Runner from a runnable Node."""

    run_kind = kind or node.run_kind
    assert run_kind is not None, f"no run kind for {node!r}"
    options = get_run_options(run_kind, run.options if run is not None else node.run_options)
    base_kwargs: dict[str, Any] = {
        "runtime": runtime,
        "options": options,
        "context": context,
        "inputs": inputs,
        "run": run,
        "track": track,
        "node": node,
        "parent": parent,
        **kwargs,
    }
    if run_kind == RunKind.CODE:
        from bench.runtime.code import CodeFunctionRunner

        code = getattr(node, "code", None) or Code.empty()
        runner = CodeFunctionRunner(**base_kwargs, code=code)
    elif run_kind == RunKind.ACTION:
        from bench.runtime.action import ActionRunner

        runner = ActionRunner(**base_kwargs)
    elif run_kind == RunKind.FLOW:
        from bench.runtime.flow import FlowRunner

        runner = FlowRunner(**base_kwargs)
    elif run_kind == RunKind.STEP:
        from bench.runtime.flow import STEP_RUNNER_BY_STEP_TYPE, Step

        assert isinstance(node, Step), f"expected Step, got {node!r}"
        runner_cls = STEP_RUNNER_BY_STEP_TYPE[node.type]
        runner = runner_cls(**base_kwargs)
    elif run_kind == RunKind.PIPE:
        from bench.runtime.flow import PIPE_RUNNER_BY_PIPE_TYPE, Pipe

        assert isinstance(node, Pipe), f"expected Pipe, got {node!r}"
        runner_cls = PIPE_RUNNER_BY_PIPE_TYPE[node.type]
        runner = runner_cls(**base_kwargs)
    else:
        assert_never(run_kind)

    return cast(Runner, runner)
