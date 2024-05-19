from typing import TYPE_CHECKING, Any, Optional

import structlog

from bench.language.const import NODE_TYPES, AccessType, EnumType, NodeType, PrimitiveType, StructType, enum_
from bench.language.node import Node, node
from bench.language.property import (
    p_internal,
    p_node_parent,
    p_secret_value_packed,
    p_system,
    p_value_packed,
    p_value_runtime,
)
from bench.language.step import Step
from bench.language.text import Text
from bench.language.value import HasValues
from bench.utils.func import IdEnum
from bench.utils.uuidt import UUIDT

if TYPE_CHECKING:
    from bench.language import Block, Package, Run, Session, User, Client, Server

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)


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
    index_together=(("package_id", "created_at"),),
)
class Log(Node, HasValues):
    """
    A Log of something happening on a Bench.
    """

    parent: "Package" = p_node_parent(4, NodeType.PACKAGE)

    # meta
    kind: LogKind = p_system(30)
    level: LogLevel = p_system(31, default=LogLevel.INFO)
    logger: Optional[str] = p_system(32, default=None)
    event: Optional[str] = p_system(33, default=None)

    # content (access/edit)
    type: AccessType | None = p_internal(40, default=None)
    properties: list[int] = p_internal(41, array=True)
    new_node_packed: Any | None = p_internal(42, primitive_type=PrimitiveType.JSON)
    old_node_packed: Any | None = p_internal(43, primitive_type=PrimitiveType.JSON)

    # content (custom)
    title: Optional[str] = p_internal(45, default=None)
    text: Optional[Text] = p_internal(
        46, default=None, require=False, array=False, struct=StructType.TEXT
    )
    value_packed: Any | None = p_value_packed(47)
    secret_value_packed: Any | None = p_secret_value_packed(48)
    value: Any = p_value_runtime(47, 48)

    # context
    node: Optional["Node"] = p_system(50, require=False, array=False, references=NODE_TYPES.tuple)
    block: Optional["Block"] = p_system(51, require=False, array=False, references=NodeType.BLOCK)
    step: Optional["Step"] = p_system(52, require=False, array=False, references=NodeType.STEP)
    session: Optional["Session"] = p_system(
        55, require=False, array=False, references=NodeType.SESSION, is_bench_implicit=True
    )
    run: Optional["Run"] = p_system(
        56, require=False, array=False, references=NodeType.RUN, is_bench_implicit=True
    )
    client: Optional["Client"] = p_system(
        57, require=False, array=False, references=NodeType.CLIENT, is_bench_implicit=True
    )
    server: Optional["Server"] = p_system(
        58, require=False, array=False, references=NodeType.CLIENT, is_bench_implicit=True
    )
    user: Optional["User"] = p_system(59, require=False, array=False, references=NodeType.USER)

    def __content_str__(self):
        return f"[{self.kind.bench_name}:{self.level.bench_name}] '{self.event or self.title or self.text or '<empty>'}' ({self.created_at})"
