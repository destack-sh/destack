from asyncio import CancelledError
from datetime import datetime
from typing import TYPE_CHECKING, Any, Collection, Optional, Union, assert_never, cast

from bench.language.code import Code
from bench.language.const import (
    TERMINAL_RUN_STATUSES,
    BenchError,
    EnumType,
    NodeType,
    RunErrorKind,
    RunStatus,
    StructType,
    enum_,
)
from bench.language.node import (
    HasNodeBase,
    HasTimeIdentity,
    Node,
    PackageNode,
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
        Expression,
        LogInfo,
        LogLevel,
        NodeReference,
        Package,
        Step,
        TypeInfoBase,
        ValueObject,
    )


# pyright: reportIncompatibleVariableOverride=false
@enum_(EnumType.RUN_KIND)
class RunKind(IdEnum):
    """The kind of some runnable."""

    CODE = 1
    TEXT = 2
    STEP = 3
    FLOW = 4


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
class ModelType(IdEnum):
    # internal
    ...  # ?
    # external
    # openai
    GPT4_0 = 101
    GPT4_O_MINI = 102
    # anthropic
    CLAUDE_3_5_SONNET = 201
    # google
    GEMINI_1_5_PRO = 301
    # meta
    LLAMA_3_1_80B = 401
    LLAMA_3_1_400B = 402

    @property
    def provider(self) -> ModelProvider:
        return ModelProvider((self.value // 100) * 100)


@struct_(StructType.MODEL_OPTIONS)
class ModelOptions(Struct):
    """Options for running an ML model."""

    provider: ModelProvider | None = p_regular(30)
    model: ModelType | None = p_regular(31)


@struct_(StructType.RUN_OPTIONS)
class RunOptions(Struct):
    """
    Options for running something.
    Limits are per top level run context (i.e. the root Run in some Session).
    """

    # general
    max_runs: Optional[int] = p_regular(30, constraint=TypeConstraintIn(min_value=0))
    max_concurrency: Optional[int] = p_regular(31, constraint=TypeConstraintIn(min_value=0))
    max_attempts: Optional[int] = p_regular(32, constraint=TypeConstraintIn(min_value=-1))
    timeout: Optional[float] = p_regular(33, constraint=TypeConstraintIn(min_value=0))

    # retry
    retry_interval: Optional[float] = p_regular(40, constraint=TypeConstraintIn(min_value=0))
    backoff: Optional[float] = p_regular(41, constraint=TypeConstraintIn(min_value=1))
    max_retry_interval: Optional[float] = p_regular(42, constraint=TypeConstraintIn(min_value=0))
    jitter: Optional[float] = p_regular(43, constraint=TypeConstraintIn(min_value=0, max_value=1))
    retry_on: list["RunErrorType"] = p_regular(44, array=True)

    # debug
    breakpoints: list["Breakpoint"] = p_regular(50, array=True, struct=StructType.BREAKPOINT)

    # model
    model_options: Optional["ModelOptions"] = p_regular(
        60, require=False, array=False, struct=StructType.MODEL_OPTIONS
    )

    def to_retry(self) -> RetryOptions:
        """Turns the options into our RetryOptions."""
        return RetryOptions(
            max_attempts=self.max_attempts or 1,
            retry_interval=self.retry_interval or 1,
            backoff=self.backoff or 2,
            max_retry_interval=self.max_retry_interval or 30,
            jitter=self.jitter,
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

    status: RunStatus = p_internal(30, default=RunStatus.SCHEDULED)
    duration: Optional[float] = p_internal(31, default=None)
    started_at: Optional[datetime] = p_internal(32, default=None)
    started_epoch: Optional[int] = p_internal(33, default=None)
    terminated_at: Optional[datetime] = p_internal(35, default=None)
    terminated_epoch: Optional[int] = p_internal(36, default=None)
    error: Optional["RunError"] = p_internal(
        37, require=False, array=False, struct=StructType.RUN_ERROR
    )

    def __content_str__(self) -> str:
        if self.duration:
            return f"{self.status.bench_name}, {self.duration:.3f}s"
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
    """A span in a Run (a sort of sub-Run)."""

    type: RunSpanType = p_regular(30, default=RunSpanType.CUSTOM)
    name: str = p_regular(32, default=None)
    title: Optional[str] = p_regular(33, default=None, constraint=TITLE_CONSTRAINT)
    text: Optional["Text"] = p_regular(34, default=None, struct=StructType.TEXT)
    text_plain: Optional[str] = p_regular(35, default=None)
    started_at: Optional[datetime] = p_regular(40, default=None)
    ended_at: Optional[datetime] = p_regular(41, default=None)
    duration: Optional[float] = p_regular(42, default=None)


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
    MANUAL_NONRETRYABLE = 498
    UNKNOWN_NONRETRYABLE = 499
    # retryable
    MODEL_FAILED = 500
    MANUAL_RETRYABLE = 998
    UNKNOWN_RETRYABLE = 999

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
            elif isinstance(e, ValidationError):
                typ = RunErrorType.INVALID_VALUE
            else:
                typ = RunErrorType.RUNTIME_UNAVAILABLE
        else:
            typ = RunErrorType.UNKNOWN_NONRETRYABLE
        return RunError(kind=kind, type=typ, title=title, text=text)


@timed_node_(NodeType.RUN)
class Run(PackageNode[RunData], HasTimeIdentity, HasNodeBase, HasSessionContext):
    """
    A 'run' of Blocks (and Steps within them) or 'lambdas' (just Code/Text).
    When 'running' something that's not directly runnable (like a Text Block, Text Step or Text Lambda),
     we implicitly pass it to the corresponding default Text program.
    Once terminated, a Run is effectively immutable.
    """

    # content
    parent: Union["Package", "Run", None] = p_node_parent(4, NodeType.PACKAGE, NodeType.RUN)
    kind: RunKind = p_system(30)
    root: "Run | None" = p_node_ancestor(
        32, NodeType.RUN, require=False, store=True, wire=True, is_bench_implicit=True
    )
    if TYPE_CHECKING:
        root_ptr: Optional[NodeReference] = None

    code: Optional["Code"] = p_internal(36, require=False, array=False, struct=StructType.CODE)
    # extra run options if different from base or it's a lambda
    options: Optional["RunOptions"] = p_regular(
        38, require=False, array=False, struct=StructType.RUN_OPTIONS
    )

    # status (overall)
    status: RunStatus = p_internal(40, default=RunStatus.SCHEDULED)  # desired status
    duration: Optional[float] = p_regular(
        41,
        default=None,
        description="Duration in seconds from first attempt start to last attempt termination.",
    )
    attempts: list[RunAttempt] = p_internal(42, array=True, struct=StructType.RUN_ATTEMPT)
    error: Optional["RunError"] = p_internal(
        43, default=None, require=False, array=False, struct=StructType.RUN_ERROR
    )
    scheduled_at: Optional[datetime] = p_regular(44, default=None)
    scheduled_epoch: Optional[int] = p_regular(45, default=None)
    started_at: Optional[datetime] = p_regular(46, default=None)
    started_epoch: Optional[int] = p_regular(47, default=None)
    killed_at: Optional[datetime] = p_regular(48, default=None)
    halted_at: Optional[datetime] = p_regular(49, default=None)
    halted_epoch: Optional[int] = p_regular(50, default=None)
    halted_on_run: Optional["Run"] = p_regular(
        51, require=False, array=False, same_bench=True, references=NodeType.RUN
    )
    # halted_on_trigger: ...
    terminated_at: Optional[datetime] = p_regular(53, default=None)
    terminated_epoch: Optional[int] = p_regular(54, default=None)

    # content
    inputs_packed: Any = p_value_packed(60)
    inputs: "ValueObject | None" = p_value_runtime(
        60, typ=lambda self: cast("Run", self).input_type
    )
    outputs_packed: Any = p_value_packed(61)
    outputs: "ValueObject | None" = p_value_runtime(
        61, typ=lambda self: cast("Run", self).output_type
    )
    value_packed: Any = p_value_packed(62)
    value: "ValueObject | None" = p_value_runtime(62, typ=None)  # freely typed
    logs: list["LogInfo"] = p_internal(65, array=True, struct=StructType.LOG_INFO)
    spans: list["RunSpan"] = p_internal(66, array=True, struct=StructType.RUN_SPAN)
    events: list["RunEvent"] = p_internal(67, array=True, struct=StructType.RUN_EVENT)

    # ...HasSessionContext[70-79]

    runs: list["Run"] = p_node_children(NodeType.RUN)

    def __content_str__(self):
        node = self.step or self.block
        path = node.absolute_path if node else "<lambda>"
        if self.duration is not None:
            return f"{self.kind.bench_name}:{path}, {self.status.bench_name}, duration={self.duration:.3f}s"
        else:
            return f"{self.kind.bench_name}:{path}, {self.status.bench_name}"

    @property
    def is_active(self) -> bool:
        return self.status not in TERMINAL_RUN_STATUSES

    @property
    def input_type(self) -> "TypeInfoBase | None":
        if self.step is not None:
            return self.step.input_type
        elif self.block is not None:
            return self.block.input_type
        else:
            return None  # freely typed

    @property
    def output_type(self) -> "TypeInfoBase | None":
        if self.step is not None:
            return self.step.output_type
        elif self.block is not None:
            return self.block.output_type
        else:
            return None  # freely typed

    @property
    def base(self) -> Optional["Block"]:
        return self.block

    @staticmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        return cast(RunData, data).block_ptr

    def pause(self):
        """Pauses the Run."""
        self.halted_at = self.active_session._oracle.utc()

    def resume(self):
        """Resumes the Run."""
        self.halted_at = None

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

    @staticmethod
    def from_runnable(
        node: "Block | Step", *, inputs: Any | None = None, parent: "Run | None" = None
    ) -> "Run":
        """Creates a Run from a Block."""
        from bench.language import Block, Step
        from bench.language.value import coerce_value_object

        assert node.package is not None, f"no package for {node!r}"

        if isinstance(node, Block):
            step = None
            block = node
            kind = node.run_kind
            assert kind is not None, f"no run kind for {node!r}"
        elif isinstance(node, Step):
            step = node
            block = step.block
            kind = RunKind.STEP
        else:
            assert_never(node)

        run = Run(parent=parent or node.package, kind=kind, block=block, step=step)
        if inputs is None:
            inputs = {}
        if run.input_type is not None:
            run.inputs = coerce_value_object(run.input_type, inputs)
        return run
