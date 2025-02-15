from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Any, Optional, Sequence, Union, cast
from uuid import UUID

from bench.language.core import (
    TERMINAL_RUN_STATUSES,
    BuiltinEnum,
    CustomObject,
    EnumType,
    FieldType,
    HasNodeBase,
    LocalNodeList,
    Node,
    NodeType,
    RunSpanType,
    RunStatus,
    RuntimeNode,
    RunType,
    Struct,
    StructType,
    Text,
    TypeConstraintIn,
    enum_,
    p_internal,
    p_node_ancestor,
    p_node_children,
    p_node_parent,
    p_regular,
    p_system,
    p_value_packed,
    p_value_runtime,
    struct_,
    timed_node_,
)
from bench.language.core.node import IndexIn
from bench.pb2 import AnyNodeData, NodeReferenceData, RunData
from bench.utils.tenacity import RetryOptions

if TYPE_CHECKING:
    from bench.language import (
        Action,
        AudioOptions,
        Bench,
        Breakpoint,
        Code,
        CustomObject,
        Error,
        ErrorType,
        Flow,
        ImageOptions,
        Interruption,
        Log,
        ModelDeveloper,
        ModelFamily,
        ModelType,
        NodeReference,
        Page,
        Pipe,
        RunnableNode,
        RunPlan,
        RunSpan,
        TextOptions,
        Trigger,
        TypeBase,
        VideoOptions,
    )


# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.CACHE_MODE)
class CacheMode(BuiltinEnum):
    """How to handle caching."""

    NEVER = 1
    ALWAYS = 2


@struct_(StructType.RUN_OPTIONS)
class RunOptions(Struct):
    """
    Options for running something.
    """

    # NOTE :Incomplete: some RunOptions don't do anything yet (e.g., max_concurrency, cache, ...)

    # general
    max_attempts: Optional[int] = p_regular(
        30, constraint=TypeConstraintIn(min_value=-1), description="Maximum retry attempts per Run"
    )
    max_concurrency: Optional[int] = p_regular(
        31,
        constraint=TypeConstraintIn(min_value=0),
        description="Maximum concurrent Runs",
    )
    max_runs: Optional[int] = p_regular(
        32,
        constraint=TypeConstraintIn(min_value=0),
        description="Maximum number of Runs of this runnable",
    )
    timeout: Optional[timedelta] = p_regular(35)

    # retry
    retry_interval: Optional[timedelta] = p_regular(40)
    backoff: Optional[float] = p_regular(41, constraint=TypeConstraintIn(min_value=1))
    max_retry_interval: Optional[timedelta] = p_regular(42)
    retry_on: list["ErrorType"] = p_regular(44, array=True)

    # context
    # ...

    # control
    breakpoints: list["Breakpoint"] = p_regular(60, array=True, struct=StructType.BREAKPOINT)

    # cache
    cache_mode: Optional["CacheMode"] = p_regular(70)
    cache_retention: Optional[timedelta] = p_regular(71)

    # model
    model_developer: Optional["ModelDeveloper"] = p_regular(80)
    model_family: Optional["ModelFamily"] = p_regular(81)
    model_type: Optional["ModelType"] = p_regular(82)
    text_options: Optional["TextOptions"] = p_regular(
        83, require=False, array=False, struct=StructType.TEXT_OPTIONS
    )
    audio_options: Optional["AudioOptions"] = p_regular(
        84, require=False, array=False, struct=StructType.AUDIO_OPTIONS
    )
    image_options: Optional["ImageOptions"] = p_regular(
        85, require=False, array=False, struct=StructType.IMAGE_OPTIONS
    )
    video_options: Optional["VideoOptions"] = p_regular(
        86, require=False, array=False, struct=StructType.VIDEO_OPTIONS
    )

    def to_retry(self) -> RetryOptions:
        """Turns the options into our RetryOptions."""
        return RetryOptions(
            max_attempts=self.max_attempts or 1,
            retry_interval=self.retry_interval.total_seconds() if self.retry_interval else 1,
            backoff=self.backoff or 2,
            max_retry_interval=self.max_retry_interval.total_seconds()
            if self.max_retry_interval
            else 30,
            # NOTE: retry_on is handled separately in runtime because we need the specific ErrorType
        )


@timed_node_(NodeType.RUN, index=((IndexIn(columns=("trigger_id", "trigger_key"))),))
class Run(RuntimeNode[RunData], HasNodeBase):
    """
    Run something somewhere, somehow.
    """

    # meta
    parent: Union["Bench", "Run", None] = p_node_parent(4, NodeType.BENCH, NodeType.RUN)
    type: RunType = p_system(30)
    root: "Run | None" = p_node_ancestor(
        31, NodeType.RUN, require=False, store=True, wire=True, is_bench_implicit=True
    )
    if TYPE_CHECKING:
        root_ptr: Optional[NodeReference] = None
        root_id: Optional[UUID] = None
    # owned_by?
    incoming: list["Run"] = p_internal(
        35, require=False, array=True, references=NodeType.RUN, same_bench=True
    )
    options: "RunOptions" = p_internal(39, require=True, array=False, struct=StructType.RUN_OPTIONS)

    # status
    status: RunStatus = p_internal(40, default=RunStatus.SCHEDULED)
    attempt: int | None = p_internal(41)
    duration: Optional[timedelta] = p_internal(
        42,
        default=None,
        description="Duration from first attempt start to last attempt termination.",
    )
    scheduled_at: Optional[datetime] = p_system(43, default=None)
    started_at: Optional[datetime] = p_internal(44, default=None)
    stopped_at: Optional[datetime] = p_internal(45, default=None)
    interrupted_at: Optional[datetime] = p_internal(46, default=None)
    paused_at: Optional[datetime] = p_internal(47, default=None)
    resumed_at: Optional[datetime] = p_internal(48, default=None)
    terminated_at: Optional[datetime] = p_internal(49, default=None)
    error: Optional["Error"] = p_internal(
        50, default=None, require=False, array=False, struct=StructType.ERROR
    )
    interruption: Optional["Interruption"] = p_internal(
        51, require=False, array=False, references=NodeType.INTERRUPTION, same_bench=True
    )

    # flow
    page: "Page" = p_internal(60, require=True, array=False, references=NodeType.PAGE)
    flow: Optional["Flow"] = p_internal(
        61, require=False, array=False, references=NodeType.FLOW, same_bench=True
    )
    action: Optional["Action"] = p_internal(
        62, require=False, array=False, references=NodeType.ACTION, same_bench=True
    )
    pipe: Optional["Pipe"] = p_internal(
        63, require=False, array=False, references=NodeType.PIPE, same_bench=True
    )
    plan: Optional["RunPlan"] = p_internal(
        64, require=False, array=False, references=NodeType.RUN_PLAN, same_bench=True
    )
    plan_step: int | None = p_internal(65)
    trigger: Optional["Trigger"] = p_regular(
        66, require=False, array=False, references=NodeType.TRIGGER, same_bench=True
    )
    trigger_key: Optional[str] = p_internal(67, require=False, default=None)
    if TYPE_CHECKING:
        page_ptr: Optional[NodeReference] = None
        page_id: Optional[UUID] = None
        page_ck: Optional[UUID] = None
        flow_ptr: Optional[NodeReference] = None
        flow_id: Optional[UUID] = None
        flow_ck: Optional[UUID] = None
        action_ptr: Optional[NodeReference] = None
        action_id: Optional[UUID] = None
        action_ck: Optional[UUID] = None
        pipe_ptr: Optional[NodeReference] = None
        pipe_id: Optional[UUID] = None
        pipe_ck: Optional[UUID] = None
        incoming_ptr: tuple["NodeReference", ...] = ()
        trigger_ptr: Optional[NodeReference] = None
        trigger_id: Optional[UUID] = None

    # content
    variables_packed: Any = p_value_packed(70)
    variables: "CustomObject | None" = p_value_runtime(
        70, type=FieldType.VARIABLE, typ=lambda self: cast("Run", self).variable_type
    )
    inputs_packed: Any = p_value_packed(71)
    inputs: "CustomObject | None" = p_value_runtime(
        71, type=FieldType.INPUT, typ=lambda self: cast("Run", self).input_type
    )
    outputs_packed: Any = p_value_packed(72)
    outputs: "CustomObject | None" = p_value_runtime(
        72, type=FieldType.OUTPUT, typ=lambda self: cast("Run", self).output_type
    )
    title: str | None = p_regular(74, default=None)
    text: Optional["Text"] = p_regular(75, default=None, struct=StructType.TEXT)
    code: Optional["Code"] = p_regular(76, default=None, struct=StructType.CODE)

    # ...HasContext[90-99]

    runs: LocalNodeList["Run"] = p_node_children(NodeType.RUN)
    spans: LocalNodeList["RunSpan"] = p_node_children(NodeType.RUN_SPAN)
    logs: LocalNodeList["Log"] = p_node_children(NodeType.LOG)

    def __content_str__(self):
        node = self.runnable
        path = node.absolute_path if node else "???"
        if self.duration is not None:
            duration_str = f"{self.duration.total_seconds():.3f}s"
            return (
                f"{self.type.bench_name}:{path}, {self.status.bench_name}, duration={duration_str}"
            )
        else:
            return f"{self.type.bench_name}:{path}, {self.status.bench_name}"

    @property
    def runnable(self) -> Union["Flow", "Action", "Pipe", None]:
        if self.type == RunType.PIPE:
            return self.pipe
        elif self.type == RunType.ACTION:
            return self.action
        else:
            return self.flow

    @property
    def ancestors(self):
        parent = self
        while isinstance(parent, Run):
            yield parent
            parent = parent.parent

    @property
    def is_active(self) -> bool:
        return self.status not in TERMINAL_RUN_STATUSES

    @property
    def variable_type(self) -> "TypeBase | None":
        if (pipe := self.pipe) is not None:
            return pipe.variable_type
        elif (action := self.action) is not None:
            return action.variable_type
        elif (flow := self.flow) is not None:
            return flow.variable_type
        else:
            return None

    @property
    def input_type(self) -> "TypeBase | None":
        if (pipe := self.pipe) is not None:
            return pipe.input_type
        elif (action := self.action) is not None:
            return action.input_type
        elif (flow := self.flow) is not None:
            return flow.input_type
        else:
            return None

    @property
    def output_type(self) -> "TypeBase | None":
        if (pipe := self.pipe) is not None:
            return pipe.output_type
        elif (action := self.action) is not None:
            return action.output_type
        elif (flow := self.flow) is not None:
            return flow.output_type
        else:
            return None

    @property
    def base_ptr(self) -> Optional["NodeReference"]:
        if self.pipe_ptr is not None:
            return self.pipe_ptr
        elif self.action_ptr is not None:
            return self.action_ptr
        elif self.flow_ptr is not None:
            return self.flow_ptr
        else:
            return None

    @property
    def base(self) -> Optional["RunnableNode"]:
        if self.pipe_ptr is not None:
            return self.pipe
        elif self.action_ptr is not None:
            return self.action
        elif self.flow_ptr is not None:
            return self.flow
        else:
            return None

    @staticmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        run_data = cast(RunData, data)
        if run_data.pipe_ptr.metatype != 0:
            return cast(RunData, data).pipe_ptr
        elif run_data.action_ptr.metatype != 0:
            return cast(RunData, data).action_ptr
        else:
            return cast(RunData, data).flow_ptr

    @staticmethod
    def get_base_from_partial(data: dict[str, Any]) -> Optional["RunnableNode"]:
        if "pipe" in data:
            return data["pipe"]
        elif "action" in data:
            return data["action"]
        elif "flow" in data:
            return data["flow"]
        else:
            return None

    @property
    def attempts(self) -> Sequence["RunSpan"]:
        return tuple(span for span in self.spans if span.type == RunSpanType.ATTEMPT)

    @property
    def current_attempt(self) -> "RunSpan | None":
        for span in reversed(self.spans):
            if span.type == RunSpanType.ATTEMPT:
                return span
        return None

    def has(self, *nodes: Node, recursive: bool = True) -> bool:
        """Whether the Run has any of the given Nodes."""
        nodes_ck = tuple(n.ck for n in nodes)
        if self.base_ck in nodes_ck:
            return True
        for run in self._graph.get_descendants(self, NodeType.RUN, recursive=recursive):
            run = cast(Run, run)
            if run.base_ck in nodes_ck:
                return True
        return False

    def get_runs(self, runnable: "RunnableNode", recursive: bool = True) -> list["Run"]:
        """Find all Runs of a Node in this Run."""
        matching_runs: list[Run] = []
        if self.base_ck == runnable.ck:
            matching_runs.append(self)
        for run in self._graph.get_descendants(self, NodeType.RUN, recursive=recursive):
            run = cast(Run, run)
            if run.base_ck == runnable.ck:
                matching_runs.append(run)
        matching_runs.sort(
            key=lambda r: r.terminated_at or r.started_at or r.created_at,
            reverse=True,
        )
        return matching_runs

    def get_latest_run(self, runnable: "RunnableNode") -> "Run | None":
        """Find the latest Run of a Node in this Run."""
        matching_runs = self.get_runs(runnable)
        return matching_runs[0] if matching_runs else None

    def pause(self):
        """Mark this Run as paused."""
        assert self._session is not None, f"{self!r} has no session"
        self.paused_at = self._session._oracle.utc()
        thread = self.thread
        if thread:
            thread.pause_run(self)

    def resume(self):
        """Mark this Run as resumed."""
        assert self._session is not None, f"{self!r} has no session"
        self.resumed_at = self._session._oracle.utc()
        thread = self.thread
        if thread:
            thread.resume_run(self)

    def stop(self):
        """Mark this Run as stopped."""
        assert self._session is not None, f"{self!r} has no session"
        self.stopped_at = self._session._oracle.utc()
        thread = self.thread
        if thread:
            thread.stop_run(self)

    def _mark_stopped(self):
        """Mark this Run as stopped."""
        assert self._session is not None, f"{self!r} has no session"
        if self.status.is_terminal:
            return  # already terminated

        # run
        self.terminated_at = self._session._oracle.utc()
        if self.started_at:
            self.duration = self.terminated_at - self.started_at
        self.status = RunStatus.ABORTED if self.status.is_active else RunStatus.CANCELLED

        # last attempt
        if (last_attempt := self.current_attempt) is not None:
            last_attempt.terminated_at = self.terminated_at
            if last_attempt.started_at:
                last_attempt.duration = last_attempt.terminated_at - last_attempt.started_at
            last_attempt.status = (
                RunStatus.ABORTED if last_attempt.status.is_active else RunStatus.CANCELLED
            )

    cancel = abort = stop

    async def wait_until_status(self, status: RunStatus):
        """Wait until this Run reaches the given status."""
        await self.wait_until(lambda self: self.status == status)

    async def wait_until_terminated(self):
        """Wait until this Run is terminated."""
        await self.wait_until(lambda self: self.status.is_terminal)
