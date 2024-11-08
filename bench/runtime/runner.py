import abc
import asyncio
from typing import TYPE_CHECKING, Any, ClassVar, Iterable, assert_never

import structlog
from opentelemetry import trace

from bench.language.code import Code
from bench.language.const import ObjectKind, RunStatus
from bench.language.field import TypeBase
from bench.language.flow import Pipe
from bench.language.log import LogInfo
from bench.language.run import (
    Context,
    Run,
    RunAttempt,
    RunError,
    RunEvent,
    RunKind,
    RunOptions,
    RunSpan,
)
from bench.language.value import CustomObject
from bench.runtime.core import RunImpossibleError
from bench.utils.uuidt import UUIDT

if TYPE_CHECKING:
    from bench.language import Block, Step
    from bench.runtime.runtime import Runtime


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class Runner:
    """A runner for a single Run (tracked Run or untracked RunSpan)."""

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
        "output_type",
        "outputs",
        "parent",
        "run",
        "runs",
        "runtime",
        "spans",
        "status",
        "task",
    )

    kind: ClassVar[RunKind]

    def __init__(
        self,
        *,
        runtime: "Runtime",
        node: "Block | Step | Pipe",
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
        self.runs: list[Runner] = []
        self.run = run
        self.task: asyncio.Task | None = None
        self.is_cancelled = False

        # nest active Runners/Runs
        if self.parent is not None:
            self.parent.runs.append(self)
        if track and run is None:
            if self.parent is not None:
                assert self.parent.run is not None, f"{self.parent!r} has no Run"
                parent_run = self.parent.run
            else:
                parent_run = None
            with tracer.start_as_current_span("runtime.create_run"):
                run = Run(
                    parent=parent_run or self.runtime.session.package,
                    kind=self.kind,
                    block=node if isinstance(node, Block) else node.block,
                    step=node if isinstance(node, Step) else None,
                    options=options,
                    status=self.status,
                    inputs=self.inputs,
                    session=self.runtime.session,
                    _skip_validate_self=True,
                )
                self.runtime.session._create(run)
            self.run = run

    def __str__(self):
        str_parts: list[str] = [
            self.status.bench_name,
            f"node={self.node!r}",
            f"options={self.options!r}",
        ]
        if self.attempts:
            str_parts.append(f"attempts={self.attempts}")
        if self.runs:
            str_parts.append(f"runs={len(self.runs)}")
        if self.run:
            str_parts.append(f"run={self.run}")
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
        for run in self.runs:
            yield from run.walk()

    @property
    def is_tracked(self) -> bool:
        return self.run is not None

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

    @abc.abstractmethod
    async def run_once(self) -> None:
        """Runs the runnable once."""
        raise NotImplementedError


def run_from_node(
    node: "Block | Step | Pipe",
    *,
    options: RunOptions | None = None,
    inputs: Any | None = None,
    parent: "Run | None" = None,
    **kwargs,
) -> "Run":
    """Creates a Run from a runnable Node."""
    from bench.language import Block, Step
    from bench.language.value import coerce_custom_object

    if options is None:
        options = RunOptions()

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

    run = Run(
        parent=parent or node.package,
        kind=kind,
        block=block,
        step=step,
        options=options,
        **kwargs,
    )
    if inputs is None:
        inputs = {}
    if run.input_type is not None:
        inputs = coerce_custom_object(ObjectKind.INPUT, run.input_type, inputs)
        run.inputs = inputs
        if kwargs:
            inputs.update(kwargs)
    return run


def runner_from_run(runtime: "Runtime", run: Run, *, track: bool) -> "Runner":
    """Make a Runner from a Run."""
    node = run.step or run.block
    if node is None:
        raise RunImpossibleError(f"no node for {run!r}")
    if run.inputs is None and run.input_type is not None:
        inputs = CustomObject.new(ObjectKind.INPUT, {}, run.input_type)
    else:
        inputs = run.inputs

    return runner_from_node(
        runtime,
        node,
        kind=run.kind,
        options=run.options,
        context=run.context,
        inputs=inputs,
        run=run,
        track=track,
    )


def runner_from_node(
    runtime: "Runtime",
    node: "Block | Step | Pipe",
    track: bool,
    *,
    kind: RunKind | None = None,
    options: RunOptions | None = None,
    context: Context | None = None,
    inputs: Any | None = None,
    parent: "Run | None" = None,
    run: Run | None = None,
    **kwargs,
) -> "Runner":
    """Make a Runner from a runnable Node."""

    if options is None:
        options = RunOptions()

    base_kwargs: dict[str, Any] = {
        "runtime": runtime,
        "options": options,
        "context": context,
        "inputs": inputs,
        "run": run,
        "track": track,
        "node": node,
        "parent": parent,
    }

    run_kind = kind or node.run_kind
    assert run_kind is not None, f"no run kind for {node!r}"
    if run_kind == RunKind.CODE:
        from bench.runtime.code import CodeFunctionRunner

        code = getattr(node, "code", None) or Code.empty()
        return CodeFunctionRunner(**base_kwargs, code=code)
    elif run_kind == RunKind.ACTION:
        from bench.runtime.action import ActionRunner

        return ActionRunner(**base_kwargs)
    elif run_kind == RunKind.FLOW:
        from bench.runtime.flow import FlowRunner

        return FlowRunner(**base_kwargs)
    elif run_kind == RunKind.STEP:
        from bench.runtime.flow import STEP_RUNNER_BY_STEP_TYPE

        assert isinstance(node, Step), f"expected Step, got {node!r}"
        runner_cls = STEP_RUNNER_BY_STEP_TYPE[node.type]
        return runner_cls(**base_kwargs)
    elif run_kind == RunKind.PIPE:
        from bench.runtime.flow import PIPE_RUNNER_BY_PIPE_TYPE

        assert isinstance(node, Pipe), f"expected Pipe, got {node!r}"
        runner_cls = PIPE_RUNNER_BY_PIPE_TYPE[node.type]
        return runner_cls(**base_kwargs)
    else:
        assert_never(run_kind)
