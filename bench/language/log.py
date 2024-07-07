from typing import TYPE_CHECKING, Any, Optional

import structlog

from bench.language.const import (
    NODE_TYPES,
    AccessType,
    EnumType,
    NodeType,
    PrimitiveType,
    StructType,
    enum_,
)
from bench.language.node import HasTimeIdentity, Node, PackageNode, timed_node
from bench.language.property import p_node_parent, p_system
from bench.language.session import HasSessionContext
from bench.language.value import HasValues
from bench.proto.wire import LogData
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import ChangeCategory, ChangeVignette, Edit, NodeReference, Package

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


@timed_node(NodeType.LOG)
class Log(PackageNode[LogData], HasTimeIdentity, HasSessionContext, HasValues):
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

    # content
    type: AccessType | None = p_system(40, default=None)
    node: Optional["Node"] = p_system(
        41, require=False, array=False, references=NODE_TYPES.tuple, same_bench=True
    )
    if TYPE_CHECKING:
        node_ptr: Optional[NodeReference] = None
    properties: list[int] = p_system(42, array=True, primitive_type=PrimitiveType.INT16)
    old_node_packed: Any | None = p_system(43, primitive_type=PrimitiveType.JSON)
    old_node_secret_packed: Any | None = p_system(
        44, primitive_type=PrimitiveType.JSON, encrypt=True, sensitive=True, defer=True
    )
    new_node_packed: Any | None = p_system(45, primitive_type=PrimitiveType.JSON)
    new_node_secret_packed: Any | None = p_system(
        46, primitive_type=PrimitiveType.JSON, encrypt=True, sensitive=True, defer=True
    )
    new_revision: int | None = p_system(47, primitive_type=PrimitiveType.INT64)
    category: Optional["ChangeCategory"] = p_system(48, require=False, array=False)
    vignette: Optional["ChangeVignette"] = p_system(
        49, require=False, array=False, struct=StructType.CHANGE_VIGNETTE
    )

    # context
    # ...HasSessionContext[70-79]

    def __content_str__(self):
        return f"[{self.kind.bench_name}:{self.level.bench_name}] ({self.created_at})"

    def to_edit(self) -> "Edit":
        raise NotImplementedError
