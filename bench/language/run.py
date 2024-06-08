from datetime import datetime
from typing import TYPE_CHECKING, Any, Collection, Optional, Union, cast

from bench.language.code_ import Code
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
    p_node_ancestor_first,
    p_node_child,
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
from bench.utils.dt import utcnow
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Block, NodeReference, Package, TypeInfoBase

# pyright: reportIncompatibleVariableOverride=false


@struct_(StructType.RUN_OPTIONS)
class RunOptions(Struct):
    """Options for running something."""

    max_concurrency: Optional[int] = p_regular(30, constraint=TypeConstraintIn(min_value=0))
    max_attempts: Optional[int] = p_regular(31, constraint=TypeConstraintIn(min_value=-1))
    retry_interval: Optional[float] = p_regular(32, constraint=TypeConstraintIn(min_value=0))
    backoff: Optional[float] = p_regular(33, constraint=TypeConstraintIn(min_value=1))
    max_retry_interval: Optional[float] = p_regular(34, constraint=TypeConstraintIn(min_value=0))
    retry_on: list["RunErrorType"] = p_regular(35, array=True)


@struct_(StructType.RETRY_ATTEMPT)
class RetryAttempt(Struct):
    """A single attempt at a Run."""

    status: RunStatus = p_internal(30, default=RunStatus.SCHEDULED)
    duration: Optional[float] = p_internal(31, default=None)
    started_at: Optional[datetime] = p_internal(32, default=None)
    started_epoch: Optional[int] = p_internal(33, default=None)
    paused_at: Optional[datetime] = p_internal(34, default=None)
    terminated_at: Optional[datetime] = p_internal(35, default=None)
    terminated_epoch: Optional[int] = p_internal(36, default=None)
    error: Optional["RunError"] = p_internal(
        37, require=False, array=False, struct=StructType.RUN_ERROR
    )


@enum_(EnumType.RUN_ERROR_TYPE)
class RunErrorType(IdEnum):
    NO_RUNTIME_AVAILABLE = 1


@struct_(StructType.RUN_ERROR)
class RunError(Struct, BenchError):
    """An error that occurred in the context of a Run."""

    kind: RunErrorKind = p_internal(30)
    type: Optional[RunErrorType] = p_internal(31, default=None)
    title: Optional[str] = p_internal(32, default=None, constraint=TITLE_CONSTRAINT)
    text: Optional["Text"] = p_internal(33, default=None, struct=StructType.TEXT)
    node: Optional["Node"] = p_internal(34, require=False, array=False, references=NodeType.BLOCK)

    def __content_str__(self) -> str:
        parts = [self.kind.bench_name]
        if self.type is not None:
            parts.append(self.type.bench_name)
        parts.append(self.title or "<no title>")
        return ", ".join(parts)

    @staticmethod
    def from_exception(e: Exception) -> "RunError":
        return RunError(kind=RunErrorKind.INTERNAL, title=str(e))


@timed_node(NodeType.RUN)
class Run(PackageNode[RunData], HasTimeIdentity, HasNodeBase, HasSessionContext, HasValues):
    """
    A 'run' of Blocks (and Steps within them) or 'lambdas' (just Code/Text).
    When 'running' something that's not directly runnable (like a Text Block, Text Step or Text Lambda),
     we implicitly pass it to the corresponding default Text program.
    Once terminated, a Run is effectively immutable.
    """

    # content
    parent: Union["Package", "Run"] = p_node_parent(4, NodeType.PACKAGE, NodeType.RUN)
    kind: RunKind = p_system(30)
    root: "Run" = p_node_ancestor_first(
        32, NodeType.RUN, require=True, store=True, wire=True, is_bench_implicit=True
    )
    if TYPE_CHECKING:
        root_ptr: Optional[NodeReference] = None

    code: Optional["Code"] = p_internal(36, require=False, array=False, struct=StructType.CODE)
    text: Optional["Text"] = p_internal(37, require=False, array=False, struct=StructType.TEXT)
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
    scheduled_at: Optional[datetime] = p_regular(43, default=None)
    scheduled_epoch: Optional[int] = p_regular(44, default=None)
    started_at: Optional[datetime] = p_regular(45, default=None)
    started_epoch: Optional[int] = p_regular(46, default=None)
    paused_at: Optional[datetime] = p_regular(47, default=None)
    terminated_at: Optional[datetime] = p_regular(48, default=None)
    terminated_epoch: Optional[int] = p_regular(49, default=None)
    # attempts is populated if the first attempt is not successful

    # content
    inputs_packed: Any = p_value_packed(50)
    inputs_secret_packed: Any = p_secret_value_packed(51)
    inputs: Any = p_value_runtime(50, 51, typ=lambda self: cast("Run", self).input_type)
    outputs_packed: Any = p_value_packed(52)
    outputs_secret_packed: Any = p_secret_value_packed(53)
    outputs: Any = p_value_runtime(52, 53, typ=lambda self: cast("Run", self).output_type)
    value_packed: Any = p_value_packed(54)
    value_secret_packed: Any = p_secret_value_packed(55)
    value: Any = p_value_runtime(54, 55, typ=None)  # freely typed
    attempts: list[RetryAttempt] = p_internal(56, array=True, struct=StructType.RETRY_ATTEMPT)
    error: Optional["RunError"] = p_internal(
        57, default=None, require=False, array=False, struct=StructType.RUN_ERROR
    )

    # ...HasSessionContext[60-69]

    # NOTE :Architecture :Performance: (some) Runs will likely be stored outside the main user DB later.
    #  And maybe we'll also have 'inline runs' for non-Bench constructs that were run (like deeper profiling).
    runs: list["Run"] = p_node_child(NodeType.RUN)

    def __content_str__(self):
        if self.kind == RunKind.BLOCK:
            content_str = self.block.absolute_path if self.block else str(self.block_ptr)
        elif self.kind == RunKind.STEP:
            content_str = self.step.absolute_path if self.step else str(self.step_ptr)
        else:
            content_str = repr(self.code or self.text)
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
        assert self.status == RunStatus.RUNNING, f"cannot pause {self.status} run {self!r}"
        self.status = RunStatus.PAUSED
        self.paused_at = utcnow()

    def resume(self):
        """Resumes the Run."""
        assert self.status == RunStatus.PAUSED, f"cannot resume {self.status} run {self!r}"
        self.status = RunStatus.RUNNING
        self.paused_at = None

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
        assert not self.status.is_terminal, f"cannot fail {self.status} run {self!r}"
        self.status = RunStatus.FAILED
        self.error = error

    def _validate_component(
        self, properties: Collection[Property], invalid: ValidationHandler
    ) -> None:
        if self.block_ptr is None and self.code is None and self.text is None:
            invalid(self, "no block, code or text", (Run.block, Run.code, Run.text))
        if self.step_ptr is not None and self.block_ptr is None:
            invalid(self, "step without block", (Run.step, Run.block))
