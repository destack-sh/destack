from typing import TYPE_CHECKING, Any, Optional, Union, cast

import structlog

from bench.language.const import NodeType
from bench.language.field import TypeInfoBase
from bench.language.node import HasNodeBase, StateNode, local_node_
from bench.language.property import (
    p_internal,
    p_node_ancestor,
    p_node_parent,
    p_value_packed,
    p_value_runtime,
)
from bench.proto.wire import AnyNodeData, NodeReferenceData, RecordData
from bench.utils.fractional import INTEGER_ZERO
from bench.utils.func import describe_type

if TYPE_CHECKING:
    from bench.language import Block, TypeInfo, ValueObject

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)


@local_node_(NodeType.RECORD, passthrough="value", stored_custom=True, local=True)
class Record(StateNode[RecordData], HasNodeBase):
    """
    A Record from a DatabaseBlock.
    """

    parent: Union["Block", "Record", None] = p_node_parent(4, NodeType.BLOCK, NodeType.RECORD)
    # type: RecordType?
    order_key: str | None = p_internal(33, default=INTEGER_ZERO)
    block: "Block" = p_node_ancestor(34, NodeType.BLOCK, wire=True)

    # value
    value_packed: Any = p_value_packed(40)
    value: "ValueObject | None" = p_value_runtime(
        40, typ=lambda self: cast("Record", self).value_type
    )

    def __content_str__(self):
        return f"{describe_type(self.value) or '<empty>'}"

    @property
    def value_type(self) -> "TypeInfoBase | None":
        block = self.block
        return block.to_type(as_object=True) if block is not None else None

    @property
    def base(self) -> "Block | None":
        return self.block

    @staticmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        return cast(RecordData, data).block_ptr

    @property
    def _type(self) -> "TypeInfo":
        return getattr(self.parent, "as_type")
