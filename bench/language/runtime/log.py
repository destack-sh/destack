from datetime import datetime
from typing import TYPE_CHECKING, Any, Optional

import structlog

from bench.language.core import (
    AccessType,
    LogKind,
    LogLevel,
    Node,
    NodeType,
    PrimitiveType,
    RuntimeNode,
    Struct,
    StructType,
    active_session,
    p_internal,
    p_node_parent,
    p_system,
    struct_,
    timed_node_,
)
from bench.pb2 import LogData

if TYPE_CHECKING:
    from bench.language import (
        Bench,
        ChangeCategory,
        ChangeVignette,
        Edit,
        EditOperation,
        NodeReference,
        Text,
    )

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)


@struct_(StructType.LOG_INFO)
class LogInfo(Struct):
    """
    A simple log for user-generated logs at runtime.
    This is like a mini-Log that we can attach to Runs and also copy into the combined Log.

    Conveniently, we can make this tiny because the containing Run already has all the context.
    """

    # NOTE :Architecture: LogInfo probably shouldn't exist and should just be full Logs

    created_at: datetime = p_internal(11, default_factory=lambda: active_session()._oracle.utc())
    level: LogLevel = p_internal(31)

    text: "Text | None" = p_internal(61, require=False, array=False, struct=StructType.TEXT)
    text_plain: str | None = p_internal(62)  # small optimization to avoid large Text instances


@timed_node_(NodeType.LOG)
class Log(RuntimeNode[LogData]):
    """
    A Log of something happening in a Bench.
    """

    # meta
    parent: Optional["Bench"] = p_node_parent(4, NodeType.BENCH)
    kind: LogKind = p_system(30)
    level: LogLevel = p_system(31, default=LogLevel.INFO)
    change: Optional["Log"] = p_system(
        32, require=False, array=False, references=NodeType.LOG, same_bench=True
    )
    undo_of: Optional["Log"] = p_system(
        33, require=False, array=False, references=NodeType.LOG, same_bench=True
    )

    # content
    type: AccessType | None = p_system(40, default=None)
    node: Optional["Node"] = p_system(
        41, require=False, array=False, references="any", same_bench=True
    )
    if TYPE_CHECKING:
        node_ptr: Optional[NodeReference] = None
    node_data: Any | None = p_system(43, primitive_type=PrimitiveType.JSON)
    operations: list["EditOperation"] = p_system(44, array=True, struct=StructType.EDIT_OPERATION)
    category: Optional["ChangeCategory"] = p_system(48, require=False, array=False)
    vignette: Optional["ChangeVignette"] = p_system(
        49, require=False, array=False, struct=StructType.CHANGE_VIGNETTE
    )

    # context
    # ...HasRuntimeContext[80-99]

    def __content_str__(self):
        return f"[{self.kind.bench_name}:{self.level.bench_name}] ({self.created_at})"

    def to_edit(self) -> "Edit":
        raise NotImplementedError
