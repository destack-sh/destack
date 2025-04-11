import abc
import asyncio
from dataclasses import dataclass
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
    COMMUNICATION_NODE_TYPES,
    RUNTIME_NODE_TYPES,
    Action,
    Agent,
    Breakpoint,
    BreakpointScope,
    BreakpointSite,
    Code,
    CustomObject,
    Error,
    Flow,
    GraphCapture,
    Interruption,
    InterruptionStatus,
    InterruptionType,
    IsType,
    Kit,
    Link,
    Log,
    Membership,
    Message,
    Node,
    NodeGraph,
    NodeMode,
    NodeType,
    Package,
    ProcessStatus,
    Run,
    Runnable,
    RunOptions,
    RunType,
    Session,
    Span,
    SpanType,
    Task,
    Thread,
    Trigger,
    active_session,
)

from .error import InterruptionCancelledError, RunImpossibleError
from .options import BASE_RUN_OPTIONS_BY_KIND
from .thread import ThreadHandle

if TYPE_CHECKING:
    from .runtime import Runtime

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class Interrupted(Exception):  # noqa: N818
    """A Runner/Run is interrupted."""

    def __init__(self, runner: "Runner", span: Run | Span, interruption: Interruption):
        self.runner = runner
        self.span = span
        self.interruption = interruption


RunIn = Run | Span | SpanType | Literal["track"]


@dataclass(slots=True)
class RunnerFailedEvent:
    """Runner failed."""

    runner: "Runner"
    error: Error


@dataclass(slots=True)
class RunnerAbortedEvent:
    """Runner aborted."""

    runner: "Runner"


@dataclass(slots=True)
class RunnerCancelledEvent:
    """Runner cancelled."""

    runner: "Runner"


@dataclass(slots=True)
class RunnerOutputEvent:
    """Runner now has an output (may not be complete yet)."""

    runner: "Runner"
    outputs: CustomObject | None


@dataclass(slots=True)
class RunnerInterruptedEvent:
    """Runner was interrupted."""

    runner: "Runner"
    interruption: Interruption


@dataclass(slots=True)
class RunnerCompletedEvent:
    """Runner completed."""

    runner: "Runner"
    outputs: CustomObject | None


RunnerEvent = (
    RunnerAbortedEvent
    | RunnerCancelledEvent
    | RunnerFailedEvent
    | RunnerOutputEvent
    | RunnerInterruptedEvent
    | RunnerCompletedEvent
)


class Runner[N: Runnable = Runnable](abc.ABC):
    """
    A Runner to run a Run/Span (every Run has one Runner, some Spans have one).
    Runners work similar to asyncio Tasks, making progress until terminated or stopped by an Interruption.
    After an Interruption is handled, we try to run the Runner again - it may progress or raise another Interruption.
    Interruption-capable Runners may be nested, and it's the responsibility of the Runners
     to ensure replay stability in all sub-Runners when resuming after an Interruption.
    """

    __slots__ = (
        "capture",
        "connection",
        "error",
        "hooks",
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
    )

    runner_type: ClassVar[RunType]

    def __init__(
        self,
        *,
        runtime: "Runtime",
        node: N,
        options: RunOptions,
        run: RunIn,
        parent: "Runner[Any] | None" = None,
        inputs: CustomObject | None = None,
        outputs: IsType | CustomObject | None = None,
        agent: "Agent | None" = None,
    ) -> None:
        self.runtime = runtime
        self.node = node
        self.status = ProcessStatus.QUEUED
        self.options = options
        self.parent = parent or runtime.active_runner
        self.runners: list[Runner] = []
        self.task: asyncio.Task | None = None
        self.outer_task: asyncio.Task | None = None
        self.is_stopped = False

        # resources/inputs/outputs
        self.inputs: CustomObject | None = inputs
        self.input_type = node.input_type
        if isinstance(outputs, IsType):
            self.outputs: CustomObject | None = None
            self.output_type = outputs or node.output_type
        elif isinstance(outputs, CustomObject):
            self.outputs = outputs
            self.output_type = outputs._type
        else:
            self.outputs = None
            self.output_type = node.output_type
        self.error: Error | None = None

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
            self.mode = NodeMode.MAIN

        # track in Run/Span
        self.tracked_run: Run | None
        self.tracked_span: Span | None
        self.tracked: Span | Run
        if isinstance(run, Run) or run == "track":
            # Runner = Run
            if run == "track":
                parent_node = parent_run or node.package
                assert parent_node is not None, f"no parent for {node!r}"
                tracked_run, _ = create_run(
                    node=node,
                    parent=parent_node,
                    options=options,
                    status=self.status,
                    mode=self.mode,
                    inputs=self.inputs,
                    agent=agent,
                    session=self.session,
                )
            else:
                tracked_run = cast(Run, run)
            self.id = tracked_run.id
            # Run and Runner *must* share the same objects (so we can apply computed values)
            # get objects from Run (Node may copy them if they're from a different parent,
            #  like when we re-use a Flow's inputs for the StartAction.inputs)
            self.inputs = tracked_run.inputs
            self.options = tracked_run.options
            self.tracked_run = tracked_run
            self.tracked_span = None
            self.tracked = tracked_run
            assert (
                tracked_run.inputs is self.inputs
            ), f"{tracked_run!r} has other inputs than {self!r}"
            assert (
                tracked_run.options is self.options
            ), f"{tracked_run!r} has other options than {self!r}"
        else:
            # Runner = Span
            if type(run) is SpanType:
                assert parent_run is not None, f"{self!r} has no parent Run"
                tracked_span = Span(
                    parent=parent_run,
                    type=run,
                    status=self.status,
                    mode=self.mode,
                    agent=agent,
                    _skip_validate_self=True,
                )
                parent_run._copy_context_to(tracked_span)
                self.session._create(tracked_span)
            else:
                tracked_span = cast(Span, run)
            self.tracked_run = None
            self.tracked_span = tracked_span
            self.tracked = self.tracked_span
        self.id = self.tracked.id

        # runtime
        self.capture: GraphCapture | None = None  # for root runner
        self.hooks: list[Callable[[RunnerEvent], None]] = []
        if self.id in self.runtime._runners_by_id:
            raise RuntimeError(f"runner {self!r} already exists in {self.runtime!r}")
        self.runtime._runners_by_id[self.id] = self
        if node.id not in self.runtime._runners_by_runnable_id:
            self.runtime._runners_by_runnable_id[node.id] = []
        self.runtime._runners_by_runnable_id[node.id].append(self)

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
    def thread(self) -> ThreadHandle:
        run = self.tracked_run if self.tracked_run is not None else self.closest_tracked_run
        assert run is not None, f"{self!r} has no tracked Run"
        assert run.thread_ptr is not None, f"{run!r} has no Thread"
        thread = self.runtime.get_thread(run.thread_ptr.id)
        assert thread is not None, f"{run!r} has no Thread"
        return thread

    @property
    def agent(self) -> Agent | None:
        return self.tracked.agent

    @property
    def is_active(self) -> bool:
        return self.outer_task is not None and not self.outer_task.done()

    @property
    def is_root(self) -> bool:
        return self.parent is None

    @property
    def root(self) -> "Runner":
        root = self
        while root.parent is not None:
            root = root.parent
        return root

    @property
    def closest_tracked_run(self) -> Run | None:
        runner = self
        while runner is not None:
            if runner.tracked_run is not None:
                return runner.tracked_run
            runner = runner.parent
        return None

    def closest_runner_like[T: Runner](self, runner_cls: type[T]) -> T:
        runner = self
        while runner is not None:
            if isinstance(runner, runner_cls):
                return cast(T, runner)
            runner = runner.parent
        raise RuntimeError(f"no {runner_cls.__name__} ancestor for {self!r}")

    @property
    def should_pause(self) -> bool:
        if self.tracked_run is None:
            return False
        runner = self
        while runner is not None:
            if runner.tracked_run is not None and runner.tracked_run.should_pause:
                return True
            runner = runner.parent
        return False

    @property
    def attempts(self) -> Sequence[Span]:
        if self.tracked_run is None:
            return ()
        return tuple(span for span in self.tracked_run.spans if span.type == SpanType.ATTEMPT)

    @property
    def current_attempt(self) -> Span | None:
        if self.tracked_run is None:
            return None
        for span in reversed(self.tracked_run.spans):
            if span.type == SpanType.ATTEMPT:
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
        """Yields all ancestor Runners (excluding self)."""
        parent = self.parent
        while parent is not None:
            yield parent
            parent = parent.parent

    def get_runs(self, runnable: Runnable) -> list[Run]:
        """Find all Runs of a Node in this Runner."""
        root_runner = self.root
        root_run = root_runner.tracked_run
        if root_run is None:
            return []
        return root_run.get_runs(runnable, recursive=True)

    def get_latest_run(self, runnable: Runnable) -> Run | None:
        """Find the latest Run of a Node in this Runner tree."""
        matching_runs = self.get_runs(runnable)
        return matching_runs[0] if matching_runs else None

    def get_runners(self, runnable: Runnable) -> list["Runner"]:
        """Find all Runners of a Node in this Runner tree."""
        matching_runs = self.get_runs(runnable)
        runners: list[Runner] = []
        for run in matching_runs:
            runner = self.runtime._runners_by_id.get(run.id)
            if runner is None:
                raise RuntimeError(f"missing runner for {run!r}")
            runners.append(runner)
        return runners

    def get_latest_runner(self, runnable: Runnable) -> "Runner | None":
        """Find the latest Runner of a Node in this Runner tree."""
        matching_runners = self.get_runners(runnable)
        return matching_runners[0] if matching_runners else None

    #
    # Hooks
    #

    def on_event(self, handler: Callable[[RunnerEvent], None], upsert: bool = False):
        """Subscribe to a Runner event."""
        if upsert:
            if handler not in self.hooks:
                self.hooks.append(handler)
        else:
            if handler in self.hooks:
                raise RuntimeError(f"handler {handler!r} already subscribed to {self!r}")
            self.hooks.append(handler)

    def fire_event(self, event: RunnerEvent):
        """Fire a Runner event."""
        for handler in self.hooks:
            handler(event)

    #
    # Interruptions
    #

    @property
    def interruption(self) -> Interruption | None:
        return self.tracked.interruption

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
        breakpoint_site: BreakpointSite | None = None,
    ):
        """Gets an Interrupt in the current Runner of the given shape (or creates one)."""
        run = self.closest_tracked_run
        assert run is not None, f"{self!r} is not tracked"
        for interruption in run.interruptions:
            interruption = cast(Interruption, interruption)
            if (
                interruption.type == kind
                and (interruption.span_ptr is None or interruption.span_ptr.id == self.id)
                and interruption.breakpoint_site == breakpoint_site
            ):
                return interruption
        interruption = Interruption.from_run(
            kind, run, span=self.tracked_span, breakpoint_site=breakpoint_site
        )
        self.session._create(interruption)
        return interruption

    def _trap_interruption(
        self,
        kind: InterruptionType,
        *,
        breakpoint_site: BreakpointSite | None = None,
    ):
        """Yield/resume an Interrupt of the given kind if set in this Runner."""
        interruption = self._get_or_create_interruption(kind, breakpoint_site=breakpoint_site)
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
            raise Interrupted(self, self.tracked, interruption)

    @final
    def _trap_breakpoint(self, site: BreakpointSite, *alias_sites: BreakpointSite):
        """Yield/resume the given kind of breakpoint if set in this Runner."""
        # handle applicable breakpoint (if any)
        if self._has_breakpoint_set(site, *alias_sites):
            self._trap_interruption(InterruptionType.YIELD, breakpoint_site=site)

    @final
    def _trap_pause(self):
        """Yield/resume a pause the given Runner if required."""
        if self.should_pause:
            self._trap_interruption(InterruptionType.PAUSE)

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
            self.tracked_run._mark_terminated()
            self.status = self.tracked_run.status

    def close(self, recursive: bool = True):
        """Close this Runner/Run."""
        if self.capture is not None:
            self.capture.close_and_detach()
        self.runtime._runners_by_id.pop(self.id, None)
        runnable_id = self.node.id
        if runnable_id in self.runtime._runners_by_runnable_id:
            self.runtime._runners_by_runnable_id[runnable_id].remove(self)
            if len(self.runtime._runners_by_runnable_id[runnable_id]) == 0:
                self.runtime._runners_by_runnable_id.pop(runnable_id, None)
        if recursive:
            for runner in self.runners:
                runner.close(recursive=True)


RUN_TYPE_BY_NODE_TYPE: dict[NodeType, RunType] = {
    NodeType.ACTION: RunType.ACTION,
    NodeType.FLOW: RunType.FLOW,
    NodeType.LINK: RunType.LINK,
}


def create_run(
    node: "Runnable",
    *,
    parent: "Package | Thread | Run | Agent",
    inputs: Any | None = None,
    options: RunOptions | None = None,
    mode: NodeMode | None = None,
    status: ProcessStatus | None = None,
    thread: "Thread | None" = None,
    target: "Message | Task | None" = None,
    trigger: "Trigger | None" = None,
    agent: "Agent | None" = None,
    session: "Session | None" = None,
    graph: NodeGraph | None = None,
) -> tuple["Run", "Thread"]:
    """
    Makes a Run from a runnable Node without adding it to the session.
    If there is no Thread at the root, we create one (non-Root Runs must have a Thread).
    """
    from bench.language import coerce_custom_object_scalar

    if session is None:
        session = active_session()

    # context
    flow: Flow | None = None
    kit: Kit | None = None
    action: Action | None = None
    link: Link | None = None
    typ: RunType | None = None
    if isinstance(node, Agent):
        typ = RunType.AGENT
        agent = node
    elif isinstance(node, Flow):
        typ = RunType.FLOW
        flow = node
    elif isinstance(node, Action):
        typ = RunType.ACTION
        action = node
        flow = node.flow
        kit = node.kit
    elif isinstance(node, Link):
        typ = RunType.LINK
        link = node
        flow = node.flow
    else:
        assert_never(node)
    if flow is None and isinstance(parent, Run):
        # inherit flow from parent if unset
        flow = parent.flow

    # create thread
    if isinstance(parent, Package):
        graph = NodeGraph(
            scope=parent._graph.scope,
            node_types=RUNTIME_NODE_TYPES | COMMUNICATION_NODE_TYPES,
            supergraph=session._supergraph,
        )
        if mode is None:
            mode = session.active_mode
        now = session._oracle.utc()
        thread = Thread(
            parent=parent,
            mode=mode,
            _graph=graph,
            started_at=now,
            active_at=now,
            status=ProcessStatus.IDLE,
        )
        session._create(thread)
        parent = thread
    elif isinstance(parent, Thread):
        thread = parent
        if mode is None:
            mode = parent.mode
        graph = parent._graph
    elif isinstance(parent, Run):
        thread = parent.thread
        if mode is None:
            mode = parent.mode
        graph = parent._graph
    elif isinstance(parent, Agent):
        if isinstance((grandparent := parent.parent), Thread):
            thread = grandparent
        else:
            raise RuntimeError(f"expected {parent!r} to be in a Thread")
        if mode is None:
            mode = parent.mode
        graph = parent._graph
    else:
        assert_never(parent)
    if thread is None:
        raise RuntimeError(f"no Thread for {node!r}")

    # create agent (and auto-instance)
    if agent is None and flow is not None:
        agent = flow.default_agent
    if agent is None:
        from bench.builtin import BenchAgent

        agent = BenchAgent
    if agent is not None and not agent.is_instance:
        agent = agent.instance()
        thread.agents.append(agent)
        membership = Membership.new(agent)
        thread.memberships.append(membership)

    # build run
    run = Run(
        parent=parent,
        type=typ,
        flow=flow,
        kit=kit,
        action=action,
        link=link,
        mode=mode,
        thread=thread,
        target=target,
        trigger=trigger,
        agent=agent,
        status=status or ProcessStatus.CREATED,
        _graph=graph,
        _skip_validate_self=True,
    )

    # inputs
    if inputs is None:
        inputs = {}
    if (input_type := run.input_type) is not None:
        inputs = coerce_custom_object_scalar(inputs, input_type)
        run.inputs = inputs

    # options
    if options is None:
        if isinstance(run_options := getattr(node, "options", None), RunOptions):
            options = run_options.clone()
            options.set_default(BASE_RUN_OPTIONS_BY_KIND[typ], copy=False)
        else:
            options = BASE_RUN_OPTIONS_BY_KIND[typ].clone()
    else:
        options.set_default(BASE_RUN_OPTIONS_BY_KIND[typ], copy=False)
    run.options = options

    session._create(run)

    return run, thread


def restore_runner(runtime: "Runtime", run: Run) -> "Runner":
    """Make a Runner from a Run."""
    node = run.runnable
    if node is None:
        raise RunImpossibleError(f"no node for {run!r}")

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
        inputs=inputs,
    )


def make_runner(
    runtime: "Runtime",
    node: Runnable,
    run: RunIn,
    *,
    type: RunType | None = None,
    inputs: Any | None = None,
    outputs: IsType | CustomObject | None = None,
    options: RunOptions | None = None,
    agent: "Agent | None" = None,
    parent: "Runner[Any] | None" = None,
) -> "Runner":
    """Make a Runner from a runnable Node."""

    RUN_TYPE = type or RUN_TYPE_BY_NODE_TYPE.get(node.metatype)
    assert RUN_TYPE is not None, f"no run kind for {node!r}"

    # inputs
    input_type = node.input_type
    if inputs is None and input_type is not None:
        inputs = CustomObject.new({}, typ=input_type, supergraph=runtime.session._supergraph)

    # options
    if options is None and isinstance(run_options := getattr(node, "options", None), RunOptions):
        options = run_options.clone()
    if options is None:
        options = BASE_RUN_OPTIONS_BY_KIND[RUN_TYPE].clone()
    else:
        options.set_default(BASE_RUN_OPTIONS_BY_KIND[RUN_TYPE], copy=False)

    # build runner
    base_kwargs: dict[str, Any] = {
        "runtime": runtime,
        "options": options,
        "inputs": inputs,
        "outputs": outputs,
        "run": run,
        "node": node,
        "parent": parent,
        "agent": agent,
    }

    # map to runner
    if RUN_TYPE == RunType.CODE:
        from bench.runtime.code import CodeFunctionRunner

        code = getattr(node, "code", None) or Code.empty()
        runner = CodeFunctionRunner(**base_kwargs, code=code)
    elif RUN_TYPE == RunType.AGENT:
        from bench.runtime.agent import AgentRunner

        runner = AgentRunner(**base_kwargs)
    elif RUN_TYPE == RunType.FLOW:
        from bench.runtime.flow import FlowRunner

        runner = FlowRunner(**base_kwargs)
    elif RUN_TYPE == RunType.ACTION:
        from bench.runtime.flow import ACTION_RUNNER_BY_ACTION_TYPE

        assert isinstance(node, Action), f"expected Action, got {node!r}"
        runner_cls = ACTION_RUNNER_BY_ACTION_TYPE[node.type]
        runner = runner_cls(**base_kwargs)
    elif RUN_TYPE == RunType.LINK:
        from bench.runtime.flow import LINK_RUNNER_BY_LINK_TYPE

        assert isinstance(node, Link), f"expected Link, got {node!r}"
        runner_cls = LINK_RUNNER_BY_LINK_TYPE[node.type]
        runner = runner_cls(**base_kwargs)
    else:
        assert_never(RUN_TYPE)

    return cast(Runner, runner)
