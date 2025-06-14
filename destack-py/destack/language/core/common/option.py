from typing import TYPE_CHECKING, Union

from destack.pb2 import OptionData

from ..builtin import (
    Entity,
    HasIcon,
    HasName,
    IsDeletable,
    IsSourceable,
    IsTaggable,
    Node,
    NodeType,
    Spatial,
    builtin_node,
    property_parent_,
)

if TYPE_CHECKING:
    from destack.language import CustomStructDefinition, Field

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.OPTION)
class Option(
    Spatial,
    Entity,
    HasName,
    HasIcon,
    IsTaggable,
    IsDeletable,
    IsSourceable,
    Node[OptionData],
):
    parent: Union["CustomStructDefinition", "Field", None] = property_parent_(
        node_is_customizable=True
    )
