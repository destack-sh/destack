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
    Severity,
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
from bench.pb2 import AnyNodeData, NodeReferenceData, RunData, RunPlanData, RunSpanData
from bench.utils.tenacity import RetryOptions

if TYPE_CHECKING:
    from bench.language import (
        Action,
        AudioOptions,
        Bench,
        Block,
        Breakpoint,
        Call,
        CallExecutionMode,
        CallFailureMode,
        CallPlan,
        CallTerminationMode,
        Code,
        CustomObject,
        Error,
        ErrorType,
        ImageOptions,
        Interruption,
        Log,
        ModelDeveloper,
        ModelFamily,
        ModelType,
        NodeReference,
        Pipe,
        RunnableNode,
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


@timed_node_(NodeType.RUN_PLAN)
class RunPlan(RuntimeNode[RunPlanData]):
    """A RunPlan is a plan for a sequence of Runs."""

    # meta
    parent: Union["Run", None] = p_node_parent(4, NodeType.RUN)
    execution: "CallExecutionMode" = p_internal(31)
    on_terminate: "CallTerminationMode" = p_internal(33)
    on_error: "CallFailureMode" = p_internal(34)

    # status
    status: RunStatus = p_regular(40, default=RunStatus.SCHEDULED)
    started_by: "Run" = p_internal(41, require=False, array=False, references=NodeType.RUN)
    terminated_by: Optional["Run"] = p_internal(
        42, require=False, array=False, references=NodeType.RUN
    )
    error: Optional["Error"] = p_internal(53, require=False, array=False, struct=StructType.ERROR)
    if TYPE_CHECKING:
        started_by_ptr: Optional[NodeReference] = None
        started_by_id: Optional[UUID] = None
        terminated_by_ptr: Optional[NodeReference] = None
        terminated_by_id: Optional[UUID] = None

    # content
    title: str | None = p_regular(50, default=None)
    text: Optional["Text"] = p_regular(51, default=None, struct=StructType.TEXT)
    calls: list["Call"] = p_internal(52, require=True, array=True, struct=StructType.CALL)
    step: int | None = p_internal(55)

    def complete(self, by: "Run") -> None:
        self.terminated_by = by
        self.status = RunStatus.COMPLETED

    def fail(self, by: "Run") -> None:
        self.terminated_by = by
        self.error = by.error
        self.status = RunStatus.FAILED

    @staticmethod
    def new(
        run: "Run",
        call_plan: "CallPlan",
        status: RunStatus = RunStatus.SCHEDULED,
    ) -> "RunPlan":
        return RunPlan(
            parent=run,
            status=status,
            started_by=run,
            execution=call_plan.execution,
            on_terminate=call_plan.on_terminate,
            on_error=call_plan.on_error,
            calls=call_plan.calls,
            _skip_validate_self=True,
        )


@timed_node_(NodeType.RUN_SPAN)
class RunSpan(RuntimeNode[RunSpanData]):
    """A RunSpan is a specific not-individually-controllable part of a Run."""

    # meta
    parent: Union["Run", None] = p_node_parent(4, NodeType.RUN)
    type: RunSpanType = p_regular(30)
    root: "Run | None" = p_node_ancestor(
        31, NodeType.RUN, require=False, store=True, wire=True, is_bench_implicit=True
    )
    if TYPE_CHECKING:
        root_ptr: Optional[NodeReference] = None
        root_id: Optional[UUID] = None
    severity: "Severity" = p_regular(33, default=Severity.INFO)

    # status
    status: RunStatus = p_regular(40, default=RunStatus.SCHEDULED)
    duration: Optional[timedelta] = p_regular(41, default=None)
    # cached_duration, active_duration, ...?
    started_at: Optional[datetime] = p_regular(42, default=None)
    terminated_at: Optional[datetime] = p_regular(43, default=None)
    interrupted_at: Optional[datetime] = p_regular(44, default=None)
    interruption: Optional["Interruption"] = p_internal(
        53, require=False, array=False, references=NodeType.INTERRUPTION, same_bench=True
    )
    error: Optional["Error"] = p_internal(54, require=False, array=False, struct=StructType.ERROR)

    # content
    title: str | None = p_regular(60, default=None)
    text: Optional["Text"] = p_regular(61, default=None, struct=StructType.TEXT)
    code: Optional["Code"] = p_regular(62, default=None, struct=StructType.CODE)
    nodes: list["Node"] = p_regular(65, array=True, require=False, references="any")

    @property
    def is_retryable(self) -> bool:
        return self.error is None or self.error.is_retryable


@timed_node_(NodeType.RUN, index=(("trigger_id", "trigger_key"),))
class Run(RuntimeNode[RunData], HasNodeBase):
    """
    Run a Block, Action or some lambda (Code) in a Session.
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
    block: Optional["Block"] = p_internal(32, require=False, array=False, references=NodeType.BLOCK)
    action: Optional["Action"] = p_internal(
        33, require=False, array=False, references=NodeType.ACTION
    )
    pipe: Optional["Pipe"] = p_internal(34, require=False, array=False, references=NodeType.PIPE)
    if TYPE_CHECKING:
        block_ptr: Optional[NodeReference] = None
        block_id: Optional[UUID] = None
        block_ck: Optional[UUID] = None
        action_ptr: Optional[NodeReference] = None
        action_id: Optional[UUID] = None
        action_ck: Optional[UUID] = None
        pipe_ptr: Optional[NodeReference] = None
        pipe_id: Optional[UUID] = None
        pipe_ck: Optional[UUID] = None
    options: "RunOptions" = p_internal(39, require=True, array=False, struct=StructType.RUN_OPTIONS)

    # flow
    incoming: list["Run"] = p_internal(
        40, require=False, array=True, references=NodeType.RUN, same_bench=True
    )
    plan: Optional["RunPlan"] = p_internal(
        41, require=False, array=False, references=NodeType.RUN_PLAN, same_bench=True
    )
    plan_step: int | None = p_internal(42)
    trigger: Optional["Trigger"] = p_regular(
        43, require=False, array=False, references=NodeType.TRIGGER, same_bench=True
    )
    if TYPE_CHECKING:
        incoming_ptr: tuple["NodeReference", ...] = ()
        trigger_ptr: Optional[NodeReference] = None
        trigger_id: Optional[UUID] = None
    trigger_key: Optional[str] = p_internal(44, require=False, default=None)

    # status
    status: RunStatus = p_internal(50, default=RunStatus.SCHEDULED)
    attempt: int | None = p_internal(51)
    duration: Optional[timedelta] = p_internal(
        52,
        default=None,
        description="Duration from first attempt start to last attempt termination.",
    )
    # cached_duration, active_duration, ...?
    scheduled_at: Optional[datetime] = p_system(55, default=None)
    started_at: Optional[datetime] = p_internal(56, default=None)
    stopped_at: Optional[datetime] = p_internal(57, default=None)
    interrupted_at: Optional[datetime] = p_internal(58, default=None)
    paused_at: Optional[datetime] = p_internal(59, default=None)
    resumed_at: Optional[datetime] = p_internal(60, default=None)
    terminated_at: Optional[datetime] = p_internal(61, default=None)
    error: Optional["Error"] = p_internal(
        62, default=None, require=False, array=False, struct=StructType.ERROR
    )
    interruption: Optional["Interruption"] = p_internal(
        63, require=False, array=False, references=NodeType.INTERRUPTION, same_bench=True
    )

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
    def runnable(self) -> Union["Block", "Action", "Pipe", None]:
        if self.type == RunType.PIPE:
            return self.pipe
        elif self.type == RunType.ACTION:
            return self.action
        else:
            return self.block

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
        elif (block := self.block) is not None:
            return block.variable_type
        else:
            return None

    @property
    def input_type(self) -> "TypeBase | None":
        if (pipe := self.pipe) is not None:
            return pipe.input_type
        elif (action := self.action) is not None:
            return action.input_type
        elif (block := self.block) is not None:
            return block.input_type
        else:
            return None

    @property
    def output_type(self) -> "TypeBase | None":
        if (pipe := self.pipe) is not None:
            return pipe.output_type
        elif (action := self.action) is not None:
            return action.output_type
        elif (block := self.block) is not None:
            return block.output_type
        else:
            return None

    @property
    def base_ptr(self) -> Optional["NodeReference"]:
        if self.pipe_ptr is not None:
            return self.pipe_ptr
        elif self.action_ptr is not None:
            return self.action_ptr
        else:
            return self.block_ptr

    @property
    def base(self) -> Optional["RunnableNode"]:
        if self.pipe_ptr is not None:
            return self.pipe
        elif self.action_ptr is not None:
            return self.action
        else:
            return self.block

    @staticmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        run_data = cast(RunData, data)
        if run_data.pipe_ptr.metatype != 0:
            return cast(RunData, data).pipe_ptr
        elif run_data.action_ptr.metatype != 0:
            return cast(RunData, data).action_ptr
        else:
            return cast(RunData, data).block_ptr

    @property
    def attempts(self) -> Sequence[RunSpan]:
        return tuple(span for span in self.spans if span.type == RunSpanType.ATTEMPT)

    @property
    def current_attempt(self) -> RunSpan | None:
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
            thread.pause(self)

    def resume(self):
        """Mark this Run as resumed."""
        assert self._session is not None, f"{self!r} has no session"
        self.resumed_at = self._session._oracle.utc()
        thread = self.thread
        if thread:
            thread.resume(self)

    def stop(self):
        """Mark this Run as stopped."""
        assert self._session is not None, f"{self!r} has no session"
        self.stopped_at = self._session._oracle.utc()
        thread = self.thread
        if thread:
            thread.stop(self)

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


#
# Control flow
#
