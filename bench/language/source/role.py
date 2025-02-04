from typing import TYPE_CHECKING, Union

from bench.language.core import InlineSourceNode, NodeType, node_, p_node_parent
from bench.pb2 import RoleData

if TYPE_CHECKING:
    from bench.language import Page

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.ROLE)
class Role(InlineSourceNode[RoleData]):
    """A Role to assign to something."""

    parent: Union["Page", None] = p_node_parent(4, NodeType.PAGE)
