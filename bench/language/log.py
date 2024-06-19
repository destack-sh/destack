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
from bench.proto.wire import EditData, LogData
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import NodeReference, Package

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

    # content (access)
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

    # content (custom)
    title: Optional[str] = p_internal(50, default=None)
    text: Optional[Text] = p_internal(
        51, default=None, require=False, array=False, struct=StructType.TEXT
    )
    value_packed: Any | None = p_value_packed(52)
    secret_value_packed: Any | None = p_secret_value_packed(53)
    value: Any = p_value_runtime(52, 53, typ=None)  # free type

    # context
    # ...HasSessionContext[60-69]

    def __content_str__(self):
        return f"[{self.kind.bench_name}:{self.level.bench_name}] '{self.title or self.text or '<empty>'}' ({self.created_at})"

    def to_edit(self) -> EditData:
        """Restores the edit of an access Log"""
        from bench.proto import wire, wiring

        assert self.kind == LogKind.EDIT, f"{self!r} is not an edit"
        assert self.type is not None, f"{self!r} has no type"
        assert self.node_ptr is not None, f"{self!r} has no node"
        assert self.new_revision is not None, f"{self!r} has no new revision"

        # NOTE :Incomplete: we ignore :SecretValues in edit Log for now
        old_node_packed = (
            wiring.pack_proto_json(self.old_node_packed)
            if self.old_node_packed is not None
            else None
        )
        new_node_packed = (
            wiring.pack_proto_json(self.new_node_packed)
            if self.new_node_packed is not None
            else None
        )
        edit = EditData(
            id=str(self.id),
            type=wire.EditType(self.type),
            node_ptr=self.node_ptr._to_data(),
            properties=self.properties,
            old_node_packed=old_node_packed,
            new_node_packed=new_node_packed,
            revision=self.new_revision,
            epoch=self.created_epoch,
            edited_at=self.created_at,
        )
        return edit
