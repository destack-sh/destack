from typing import TYPE_CHECKING, Union

from destack.pb2 import OptionData

from ..builtin import (
    Entity,
    HasIcon,
    HasName,
    IsDeletable,
    IsSourceable,
    IsTaggable,
    IsTemplatable,
    Node,
    NodeType,
    Spatial,
    builtin_node,
    property_parent_,
)

if TYPE_CHECKING:
    from destack.language import Field, Schema

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.OPTION)
class Option(
    Spatial,
    Entity,
    HasName,
    HasIcon,
    IsTaggable,
    IsTemplatable,
    IsDeletable,
    IsSourceable,
    Node[OptionData],
):
    parent: Union["Schema", "Field", None] = property_parent_(node_is_customizable=True)
