from typing import TYPE_CHECKING, Union

from bench.language.core import (
    IsBlockable,
    IsModal,
    IsNamed,
    IsTemplatable,
    Node,
    NodeType,
    node_,
    property_parent_,
)
from bench.pb2 import RoleData

if TYPE_CHECKING:
    from bench.language import Page, Team

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.ROLE)
class Role(IsTemplatable, IsModal, IsNamed, IsBlockable, Node[RoleData]):
    """A Role to assign to something."""

    parent: Union["Team", "Page", None] = property_parent_()
