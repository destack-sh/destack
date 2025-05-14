from typing import TYPE_CHECKING, Optional, Union

from bench.language.core import (
    VIEW_NODE_TYPES,
    IsModal,
    IsNamed,
    IsTemplatable,
    Node,
    NodeType,
    PageNode,
    Selection,
    StructType,
    node_component_,
    p_node_parent,
    p_regular,
)
from bench.pb2 import AnyNodeData

if TYPE_CHECKING:
    from bench.language import Page, Space

# pyright: reportIncompatibleVariableOverride=false


#
# Views
#


@node_component_()
class ViewBase[NodeDataT: AnyNodeData](
    IsTemplatable,
    IsModal,
    IsNamed,
    PageNode[NodeDataT],
):
    """A View is a graphical interface."""

    parent: Union["Space", "ViewBase", "Page", None] = p_node_parent(
        4, NodeType.SPACE, *VIEW_NODE_TYPES.tuple, NodeType.PAGE
    )

    # behavior
    focus: Optional[Node] = p_regular(
        70, default=None, require=False, array=False, references="any"
    )
    selection: Optional[Selection] = p_regular(
        71, default=None, require=False, struct=StructType.SELECTION
    )
    ...  # actions/effects/...

    # flags
    # is_input: bool = p_regular(82, default=False)
    # is_inline: bool = p_regular(83, default=False)
    # is_minimal: bool = p_regular(84, default=False)
