from typing import TYPE_CHECKING, Any, Optional, cast, final

import structlog

from bench.language.const import NodeType, ObjectKind, StructType
from bench.language.field import TypeBase
from bench.language.node import HasNodeBase, StateNode, node_
from bench.language.property import (
    p_internal,
    p_node_parent,
    p_regular,
    p_system,
    p_value_packed,
    p_value_runtime,
)
from bench.proto.wire import AnyNodeData, NodeReferenceData, RecordData
from bench.utils.fractional import INTEGER_ZERO

if TYPE_CHECKING:
    from bench.language import Bench, Block, CustomObject, Icon, Text

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)


@node_(
    NodeType.RECORD,
    passthrough_get=("value",),
    passthrough_set=("value",),
    stored_value_unraveled=True,
)
class Record(StateNode[RecordData], HasNodeBase):
    """
    A Record from a DatabaseBlock. May references other Records (except for :ManyToManyRecords).
    """

    # NOTE :Incomplete: support nested Records? (Record.parent->Record)
    parent: Optional["Bench"] = p_node_parent(4, NodeType.BENCH)
    # type: RecordType?
    title: Optional[str] = p_regular(32, default=None)
    order_key: str | None = p_internal(33, default=INTEGER_ZERO)
    icon: Optional["Icon"] = p_regular(34, default=None, struct=StructType.ICON)
    text: Optional["Text"] = p_regular(
        35, default=None, require=False, array=False, struct=StructType.TEXT
    )
    block: "Block" = p_system(36, require=True, references=NodeType.BLOCK)

    # value
    value_packed: Any = p_value_packed(40)
    value: "CustomObject | None" = p_value_runtime(
        40, kind=ObjectKind.MEMBER, typ=lambda self: cast("Record", self).value_type
    )

    @final
    def __repr__(self):  # type: ignore
        # override the default __repr__ for records
        block = self.block
        type_name = block.code_name if block is not None else "?"
        return f"<{type_name}Record {self!s}>"

    def __content_str__(self):
        value = self.value
        if value is not None:
            return str(value)
        else:
            return ""

    @property
    def value_type(self) -> "TypeBase | None":
        block = self.block
        return block.to_type_maybe(of="value") if block is not None else None

    @property
    def base(self) -> "Block | None":
        return self.block

    @staticmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        return cast(RecordData, data).block_ptr
