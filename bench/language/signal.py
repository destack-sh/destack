from typing import TYPE_CHECKING, Any, Optional, cast

import structlog

from bench.language.const import BlockType, NodeType
from bench.language.field import TypeBase
from bench.language.node import HasNodeBase, RuntimeNode, timed_node_
from bench.language.property import p_internal, p_node_parent, p_value_packed, p_value_runtime
from bench.language.session import HasSessionContext
from bench.language.validation import constraint
from bench.proto.wire import AnyNodeData, NodeReferenceData, SignalData

if TYPE_CHECKING:
    from bench.language import Block, CustomObject, Package

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)

# NOTE :Architecture: should Signal be materialized like Records?
#  (although generally, we want to query Signals jointly more than Records?...)


@timed_node_(NodeType.SIGNAL, passthrough_get="value", passthrough_set="value")
class Signal(RuntimeNode[SignalData], HasNodeBase, HasSessionContext):
    """
    A Signal emitted in this Bench, usually received in Triggers.
    """

    parent: "Package | None" = p_node_parent(4, NodeType.PACKAGE)
    block: "Block" = p_internal(
        32,
        require=True,
        array=False,
        references=NodeType.BLOCK,
        constraint=constraint(block_types=[BlockType.SIGNAL]),
    )

    # content
    value_packed: Any | None = p_value_packed(42)
    value: "CustomObject | None" = p_value_runtime(
        42, typ=lambda self: cast("Signal", self).value_type
    )

    # context
    # ...HasSessionContext[70-89]

    @property
    def value_type(self) -> "TypeBase | None":
        return self.block.to_type(as_object=True)

    @property
    def base(self) -> Optional["Block"]:
        return self.block

    @staticmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        return (cast(SignalData, data)).block_ptr
