from datetime import datetime
from typing import TYPE_CHECKING, Any, Collection, Optional, Union, cast

from bench.language.code_ import Code
from bench.language.const import (
    TERMINAL_RUN_STATUSES,
    BenchError,
    NodeType,
    RunErrorKind,
    RunKind,
    RunStatus,
    StructType,
)
from bench.language.node import BasedNode, Node, Struct, node, struct
from bench.language.property import (
    Property,
    p_internal,
    p_node_ancestor_root,
    p_node_child,
    p_node_parent,
    p_secret_value_packed,
    p_system,
    p_value_packed,
    p_value_runtime,
)
from bench.language.session import HasSessionContext
from bench.language.text import Text
from bench.language.validation import ValidationHandler
from bench.language.value import HasValues
from bench.proto.wire import AnyNodeData, NodeReferenceData, RunData
from bench.utils.uuidt import UUIDT

if TYPE_CHECKING:
    from bench.language import Block, Package, Step

# pyright: reportIncompatibleVariableOverride=false


@node(
    NodeType.RUN,
    local=True,
    no_ck=True,  # no persistent identity
    id_factory=UUIDT,
    index_together=(("package_id", "created_at"),),
)
class Run(BasedNode[RunData], HasSessionContext, HasValues):
    """
    A 'run' of Blocks (and Steps within them) or 'lambdas' (just Code/Text).
    When 'running' something that's not directly runnable (like a Text Block, Text Step or Text Lambda),
     we implicitly pass it to the corresponding default Text program.
    Once terminated, a Run is effectively immutable.
    """

    # content
    parent: Union["Package", "Run"] = p_node_parent(4, NodeType.PACKAGE, NodeType.RUN)
    kind: RunKind = p_system(30)
    root: Optional["Run"] = p_node_ancestor_root(
        32, NodeType.RUN, require=False, store=True, wire=True, is_bench_implicit=True
    )

    if TYPE_CHECKING:
        session_ptr: Optional[NodeReferenceData] = None
        root_ptr: Optional[NodeReferenceData] = None
        server_ptr: Optional[NodeReferenceData] = None
        block_ptr: Optional[NodeReferenceData] = None
        step_ptr: Optional[NodeReferenceData] = None
    code: Optional["Code"] = p_internal(36, require=False, array=False, struct=StructType.CODE)
    text: Optional["Text"] = p_internal(37, require=False, array=False, struct=StructType.TEXT)

    # status
    status: RunStatus = p_internal(40, default=RunStatus.SCHEDULED)
    duration: Optional[float] = p_internal(41, default=None)
    scheduled_at: Optional[datetime] = p_internal(42, default=None)
    started_at: Optional[datetime] = p_internal(43, default=None)
    paused_at: Optional[datetime] = p_internal(44, default=None)
    terminated_at: Optional[datetime] = p_internal(45, default=None)

    # value
    inputs_packed: Any = p_value_packed(50)
    inputs_secret_packed: Any = p_secret_value_packed(51)
    inputs: Any = p_value_runtime(50, 51)
    outputs_packed: Any = p_value_packed(52)
    outputs_secret_packed: Any = p_secret_value_packed(53)
    outputs: Any = p_value_runtime(52, 53)
    value_packed: Any = p_value_packed(54)
    value_secret_packed: Any = p_secret_value_packed(55)
    value: Any = p_value_runtime(54, 55)
    error: Optional["RunError"] = p_internal(
        56, default=None, require=False, array=False, struct=StructType.RUN_ERROR
    )

    # context
    # ...InSessionNode[60-69]

    # NOTE :Architecture :Performance: (some) Runs will likely be stored outside the main user DB later.
    #  And maybe we'll also have 'inline runs' for non-Bench constructs that were run (like deeper profiling).
    runs: list["Run"] = p_node_child(NodeType.RUN)

    def __content_str__(self):
        value_keys_str = ", ".join(self.value.keys()) if self.value else ""
        return f"{self.block} ({self.status}, value={value_keys_str or '<none>'}, {self.id})"

    @property
    def active(self) -> bool:
        return self.status not in TERMINAL_RUN_STATUSES

    @property
    def base(self) -> Optional["Block"]:
        return self.block

    @staticmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        return cast(RunData, data).block_ptr

    def _validate_component(
        self, properties: Collection[Property], invalid: ValidationHandler
    ) -> None:
        if self.block_ptr is None and self.code is None and self.text is None:
            invalid(self, "no block, code or text", (Run.block, Run.code, Run.text))
        if self.step_ptr is not None and self.block_ptr is None:
            invalid(self, "step without block", (Run.step, Run.block))


@struct(StructType.RUN_CODE_FRAME)
class RunCodeFrame(Struct):
    node: Node = p_internal(30, array=False, require=True, references=NodeType.BLOCK)
    lineno: int = p_internal(31)
    name: str = p_internal(32)
    line: str = p_internal(33)


@struct(StructType.RUN_ERROR)
class RunError(Struct, BenchError):
    kind: RunErrorKind = p_internal(30)
    type: str = p_internal(31)
    message: Optional[str] = p_internal(32, default=None)
    node: Optional["Node"] = p_internal(33, require=False, array=False, references=NodeType.BLOCK)
    traceback: list[RunCodeFrame] = p_internal(34, array=True, struct=StructType.RUN_CODE_FRAME)

    @staticmethod
    def from_exception(e: Exception) -> "RunError":
        return RunError(
            kind=RunErrorKind.RUNTIME,
            type=type(e).__name__,
            message=str(e),
            traceback=[],
        )
