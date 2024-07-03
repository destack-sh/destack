from datetime import datetime
from typing import TYPE_CHECKING, Any, Optional, cast

from bench.language.const import (
    NodeType,
    NotificationKind,
    StructType,
)
from bench.language.field import TypeInfoBase
from bench.language.node import (
    HasNodeBase,
    HasPersistentIdentity,
    HasTimeIdentity,
    InlineStruct,
    PackageNode,
    object_component,
    timed_node,
)
from bench.language.property import (
    p_internal,
    p_node_parent,
    p_regular,
    p_secret_value_packed,
    p_system,
    p_value_packed,
    p_value_runtime,
)
from bench.language.validation import TITLE_CONSTRAINT
from bench.language.value import HasValues
from bench.proto.wire import (
    AnyNodeData,
    NodeReferenceData,
    NotificationData,
)

if TYPE_CHECKING:
    from bench.language import Block, Package, Text, ValueObject

# pyright: reportIncompatibleVariableOverride=false


@timed_node(NodeType.NOTIFICATION, passthrough="value")
class Notification(
    PackageNode[NotificationData],
    HasTimeIdentity,
    HasPersistentIdentity,
    HasNodeBase,
    HasValues,
):
    """
    A Notification for a Bench (author = created_by).
    """

    parent: "Package | None" = p_node_parent(4, NodeType.PACKAGE)
    kind: NotificationKind = p_regular(30)
    type: "Block" = p_system(32, require=True, array=False, references=NodeType.BLOCK)
    expires_at: Optional[datetime] = p_internal(33, default=None)
    read_at: Optional[datetime] = p_internal(34, default=None)

    # content
    title: Optional[str] = p_regular(40, constraint=TITLE_CONSTRAINT)
    text: Optional["Text"] = p_regular(41, require=False, array=False, struct=StructType.TEXT)
    value_packed: Any | None = p_value_packed(42)
    secret_value_packed: Any | None = p_secret_value_packed(43)
    value: "ValueObject | None" = p_value_runtime(
        42, 43, typ=lambda self: cast("Notification", self).value_type
    )

    # context
    # ...HasSessionContext[70-79]

    @property
    def value_type(self) -> "TypeInfoBase":
        typ = self.type.to_type(as_object=True)
        assert typ is not None, f"{self.type!r} has no type for {self!r}"
        return typ

    @property
    def base(self) -> Optional["Block"]:
        return self.type

    @staticmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        return (cast(NotificationData, data)).type_ptr


#
# Custom notification states
#


@object_component()
class NotificationState(InlineStruct):
    """Builtin special Value as the state of some specific notification type (in Notification.value)."""

    pass
