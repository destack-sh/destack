from typing import TYPE_CHECKING, Union

from bench.language.core import (
    IsBlockable,
    IsModal,
    IsNamed,
    IsTemplatable,
    Node,
    p_node_parent,
)
from bench.language.core.const import NodeTrait
from bench.language.core.node import node_trait_
from bench.pb2 import AnyNodeData

if TYPE_CHECKING:
    from bench.language import Page, Space, ViewBase

# pyright: reportIncompatibleVariableOverride=false


@node_trait_(NodeTrait.STYLE)
class StyleBase[NodeDataT: AnyNodeData](
    IsTemplatable,
    IsModal,
    IsNamed,
    IsBlockable,
    Node if TYPE_CHECKING else object,
):
    """A Style is a graphical interface."""

    parent: Union["Space", "ViewBase", "Page", None] = p_node_parent(4)
