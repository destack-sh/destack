from destack.language.core import (
    IsCustomNode,
    IsCustomNodeDefinition,
    Node,
    NodeType,
    builtin_node,
    property_,
)
from destack.pb2 import CustomViewData, CustomViewDefinitionData

from .container import ContainerView

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.CUSTOM_VIEW_DEFINITION)
class CustomViewDefinition(
    ContainerView,
    IsCustomNodeDefinition,
    Node[CustomViewDefinitionData],
):
    """A definition for a custom View type."""

    pass


@builtin_node(NodeType.CUSTOM_VIEW)
class CustomView(
    ContainerView,
    IsCustomNode,
    Node[CustomViewData],
):
    definition: "CustomViewDefinition" = property_(
        6,
        description="The CustomViewDefinition this CustomView is an instance of.",
    )
