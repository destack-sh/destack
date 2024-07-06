from datetime import datetime
from typing import TYPE_CHECKING, Any, Collection, Optional, Union, cast

from bench.language.code import Code
from bench.language.const import (
    TERMINAL_RUN_STATUSES,
    BenchError,
    EnumType,
    FieldZone,
    NodeType,
    RunErrorKind,
    RunKind,
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
    timed_node,
)
from bench.language.property import (
    Property,
    p_internal,
    p_node_ancestor,
    p_node_children,
    p_node_parent,
    p_regular,
    p_secret_value_packed,
    p_system,
    p_value_packed,
    p_value_runtime,
)
from bench.language.session import HasSessionContext
from bench.language.text import Text
from bench.language.validation import TITLE_CONSTRAINT, TypeConstraintIn, ValidationHandler
from bench.language.value import HasValues
from bench.proto.wire import AnyNodeData, NodeReferenceData, RunData
from bench.utils.casing import Casing, to_casing
from bench.utils.func import IdEnum
from bench.utils.tenacity import RetryOptions

if TYPE_CHECKING:
    from bench.language import Block, Expression, NodeReference, Package, TypeInfoBase, ValueObject

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.CODE_KIND)
class CodeKind(IdEnum):
    """
    The implicit 'kind' of some Code.
    We don't set this explicitly in Code because it depends on where the Code is used.
    """

    SNIPPET = 2  # for inline expressions and procedures anywhere (import only)
    SCRIPT = 4  # for defining Python-level commons in Block 'scripts' (import & export)
    FUNCTION = 6  # for Python functions in Blocks/Steps (import only)


@enum_(EnumType.RUNNABLE_KIND)
class RunnableKind(IdEnum):
    """The kind of some runnable."""

    CODE = 1
    TEXT = 2
    STEP = 3
    FLOW = 4


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
    retry_interval: Optional[float] = p_regular(33, constraint=TypeConstraintIn(min_value=0))
    backoff: Optional[float] = p_regular(34, constraint=TypeConstraintIn(min_value=1))
    max_retry_interval: Optional[float] = p_regular(35, constraint=TypeConstraintIn(min_value=0))
    jitter: Optional[float] = p_regular(36, constraint=TypeConstraintIn(min_value=0, max_value=1))
    retry_on: list["RunErrorType"] = p_regular(38, array=True)
    breakpoints: list["Breakpoint"] = p_regular(39, array=True, struct=StructType.BREAKPOINT)

    # flow
    ...

    def to_retry(self) -> RetryOptions:
        """Turns the options into our RetryOptions."""
        return RetryOptions(
            max_attempts=self.max_attempts or 1,
            retry_interval=self.retry_interval or 1,
            backoff=self.backoff or 2,
            max_retry_interval=self.max_retry_interval or 30,
            jitter=self.jitter,
        )


@enum_(EnumType.BREAKPOINT_KIND)
class BreakpointKind(IdEnum):
    # run
    START_RUN = 1
    FAIL_RUN = 2
    COMPLETE_RUN = 3
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
        duration_str = f"{self.duration:.3f}" if self.duration else "<running>"
        return f"{self.status.bench_name}, {duration_str}s"


@struct_(StructType.RUN_TRACE)
class RunTrace(Struct):
    """A stacktrace for a Run."""

    frames: list["RunFrame"] = p_regular(30, array=True, struct=StructType.RUN_FRAME)


@struct_(StructType.RUN_FRAME)
class RunFrame(Struct):
    """A single frame in a stacktrace."""

    pass


@enum_(EnumType.RUN_ERROR_TYPE)
class RunErrorType(IdEnum):
    RUNTIME_UNAVAILABLE = 1
    NOT_RUNNABLE = 2


@struct_(StructType.RUN_ERROR)
class RunError(Struct, BenchError):
    """An error that occurred in the context of a Run."""

    kind: RunErrorKind = p_internal(30)
    type: Optional[RunErrorType] = p_internal(31, default=None)
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

    @staticmethod
    def from_exception(kind: RunErrorKind, e: Exception) -> "RunError":
        # NOTE :Incomplete: get run error trace/frames/node/...
        title = to_casing(e.__class__.__name__, Casing.CAMEL, allow_whitespace=True)
        return RunError(kind=kind, title=title, text=Text.plain(str(e)))


@timed_node(NodeType.RUN)
class Run(PackageNode[RunData], HasTimeIdentity, HasNodeBase, HasSessionContext, HasValues):
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
    status: RunStatus = p_regular(40, default=RunStatus.SCHEDULED)  # desired status
    current_status: Optional[RunStatus] = p_regular(41, default=None)
    duration: Optional[float] = p_regular(
        42,
        default=None,
        description="Duration in seconds from first attempt start to last attempt termination.",
    )
    attempts: list[RunAttempt] = p_internal(43, array=True, struct=StructType.RUN_ATTEMPT)
    error: Optional["RunError"] = p_internal(
        44, default=None, require=False, array=False, struct=StructType.RUN_ERROR
    )
    scheduled_at: Optional[datetime] = p_regular(45, default=None)
    scheduled_epoch: Optional[int] = p_regular(46, default=None)
    started_at: Optional[datetime] = p_regular(47, default=None)
    started_epoch: Optional[int] = p_regular(48, default=None)
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
    inputs_secret_packed: Any = p_secret_value_packed(61)
    inputs: "ValueObject | None" = p_value_runtime(
        60, 61, typ=lambda self: cast("Run", self).input_type
    )
    outputs_packed: Any = p_value_packed(62)
    outputs_secret_packed: Any = p_secret_value_packed(63)
    outputs: "ValueObject | None" = p_value_runtime(
        62, 63, typ=lambda self: cast("Run", self).output_type
    )
    value_packed: Any = p_value_packed(64)
    value_secret_packed: Any = p_secret_value_packed(65)
    value: "ValueObject | None" = p_value_runtime(64, 65, typ=None)  # freely typed

    # ...HasSessionContext[70-79]

    # NOTE :Architecture :Performance: (some) Runs will likely be stored outside the main user DB later.
    #  And maybe we'll also have 'inline runs' for non-Bench constructs that were run (like deeper profiling).
    runs: list["Run"] = p_node_children(NodeType.RUN)

    def __content_str__(self):
        if self.kind == RunKind.BLOCK:
            content_str = self.block.absolute_path if self.block else str(self.block_ptr)
        elif self.kind == RunKind.STEP:
            content_str = self.step.absolute_path if self.step else str(self.step_ptr)
        else:
            content_str = repr(self.code) if self.code else "<no code>"
        if self.duration is not None:
            return f"{content_str}, {self.status.bench_name}, duration={self.duration:.3f}s"
        else:
            return f"{content_str}, {self.status.bench_name}"

    @property
    def is_active(self) -> bool:
        return self.status not in TERMINAL_RUN_STATUSES

    @property
    def input_type(self) -> "TypeInfoBase | None":
        if self.step is not None:
            return self.step.to_type(as_object=True, zone=FieldZone.INPUT)
        elif self.block is not None:
            return self.block.to_type(as_object=True, zone=FieldZone.INPUT)
        else:
            return None  # freely typed

    @property
    def output_type(self) -> "TypeInfoBase | None":
        if self.step is not None:
            return self.step.to_type(as_object=True, zone=FieldZone.OUTPUT)
        elif self.block is not None:
            return self.block.to_type(as_object=True, zone=FieldZone.OUTPUT)
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
        assert self.current_status == RunStatus.RUNNING, f"cannot pause {self!r}"
        self.status = RunStatus.PAUSED

    def resume(self):
        """Resumes the Run."""
        assert self.current_status == RunStatus.PAUSED, f"cannot resume {self!r}"
        self.status = RunStatus.RUNNING

    def cancel(self):
        """Cancels the Run before it happens."""
        assert not self.status.is_terminal and not self.status.is_active, f"cannot cancel {self!r}"
        self.status = RunStatus.CANCELLED

    def abort(self):
        """Stops an active Run forcefully."""
        assert self.status.is_active, f"cannot abort {self!r}"
        self.status = RunStatus.ABORTED

    def kill(self):
        """Kills a Run by any means necessary."""
        assert not self.status.is_terminal, f"cannot kill {self!r}"
        if self.status.is_active:
            self.abort()
        else:
            self.cancel()

    def fail(self, error: "RunError"):
        """Fails the Run with the given error."""
        assert not self.status.is_terminal, f"cannot fail {self!r}"
        self.status = RunStatus.FAILED
        self.error = error

    def _validate_component(
        self, properties: Collection[Property], invalid: ValidationHandler
    ) -> None:
        if self.root_ptr is not None and self.root_ptr.id == self.id:
            invalid(self, "root points to self", (Run.root, Run.id))
