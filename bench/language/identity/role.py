from typing import TYPE_CHECKING, Union

from bench.language.core import (
    IsModal,
    IsNamed,
    IsTemplatable,
    NodeType,
    PageNode,
    node_,
    p_node_parent,
)
from bench.pb2 import RoleData

if TYPE_CHECKING:
    from bench.language import Page, Team

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.ROLE)
class Role(IsTemplatable, IsModal, IsNamed, PageNode[RoleData]):
    """A Role to assign to something."""

    parent: Union["Team", "Page", None] = p_node_parent(4, NodeType.TEAM, NodeType.PAGE)
