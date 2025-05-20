from typing import TYPE_CHECKING, Union

from bench.language.core import (
    IsModal,
    IsNamed,
    IsTemplatable,
    PageNode,
    node_component_,
    p_node_parent,
)
from bench.pb2 import AnyNodeData

if TYPE_CHECKING:
    from bench.language import Page, Space, ViewBase

# pyright: reportIncompatibleVariableOverride=false


@node_component_()
class StyleBase[NodeDataT: AnyNodeData](
    IsTemplatable,
    IsModal,
    IsNamed,
    PageNode[NodeDataT],
):
    """A Style is a graphical interface."""

    parent: Union["Space", "ViewBase", "Page", None] = p_node_parent(4)
