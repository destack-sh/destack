from typing import TYPE_CHECKING, Any, Optional, cast

import structlog

from bench.language.const import BlockType, NodeType
from bench.language.field import TypeInfoBase
from bench.language.node import (
    HasNodeBase,
    HasTimeIdentity,
    PackageNode,
    Struct,
    object_component,
    timed_node_,
)
from bench.language.property import (
    p_internal,
    p_node_parent,
    p_value_packed,
    p_value_runtime,
)
from bench.language.session import HasSessionContext
from bench.language.validation import constraint
from bench.language.value import HasValues
from bench.proto.wire import AnyNodeData, NodeReferenceData, SignalData

if TYPE_CHECKING:
    from bench.language import Block, Package, ValueObject

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)

# NOTE :Architecture: should Signal be materialized like Records?
#  (although generally, we want to query Signals together more than Records...)


@timed_node_(NodeType.SIGNAL, passthrough="value")
class Signal(PackageNode[SignalData], HasTimeIdentity, HasNodeBase, HasSessionContext, HasValues):
    """
    A Signal emitted in this Bench, usually received in Triggers.
    """

    parent: "Package | None" = p_node_parent(4, NodeType.PACKAGE)
    type: "Block" = p_internal(
        32,
        require=True,
        array=False,
        references=NodeType.BLOCK,
        constraint=constraint(block_type=BlockType.SIGNAL),
    )

    # content
    value_packed: Any | None = p_value_packed(42)
    value: "ValueObject | None" = p_value_runtime(
        42, typ=lambda self: cast("Signal", self).value_type
    )

    # context
    # ...HasSessionContext[70-79]

    @property
    def value_type(self) -> "TypeInfoBase | None":
        return self.type.to_type(as_object=True)

    @property
    def base(self) -> Optional["Block"]:
        return self.type

    @staticmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        return (cast(SignalData, data)).type_ptr


#
# Custom signal states
#


@object_component()
class SignalState(Struct):
    """Builtin special Value as the state of some specific signal type (in Signal.value)."""

    pass
