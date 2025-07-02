from typing import TYPE_CHECKING, Union

from destack.language.core import (
    Enum,
    EnumType,
    HasIcon,
    IsOwnable,
    NodeType,
    builtin_enum,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

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
class Layer(HasIcon, IsOwnable, ContainerView):
    """A Layer is a named container for Views."""

    parent: Union["Scene", "Canvas", None] = builtin_property_parent(node_is_extensible=True)
    type: LayerType = builtin_property(30, default=LayerType.GENERAL)
