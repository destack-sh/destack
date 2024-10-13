from typing import TYPE_CHECKING, Any, Optional, Union, cast, final

import structlog

from bench.language.const import NodeType
from bench.language.field import TypeInfoBase
from bench.language.node import HasNodeBase, StateNode, local_node_
from bench.language.property import (
    p_internal,
    p_node_parent,
    p_system,
    p_value_packed,
    p_value_runtime,
)
from bench.proto.wire import AnyNodeData, NodeReferenceData, RecordData
from bench.utils.fractional import INTEGER_ZERO

if TYPE_CHECKING:
    from bench.language import Block, CustomObject

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)


@local_node_(NodeType.RECORD, passthrough="value", stored_custom=True, local=True)
class Record(StateNode[RecordData], HasNodeBase):
    """
    A Record from a DatabaseBlock.
    """

    # NOTE :Incomplete: support nested Records (Record.parent->Record)
    parent: Union["Block", None] = p_node_parent(4, NodeType.BLOCK)
    # type: RecordType?
    order_key: str | None = p_internal(33, default=INTEGER_ZERO)
    block: "Block" = p_system(34, require=True, references=NodeType.BLOCK)

    # value
    value_packed: Any = p_value_packed(40)
    value: "CustomObject | None" = p_value_runtime(
        40, typ=lambda self: cast("Record", self).value_type
    )

    @final
    def __repr__(self):  # type: ignore
        # override the default __repr__ for records
        block = self.block
        type_name = block.code_name if block is not None else "?Record"
        return f"<{type_name} {self!s}>"

    def __content_str__(self):
        value = self.value
        if value is not None:
            return str(value)
        else:
            return ""

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
