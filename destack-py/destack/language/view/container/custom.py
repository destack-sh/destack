from destack.language.core import (
    IsCustomNode,
    IsCustomNodeDefinition,
    Node,
    NodeType,
    node_,
    property_,
)
from destack.pb2 import CustomViewData, CustomViewDefinitionData

from .container import IsContainerView

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.CUSTOM_VIEW_DEFINITION)
class CustomViewDefinition(
    IsCustomNodeDefinition,
    IsContainerView,
    Node[CustomViewDefinitionData],
):
    """A definition for a custom View type."""

    pass


@node_(NodeType.CUSTOM_VIEW)
class CustomView(
    IsCustomNode,
    IsContainerView,
    Node[CustomViewData],
):
    definition: "CustomViewDefinition" = property_(
        17,
        description="The CustomViewDefinition this CustomView is an instance of.",
    )
