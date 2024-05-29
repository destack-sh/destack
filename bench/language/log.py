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
from bench.language.node import Node, timed_node
from bench.language.property import (
    p_internal,
    p_node_parent,
    p_secret_value_packed,
    p_system,
    p_value_packed,
    p_value_runtime,
)
from bench.language.session import HasSessionContext
from bench.language.text import Text
from bench.language.value import HasValues
from bench.utils.func import IdEnum, bittuple

if TYPE_CHECKING:
    from bench.language import Package

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)

SELF_LOGGED_NODE_TYPES: bittuple[NodeType] = bittuple(
    NodeType.SESSION, NodeType.RUN, NodeType.SIGNAL, NodeType.LOG
)


@enum_(EnumType.LOG_KIND)
class LogKind(IdEnum):
    # access
    READ = 1
    EDIT = 2
    USE = 3

    # custom
    CUSTOM = 10


@enum_(EnumType.LOG_LEVEL)
class LogLevel(IdEnum):
    TRACE = 1
    DEBUG = 2
    INFO = 3
    WARNING = 4
    ERROR = 5
    CRITICAL = 6


@timed_node(NodeType.LOG)
class Log(Node, HasSessionContext, HasValues):
    """
    A Log of something happening in a Bench.
    """

    parent: "Package" = p_node_parent(4, NodeType.PACKAGE)

    # meta
    kind: LogKind = p_system(30)
    level: LogLevel = p_system(31, default=LogLevel.INFO)

    # content (access)
    node: Optional["Node"] = p_system(
        40, require=False, array=False, references=NODE_TYPES.tuple, same_bench=True
    )
    type: AccessType | None = p_system(41, default=None)
    properties: list[int] = p_system(42, array=True)
    new_node_packed: Any | None = p_system(43, primitive_type=PrimitiveType.JSON)
    old_node_packed: Any | None = p_system(44, primitive_type=PrimitiveType.JSON)

    # content (custom)
    title: Optional[str] = p_internal(50, default=None)
    text: Optional[Text] = p_internal(
        51, default=None, require=False, array=False, struct=StructType.TEXT
    )
    value_packed: Any | None = p_value_packed(52)
    secret_value_packed: Any | None = p_secret_value_packed(53)
    value: Any = p_value_runtime(52, 53)

    # context
    # ...HasSessionContext[60-69]

    def __content_str__(self):
        return f"[{self.kind.bench_name}:{self.level.bench_name}] '{self.title or self.text or '<empty>'}' ({self.created_at})"
