from typing import TYPE_CHECKING, Union

from destack.language.core import (
    Enum,
    EnumType,
    IsOwnable,
    NodeType,
    builtin_enum,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

from ..view import ContainerView

if TYPE_CHECKING:
    from destack.language import Canvas, Icon, Scene

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.LAYER_TYPE)
class LayerType(Enum):
    """Built-in layer types."""

    GENERAL = 1
    SHAPE = 2
    # RASTER, ...


@builtin_node(NodeType.LAYER)
class Layer(IsOwnable, ContainerView):
    """A Layer is a named container for Views."""

    parent: Union["Scene", "Canvas", None] = builtin_property_parent(node_is_extensible=True)
    type: LayerType = builtin_property(100, default=LayerType.GENERAL)
    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)
