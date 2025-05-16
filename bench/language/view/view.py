from typing import TYPE_CHECKING, Optional, Union

from bench.language.core import (
    VIEW_NODE_TYPES,
    IsInstantiable,
    IsModal,
    IsNamed,
    NodeType,
    PageNode,
    node_component_,
    p_node_parent,
    p_regular,
)
from bench.pb2 import AnyNodeData

if TYPE_CHECKING:
    from bench.language import Dimension, Page, Position, Space

# pyright: reportIncompatibleVariableOverride=false


@node_component_()
class ViewBase[NodeDataT: AnyNodeData](
    IsInstantiable,
    IsModal,
    IsNamed,
    PageNode[NodeDataT],
):
    """A View is a graphical interface."""

    parent: Union["Space", "ViewBase", "Page", None] = p_node_parent(
        4, NodeType.SPACE, *VIEW_NODE_TYPES.tuple, NodeType.PAGE
    )
    # variant_of, ...

    # sizing
    position: Optional["Position"] = p_regular(40)
    width: Optional["Dimension"] = p_regular(41)
    height: Optional["Dimension"] = p_regular(42)
    min_width: Optional["Dimension"] = p_regular(43)
    min_height: Optional["Dimension"] = p_regular(44)
    max_width: Optional["Dimension"] = p_regular(45)
    max_height: Optional["Dimension"] = p_regular(46)
