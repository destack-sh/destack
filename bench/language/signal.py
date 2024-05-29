from typing import TYPE_CHECKING, Any, Optional, cast

import structlog

from bench.language.const import NodeType
from bench.language.node import BasedNode, timed_node
from bench.language.property import (
    p_internal,
    p_node_parent,
    p_secret_value_packed,
    p_value_packed,
    p_value_runtime,
)
from bench.language.session import HasSessionContext
from bench.language.value import HasValues
from bench.proto.wire import AnyNodeData, NodeReferenceData, SignalData

if TYPE_CHECKING:
    from bench.language import Block, Package

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)

# NOTE :Architecture: should Signal be materialized like Records?
#  (although generally, we want to query Signals together more than Records...)


@timed_node(NodeType.SIGNAL, passthrough="value")
class Signal(BasedNode[SignalData], HasSessionContext, HasValues):
    """
    A Signal emitted in this Bench, usually received in Triggers.
    """

    parent: "Package" = p_node_parent(4, NodeType.PACKAGE)
    type: Optional["Block"] = p_internal(32, require=False, array=False, references=NodeType.BLOCK)

    # content
    value_packed: Any | None = p_value_packed(42)
    secret_value_packed: Any | None = p_secret_value_packed(43)
    value: Any = p_value_runtime(42, 43, type=32)

    # context
    # ...HasSessionContext[60-69]

    @property
    def base(self) -> Optional["Block"]:
        return self.type

    @staticmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        return (cast(SignalData, data)).type_ptr
