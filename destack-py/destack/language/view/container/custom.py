from typing import TYPE_CHECKING, Optional, Union

from destack.language.core import (
    IsCustomNode,
    IsCustomNodeDefinition,
    Node,
    NodeType,
    builtin_node,
    property_,
    property_parent_,
)
from destack.proto import CustomViewDefinitionProto, CustomViewProto

from .container import ContainerView

if TYPE_CHECKING:
    from destack.language import Folder, Scene

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.CUSTOM_VIEW_DEFINITION)
class CustomViewDefinition(
    ContainerView,
    IsCustomNodeDefinition,
    Node[CustomViewDefinitionProto],
):
    """A definition for a custom View type."""

    parent: Union["Folder", "Scene", None] = property_parent_(node_is_customizable=False)
    prototype: Optional["CustomView"] = property_(
        6,
        description="A custom View's prototype is the default template new CustomView instances are based on.",
    )


@builtin_node(NodeType.CUSTOM_VIEW)
class CustomView(
    ContainerView,
    IsCustomNode,
    Node[CustomViewProto],
):
    definition: "CustomViewDefinition" = property_(
        6,
        description="The CustomViewDefinition this CustomView is an instance of.",
        is_managed=True,
        can_write=None,
    )
