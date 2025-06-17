from typing import TYPE_CHECKING, Union

from destack.language.core import (
    Enum,
    EnumType,
    HasIcon,
    IsOwnable,
    Node,
    NodeType,
    builtin_enum,
    builtin_node,
    property_,
    property_parent_,
)
from destack.proto import LayerProto

from ..view import ContainerView

if TYPE_CHECKING:
    from destack.language import Canvas, Scene

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.LAYER_TYPE)
class LayerType(Enum):
    """Built-in layer types."""

    GENERAL = 1
    SHAPE = 2
    # RASTER, ...


@builtin_node(NodeType.LAYER)
class Layer(
    ContainerView,
    HasIcon,
    IsOwnable,
    Node[LayerProto],
):
    """A Layer is a named container for Views."""

    parent: Union["Scene", "Canvas", None] = property_parent_(node_is_customizable=True)
    type: LayerType = property_(30, default=LayerType.GENERAL)
