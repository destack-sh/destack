from typing import TYPE_CHECKING, Any, Optional, cast

import structlog

from bench.language.const import NodeType
from bench.language.field import TypeInfoBase
from bench.language.node import HasNodeBase, HasPersistentIdentity, PackageNode, node_
from bench.language.property import (
    p_node_parent,
    p_secret_value_packed,
    p_value_packed,
    p_value_runtime,
)
from bench.language.value import HasValues
from bench.proto.wire import AnyNodeData, NodeReferenceData, RecordData
from bench.utils.func import describe_type

if TYPE_CHECKING:
    from bench.language import Block, TypeInfo, ValueObject

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)


@node_(NodeType.RECORD, passthrough="value", stored_custom=True, local=True)
class Record(PackageNode[RecordData], HasPersistentIdentity, HasNodeBase, HasValues):
    """
    A record in a DatabaseBlock.
    If the block is_materialized, the backing table is a real Postgres table.
    """

    # :RecordSchema
    parent: "Block" = p_node_parent(4, NodeType.BLOCK)
    value_packed: Any = p_value_packed(30)
    secret_value_packed = p_secret_value_packed(31)
    value: "ValueObject | None" = p_value_runtime(
        30, 31, typ=lambda self: cast("Record", self).value_type
    )

    def __content_str__(self):
        return f"{describe_type(self.value) or '<empty>'}"

    @property
    def value_type(self) -> "TypeInfoBase | None":
        return self.parent.to_type(as_object=True)

    @property
    def base(self) -> "Block":
        return self.parent

    @staticmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        return data.parent_ptr

    @property
    def _type(self) -> "TypeInfo":
        return getattr(self.parent, "as_type")
