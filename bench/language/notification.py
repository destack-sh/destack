from datetime import datetime
from typing import TYPE_CHECKING, Any, Optional, cast

from bench.language.const import (
    BlockType,
    NodeType,
    NotificationLevel,
    StructType,
)
from bench.language.field import TypeInfoBase
from bench.language.node import (
    HasNodeBase,
    RuntimeNode,
    Struct,
    object_,
    timed_node_,
)
from bench.language.property import (
    p_internal,
    p_node_parent,
    p_regular,
    p_system,
    p_value_packed,
    p_value_runtime,
)
from bench.language.validation import TITLE_CONSTRAINT, constraint
from bench.proto.wire import (
    AnyNodeData,
    NodeReferenceData,
    NotificationData,
)

if TYPE_CHECKING:
    from bench.language import Block, CustomObject, Package, Text

# pyright: reportIncompatibleVariableOverride=false


@timed_node_(NodeType.NOTIFICATION, passthrough_get="value", passthrough_set='value')
class Notification(RuntimeNode[NotificationData], HasNodeBase):
    """
    A Notification to a Bench (author = created_by).
    """

    parent: "Package | None" = p_node_parent(4, NodeType.PACKAGE)
    block: "Block" = p_system(
        32,
        require=True,
        array=False,
        references=NodeType.BLOCK,
        constraint=constraint(block_types=[BlockType.NOTIFICATION]),
    )
    level: NotificationLevel = p_regular(33)
    expires_at: Optional[datetime] = p_internal(34, default=None)
    read_at: Optional[datetime] = p_internal(35, default=None)

    # content
    title: Optional[str] = p_regular(40, constraint=TITLE_CONSTRAINT)
    text: Optional["Text"] = p_regular(41, require=False, array=False, struct=StructType.TEXT)
    value_packed: Any | None = p_value_packed(42)
    value: "CustomObject | None" = p_value_runtime(
        42, typ=lambda self: cast("Notification", self).value_type
    )

    # context
    # ...HasSessionContext[70-79]

    @property
    def value_type(self) -> "TypeInfoBase":
        typ = self.block.to_type(as_object=True)
        assert typ is not None, f"{self.block!r} has no type for {self!r}"
        return typ

    @property
    def base(self) -> Optional["Block"]:
        return self.block

    @staticmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        return (cast(NotificationData, data)).block_ptr


#
# Custom notification states
#


@object_()
class NotificationState(Struct):
    """Builtin special Value as the state of some specific notification type (in Notification.value)."""

    pass
