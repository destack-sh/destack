from typing import TYPE_CHECKING, Any, Optional, cast

import structlog

from bench.language.const import NodeType
from bench.language.node import BasedNode, node
from bench.language.property import (
    p_internal,
    p_node_parent,
    p_secret_value_packed,
    p_value_packed,
    p_value_runtime,
)
from bench.language.value import HasValues
from bench.proto.wire import AnyNodeData, NodeReferenceData, SignalData
from bench.utils.uuidt import UUIDT

if TYPE_CHECKING:
    from bench.language import Block, Package

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)


@node(NodeType.SIGNAL, passthrough="value", local=True, id_factory=UUIDT)
class Signal(BasedNode[SignalData], HasValues):
    """A signal emitted in this Bench."""

    parent: "Package" = p_node_parent(4, NodeType.PACKAGE)
    # builtin_type: ...
    type: Optional["Block"] = p_internal(
        31, require=False, array=False, references=NodeType.BLOCK, index_in_pg=True
    )
    origin: Optional["Block"] = p_internal(
        33, require=False, array=False, references=NodeType.BLOCK, index_in_pg=True
    )
    value_packed: Any | None = p_value_packed(34)
    secret_value_packed: Any | None = p_secret_value_packed(35)
    value: Any = p_value_runtime(34, 35, type=31)

    @property
    def base(self) -> Optional["Block"]:
        return self.type

    @staticmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]:
        return (cast(SignalData, data)).type_ptr
