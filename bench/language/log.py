from datetime import datetime
from typing import TYPE_CHECKING, Any, Optional

import structlog

from bench.language.const import (
    NODE_TYPES,
    AccessType,
    EnumType,
    NodeType,
    PrimitiveType,
    StructType,
    active_session,
    enum_,
)
from bench.language.node import (
    Node,
    RuntimeNode,
    Struct,
    struct_,
    timed_node_,
)
from bench.language.property import (
    p_internal,
    p_node_parent,
    p_system,
    p_value_packed,
    p_value_runtime,
)
from bench.language.session import HasSessionContext
from bench.proto.wire import LogData
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import (
        ChangeCategory,
        ChangeVignette,
        Edit,
        EditOperation,
        NodeReference,
        Package,
        Text,
    )

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)


@enum_(EnumType.LOG_KIND)
class LogKind(IdEnum):
    # access
    CHANGE = 1
    EDIT = 2

    # custom?


@enum_(EnumType.LOG_LEVEL)
class LogLevel(IdEnum):  # :LogLevel
    TRACE = 1
    DEBUG = 2
    INFO = 3
    WARNING = 4
    ERROR = 5
    CRITICAL = 6


@struct_(StructType.LOG_INFO)
class LogInfo(Struct):
    """
    A simple log for user-generated logs at runtime.
    This is like a mini-Log that we can attach to Runs and also copy into the combined Log.

    Conveniently, we can make this tiny because the containing Run already has all the context.

    NOTE :Incomplete: also capture edit events as mini logs?
    NOTE :Incomplete: copy Run.logs into Logs somewhere
    """

    created_at: datetime = p_internal(11, default_factory=lambda: active_session()._oracle.utc())
    level: LogLevel = p_internal(31)

    text: "Text | None" = p_internal(61, require=False, array=False, struct=StructType.TEXT)
    text_plain: str | None = p_internal(62)  # small optimization to avoid large Text instances
    value_packed: Any | None = p_value_packed(65)
    value: Any = p_value_runtime(65, typ=None)  # freeform value only


@timed_node_(NodeType.LOG)
class Log(RuntimeNode[LogData], HasSessionContext):
    """
    A Log of something happening in a Bench.
    """

    parent: "Package | None" = p_node_parent(4, NodeType.PACKAGE)

    # meta
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
        41, require=False, array=False, references=NODE_TYPES.tuple, same_bench=True
    )
    if TYPE_CHECKING:
        node_ptr: Optional[NodeReference] = None
    node_data: Any | None = p_system(43, primitive_type=PrimitiveType.JSON)
    operations: list["EditOperation"] = p_system(44, array=True, struct=StructType.EDIT_OPERATION)
    new_revision: int | None = p_system(47, primitive_type=PrimitiveType.INT64)
    category: Optional["ChangeCategory"] = p_system(48, require=False, array=False)
    vignette: Optional["ChangeVignette"] = p_system(
        49, require=False, array=False, struct=StructType.CHANGE_VIGNETTE
    )

    # context
    # ...HasSessionContext[70-89]

    def __content_str__(self):
        return f"[{self.kind.bench_name}:{self.level.bench_name}] ({self.created_at})"

    def to_edit(self) -> "Edit":
        raise NotImplementedError
