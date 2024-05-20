from datetime import datetime
from typing import TYPE_CHECKING, Any, Optional, cast

from bench.language.const import (
    NodeType,
    NotificationKind,
    StructType,
)
from bench.language.node import BasedNode, node
from bench.language.property import (
    p_internal,
    p_node_parent,
    p_regular,
    p_secret_value_packed,
    p_system,
    p_value_packed,
    p_value_runtime,
)
from bench.language.session import HasSessionContext
from bench.language.value import HasValues
from bench.proto.wire import (
    AnyNodeData,
    NodeReferenceData,
    NotificationData,
)
from bench.utils.uuidt import UUIDT

if TYPE_CHECKING:
    from bench.language import Block, Package, Text

# pyright: reportIncompatibleVariableOverride=false


@node(
    NodeType.NOTIFICATION,
    passthrough="value",
    local=True,
    no_ck=True,  # no persistent identity
    id_factory=UUIDT,
    index_together=(("package_id", "created_at"),),
)
class Notification(BasedNode[NotificationData], HasSessionContext, HasValues):
    """
    A Notification for a Bench (author = created_by).
    """

    parent: "Package" = p_node_parent(4, NodeType.PACKAGE)
    kind: NotificationKind = p_regular(30)
    type: Optional["Block"] = p_system(32, require=False, array=False, references=NodeType.BLOCK)
    expires_at: Optional[datetime] = p_internal(33, default=None)
    read_at: Optional[datetime] = p_internal(34, default=None)

    # content
    title: Optional[str] = p_regular(40)
    text: Optional["Text"] = p_regular(41, require=False, array=False, struct=StructType.TEXT)
    value_packed: Any | None = p_value_packed(42)
    secret_value_packed: Any | None = p_secret_value_packed(43)
    value: Any = p_value_runtime(42, 43)

    # context
    # ...InSessionNode[60-69]

    @property
    def base(self) -> Optional["Block"]:
        return self.type

    @staticmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        return (cast(NotificationData, data)).type_ptr
