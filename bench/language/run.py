from asyncio import CancelledError
from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Any, Collection, Optional, Union, cast

from bench.language.const import (
    TERMINAL_RUN_STATUSES,
    BenchError,
    EnumType,
    NodeType,
    ObjectKind,
    RunErrorKind,
    RunStatus,
    StructType,
    enum_,
)
from bench.language.node import (
    HasNodeBase,
    Node,
    RuntimeNode,
    Struct,
    struct_,
    timed_node_,
)
from bench.language.property import (
    Property,
    p_internal,
    p_node_ancestor,
    p_node_children,
    p_node_parent,
    p_regular,
    p_system,
    p_value_packed,
    p_value_runtime,
)
from bench.language.session import HasSessionContext
from bench.language.text import Text, TextLine
from bench.language.validation import (
    TITLE_CONSTRAINT,
    TypeConstraintIn,
    ValidationError,
    ValidationHandler,
)
from bench.proto.wire import AnyNodeData, NodeReferenceData, RunData
from bench.utils.func import IdEnum
from bench.utils.string import Casing, to_casing
from bench.utils.tenacity import RetryOptions

if TYPE_CHECKING:
    from bench.language import (
        Block,
        Code,
        CustomObject,
        Expression,
        LogInfo,
        LogLevel,
        NodeReference,
        Package,
        Pipe,
        Step,
        TypeBase,
    )


# pyright: reportIncompatibleVariableOverride=false
@enum_(EnumType.RUN_KIND)
class RunKind(IdEnum):
    CODE = 1
    ACTION = 2
    STEP = 10
    FLOW = 11
    PIPE = 12


@enum_(EnumType.MODEL_PROVIDER)
class ModelProvider(IdEnum):
    # internal
    # ...?
    # external
    OPENAI = 100
    ANTHROPIC = 200
    GOOGLE = 300
    META = 400


@enum_(EnumType.MODEL_TYPE)
class ModelType(IdEnum):  # :ModelType
    # internal
    ...  # ?
    # external
    # openai
    OPENAI_GPT4_0 = 101
    OPENAI_GPT4_O_MINI = 102
    OPENAI_O1_PREVIEW = 103
    OPENAI_O1_MINI = 104
    # anthropic
    ANTHROPIC_CLAUDE_3_5_SONNET = 201
    # google
    GOOGLE_GEMINI_1_5_PRO = 301
    # meta
    META_LLAMA_3_1_80B = 401
    META_LLAMA_3_1_400B = 402

    @property
    def provider(self) -> ModelProvider:
        return ModelProvider((self.value // 100) * 100)


@struct_(StructType.TEXT_OPTIONS)
class TextOptions(Struct):
    """Options for text models (in/out)."""

    temperature: Optional[float] = p_regular(
        30, constraint=TypeConstraintIn(min_value=0.0, max_value=5.0)
    )


@struct_(StructType.AUDIO_OPTIONS)
class AudioOptions(Struct):
    """Options for audio models (in/out)."""

    pass


@struct_(StructType.IMAGE_OPTIONS)
class ImageOptions(Struct):
    """Options for image models (in/out)."""

    pass


@struct_(StructType.VIDEO_OPTIONS)
class VideoOptions(Struct):
    """Options for video models (in/out)."""

    pass


@enum_(EnumType.CACHE_MODE)
class CacheMode(IdEnum):
    """How to handle caching."""

    NEVER = 1
    ALWAYS = 2


@struct_(StructType.CONTEXT_OPTIONS)
class ContextOptions(Struct):
    pass


@struct_(StructType.RUN_OPTIONS)
class RunOptions(Struct):
    """
    Options for running something.
    """

    # general
    max_attempts: Optional[int] = p_regular(
        30, constraint=TypeConstraintIn(min_value=-1), description="Maximum retry attempts per Run"
    )
    max_concurrency: Optional[int] = p_regular(
        31,
        constraint=TypeConstraintIn(min_value=0),
        description="Maximum concurrent Runs (per context)",
    )
    max_runs: Optional[int] = p_regular(
        32,
        constraint=TypeConstraintIn(min_value=0),
        description="Maximum number of Runs of this runnable (per context)",
    )
    max_inner_runs: Optional[int] = p_regular(
        33,
        constraint=TypeConstraintIn(min_value=0),
        description="Maximum number of Runs inside this Run (per context)",
    )
    timeout: Optional[timedelta] = p_regular(35)

    # retry
    retry_interval: Optional[timedelta] = p_regular(40)
    backoff: Optional[float] = p_regular(41, constraint=TypeConstraintIn(min_value=1))
    max_retry_interval: Optional[timedelta] = p_regular(42)
    retry_on: list["RunErrorType"] = p_regular(44, array=True)
    suppress_fail: Optional[bool] = p_regular(45, default=None)
    suppress_abort: Optional[bool] = p_regular(46, default=None)

    # context
    context_options: Optional["ContextOptions"] = p_regular(
        50, require=False, array=False, struct=StructType.CONTEXT_OPTIONS
    )

    # debug
    breakpoints: list["Breakpoint"] = p_regular(60, array=True, struct=StructType.BREAKPOINT)

    # cache
    cache_mode: Optional["CacheMode"] = p_regular(70)
    cache_retention: Optional[timedelta] = p_regular(71)

    # model
    model_provider: Optional["ModelProvider"] = p_regular(80)
    model_type: Optional["ModelType"] = p_regular(81)
    text_options: Optional[TextOptions] = p_regular(
        82, require=False, array=False, struct=StructType.TEXT_OPTIONS
    )
    audio_options: Optional[AudioOptions] = p_regular(
        83, require=False, array=False, struct=StructType.AUDIO_OPTIONS
    )
    image_options: Optional[ImageOptions] = p_regular(
        84, require=False, array=False, struct=StructType.IMAGE_OPTIONS
    )
    video_options: Optional[VideoOptions] = p_regular(
        85, require=False, array=False, struct=StructType.VIDEO_OPTIONS
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
            # retry_on is handled separately in runtime because we need the specific RunErrorType
        )


@enum_(EnumType.BREAKPOINT_KIND)
class BreakpointKind(IdEnum):
    # run
    START_RUN = 1
    FAIL_RUN = 2
    COMPLETE_RUN = 3
    # FAIL_ATTEMPT?
    ...
    # text
    ...
    # code
    CODE_LINE = 20
    # step
    ...


@enum_(EnumType.BREAKPOINT_ACTION)
class BreakpointAction(IdEnum):
    SUSPEND = 1


@struct_(StructType.BREAKPOINT)
class Breakpoint(Struct):
    """A breakpoint in some context."""

    kind: BreakpointKind = p_regular(30)
    action: BreakpointAction = p_regular(31, default=BreakpointAction.SUSPEND)

    condition: Optional["Expression"] = p_regular(
        40, require=False, array=False, struct=StructType.EXPRESSION
    )


@struct_(StructType.RUN_ATTEMPT)
class RunAttempt(Struct):
    """A single attempt at a Run."""

    code: Optional["Code"] = p_internal(
        30, default=None, require=False, array=False, struct=StructType.CODE
    )
    status: RunStatus = p_internal(40, default=RunStatus.SCHEDULED)
    duration: Optional[timedelta] = p_internal(41, default=None)
    started_at: Optional[datetime] = p_internal(42, default=None)
    started_epoch: Optional[int] = p_internal(43, default=None)
    terminated_at: Optional[datetime] = p_internal(45, default=None)
    terminated_epoch: Optional[int] = p_internal(46, default=None)
    error: Optional["RunError"] = p_internal(
        47, require=False, array=False, struct=StructType.RUN_ERROR
    )

    def __content_str__(self) -> str:
        if self.duration is not None:
            duration_str = f"{self.duration.total_seconds():.3f}s"
            return f"{self.status.bench_name}, {duration_str}s"
        else:
            return f"{self.status.bench_name}"


@struct_(StructType.RUN_TRACE)
class RunTrace(Struct):
    """A stacktrace for a Run."""

    frames: list["RunFrame"] = p_regular(30, array=True, struct=StructType.RUN_FRAME)


@struct_(StructType.RUN_FRAME)
class RunFrame(Struct):
    """A single frame in a stacktrace."""

    pass


@enum_(EnumType.RUN_SPAN_TYPE)
class RunSpanType(IdEnum):
    CUSTOM = 1000


@struct_(StructType.RUN_SPAN)
class RunSpan(Struct):
    """A span in a Run (a sort of mini-Run inside a tracked Run)."""

    type: RunSpanType = p_regular(30, default=RunSpanType.CUSTOM)
    name: str = p_regular(32, default=None)
    title: Optional[str] = p_regular(33, default=None, constraint=TITLE_CONSTRAINT)
    text: Optional["Text"] = p_regular(34, default=None, struct=StructType.TEXT)
    text_plain: Optional[str] = p_regular(35, default=None)
    started_at: Optional[datetime] = p_regular(40, default=None)
    terminated_at: Optional[datetime] = p_regular(41, default=None)
    duration: Optional[timedelta] = p_regular(42, default=None)


@enum_(EnumType.RUN_EVENT_TYPE)
class RunEventType(IdEnum):
    PAUSED = 1
    RESUMED = 2
    HALTED = 3
    CUSTOM = 1000


@struct_(StructType.RUN_EVENT)
class RunEvent(Struct):
    """An event in a Run of something that happened."""

    type: RunEventType = p_regular(30, default=RunEventType.CUSTOM)
    name: str = p_regular(32, default=None)
    title: Optional[str] = p_regular(33, default=None, constraint=TITLE_CONSTRAINT)
    text: Optional["Text"] = p_regular(34, default=None, struct=StructType.TEXT)
    text_plain: Optional[str] = p_regular(35, default=None)
    created_at: Optional[datetime] = p_regular(40, default=None)
    level: Optional["LogLevel"] = p_regular(41, default=None)


@enum_(EnumType.RUN_ERROR_TYPE)
class RunErrorType(IdEnum):
    # unretryable
    REPLAY = 1
    ABORTED = 2
    RUNTIME_UNAVAILABLE = 3
    RUN_IMPOSSIBLE = 4
    INVALID_VALUE = 5
    CODE_INVALID = 20
    TEXT_INVALID = 21
    MODEL_INCAPABLE = 100
    NON_RETRYABLE = 499
    # retryable
    MODEL_FAILED = 500
    ACTION_MODE_CHANGED = 501
    RETRYABLE = 999

    @property
    def is_retryable(self) -> bool:
        """Whether this error type is retryable *at runtime*"""
        return self > 500


@struct_(StructType.RUN_ERROR)
class RunError(Struct, BenchError):
    """An error that occurred in the context of a Run."""

    kind: RunErrorKind = p_internal(30)
    type: RunErrorType = p_internal(31, default=None)
    title: Optional[str] = p_internal(32, default=None, constraint=TITLE_CONSTRAINT)
    text: Optional["Text"] = p_internal(33, default=None, struct=StructType.TEXT)
    node: Optional["Node"] = p_internal(34, require=False, array=False, references=NodeType.BLOCK)
    trace: Optional[RunTrace] = p_internal(
        35, require=False, array=False, struct=StructType.RUN_TRACE
    )

    def __content_str__(self) -> str:
        parts = [self.kind.bench_name]
        if self.type is not None:
            parts.append(self.type.bench_name)
        parts.append(self.title or "<no title>")
        return ", ".join(parts)

    @property
    def is_retryable(self) -> bool:
        return self.type is None or self.type.is_retryable

    @staticmethod
    def from_exception(kind: RunErrorKind, e: BaseException) -> "RunError":
        # NOTE :Incomplete: get run error trace/frames/node/...
        # pass on inner error if there is one
        if isinstance(getattr(e, "error", None), RunError):
            return getattr(e, "error")  # manual error
        # figure out error data
        title = to_casing(e.__class__.__name__, Casing.CAMEL, allow_whitespace=True)
        if isinstance(e, SyntaxError):
            header_line = TextLine.plain(f"Syntax error at line {e.lineno}, column {e.offset}:")
            code_lines = Text.code(e.args[0])
            text = Text(lines=[header_line, *code_lines.lines])
        else:
            text = Text.plain(str(e))
        if hasattr(e, "run_error_type"):
            typ = getattr(e, "run_error_type")
            assert isinstance(typ, RunErrorType), f"unexpected {typ!r} from {e!r}"
        elif kind == RunErrorKind.RUNTIME:
            if isinstance(e, CancelledError):
                typ = RunErrorType.ABORTED
            elif isinstance(e, (TypeError, ValueError, ValidationError)):
                typ = RunErrorType.INVALID_VALUE
            else:
                typ = RunErrorType.NON_RETRYABLE
        else:
            typ = RunErrorType.NON_RETRYABLE
        return RunError(kind=kind, type=typ, title=title, text=text)


@struct_(StructType.CONTEXT)
class Context(Struct):
    pass


@timed_node_(NodeType.RUN)
class Run(RuntimeNode[RunData], HasNodeBase, HasSessionContext):
    """
    Run a Block, Step or some lambda (Code) in a Session.
    """

    # content
    parent: Union["Package", "Run", None] = p_node_parent(4, NodeType.PACKAGE, NodeType.RUN)
    kind: RunKind = p_system(30)
    root: "Run | None" = p_node_ancestor(
        32, NodeType.RUN, require=False, store=True, wire=True, is_bench_implicit=True
    )
    if TYPE_CHECKING:
        root_ptr: Optional[NodeReference] = None

    block: Optional["Block"] = p_internal(33, require=False, array=False, references=NodeType.BLOCK)
    step: Optional["Step"] = p_internal(34, require=False, array=False, references=NodeType.STEP)
    pipe: Optional["Pipe"] = p_internal(35, require=False, array=False, references=NodeType.PIPE)
    context: Optional["Context"] = p_internal(
        38, require=False, array=False, struct=StructType.CONTEXT
    )
    options: "RunOptions" = p_internal(39, require=True, array=False, struct=StructType.RUN_OPTIONS)

    # status (overall)
    status: RunStatus = p_internal(40, default=RunStatus.SCHEDULED)  # desired status
    duration: Optional[timedelta] = p_internal(
        41,
        default=None,
        description="Duration from first attempt start to last attempt termination.",
    )
    cached_duration: Optional[timedelta] = p_internal(
        42,
        default=None,
        description="Total original duration of contained Run cache hits.",
    )
    active_duration: Optional[timedelta] = p_internal(
        43,
        default=None,
        description="Total duration of direct (or contained) Run activity.",
    )
    attempts: list[RunAttempt] = p_internal(44, array=True, struct=StructType.RUN_ATTEMPT)
    error: Optional["RunError"] = p_internal(
        45, default=None, require=False, array=False, struct=StructType.RUN_ERROR
    )
    scheduled_at: Optional[datetime] = p_system(46, default=None)
    scheduled_epoch: Optional[int] = p_system(47, default=None)
    started_at: Optional[datetime] = p_internal(48, default=None)
    started_epoch: Optional[int] = p_internal(49, default=None)
    killed_at: Optional[datetime] = p_internal(50, default=None)
    # halted/... ...
    terminated_at: Optional[datetime] = p_internal(55, default=None)
    terminated_epoch: Optional[int] = p_internal(56, default=None)

    # content
    inputs_packed: Any = p_value_packed(60)
    inputs: "CustomObject | None" = p_value_runtime(
        60, kind=ObjectKind.INPUT, typ=lambda self: cast("Run", self).input_type
    )
    outputs_packed: Any = p_value_packed(61)
    outputs: "CustomObject | None" = p_value_runtime(
        61, kind=ObjectKind.OUTPUT, typ=lambda self: cast("Run", self).output_type
    )
    variables_packed: Any = p_value_packed(62)
    variables: "CustomObject | None" = p_value_runtime(
        62, kind=ObjectKind.VARIABLE, typ=lambda self: cast("Run", self).variable_type
    )
    logs: list["LogInfo"] = p_internal(65, array=True, struct=StructType.LOG_INFO)
    spans: list["RunSpan"] = p_internal(66, array=True, struct=StructType.RUN_SPAN)
    events: list["RunEvent"] = p_internal(67, array=True, struct=StructType.RUN_EVENT)

    # ...HasSessionContext[70-89]

    runs: list["Run"] = p_node_children(NodeType.RUN)

    def __content_str__(self):
        node = self.step or self.block
        path = node.absolute_path if node else "<lambda>"
        if self.duration is not None:
            duration_str = f"{self.duration.total_seconds():.3f}s"
            return (
                f"{self.kind.bench_name}:{path}, {self.status.bench_name}, duration={duration_str}"
            )
        else:
            return f"{self.kind.bench_name}:{path}, {self.status.bench_name}"

    @property
    def is_active(self) -> bool:
        return self.status not in TERMINAL_RUN_STATUSES

    @property
    def variable_type(self) -> "TypeBase | None":
        if self.step is not None:
            return self.step.variable_type
        elif self.block is not None:
            return self.block.variable_type
        else:
            return None

    @property
    def input_type(self) -> "TypeBase | None":
        if self.step is not None:
            return self.step.input_type
        elif self.block is not None:
            return self.block.input_type
        else:
            return None

    @property
    def output_type(self) -> "TypeBase | None":
        if self.step is not None:
            return self.step.output_type
        elif self.block is not None:
            return self.block.output_type
        else:
            return None

    @property
    def base(self) -> Optional["Step | Block"]:
        if self.step_ptr is not None:
            return self.step
        else:
            return self.block

    @staticmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        run_data = cast(RunData, data)
        if run_data.step_ptr.metatype != 0:
            return cast(RunData, data).step_ptr
        else:
            return cast(RunData, data).block_ptr

    def cancel(self):
        """Cancels the Run before it happens."""
        self.killed_at = self.active_session._oracle.utc()

    def abort(self):
        """Stops an active Run forcefully."""
        self.killed_at = self.active_session._oracle.utc()

    def kill(self):
        """Kills a Run by any means necessary."""
        self.killed_at = self.active_session._oracle.utc()

    def _validate_component(
        self, properties: Collection[Property], invalid: ValidationHandler
    ) -> None:
        if self.root_ptr is not None and self.root_ptr.id == self.id:
            invalid(self, "root points to self", (Run.root, Run.id))
