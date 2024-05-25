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
from bench.language.node import Node, node
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
from bench.utils.func import IdEnum
from bench.utils.uuidt import UUIDT

if TYPE_CHECKING:
    from bench.language import Package

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)

# we don't want edits to core runtime types to trigger logs/signals (circular, and very noisy)
MUTED_NODE_TYPES: tuple[NodeType, ...] = (
    NodeType.SESSION,
    NodeType.RUN,
    NodeType.SIGNAL,
    NodeType.LOG,
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


@node(
    NodeType.LOG,
    local=True,
    no_ck=True,  # no persistent identity
    id_factory=UUIDT,
    index_together=(
        ("epoch",),
        ("package_id", "created_at"),
    ),
)
class Log(Node, HasSessionContext, HasValues):
    """
    A Log of something happening on a Bench.
    """

    parent: "Package" = p_node_parent(4, NodeType.PACKAGE)

    # meta
    kind: LogKind = p_system(30)
    level: LogLevel = p_system(31, default=LogLevel.INFO)
    epoch: Optional[int] = p_internal(32, default=None, primitive_type=PrimitiveType.INT64)

    # content (access)
    node: Optional["Node"] = p_system(
        40, require=False, array=False, references=NODE_TYPES.tuple, is_bench_implicit=True
    )
    type: AccessType | None = p_internal(41, default=None)
    properties: list[int] = p_internal(42, array=True)
    new_node_packed: Any | None = p_internal(43, primitive_type=PrimitiveType.JSON)
    old_node_packed: Any | None = p_internal(44, primitive_type=PrimitiveType.JSON)

    # content (custom)
    title: Optional[str] = p_internal(45, default=None)
    text: Optional[Text] = p_internal(
        46, default=None, require=False, array=False, struct=StructType.TEXT
    )
    value_packed: Any | None = p_value_packed(47)
    secret_value_packed: Any | None = p_secret_value_packed(48)
    value: Any = p_value_runtime(47, 48)

    # context
    # ...InSessionNode[60-69]

    def __content_str__(self):
        return f"[{self.kind.bench_name}:{self.level.bench_name}] '{self.title or self.text or '<empty>'}' ({self.created_at})"
