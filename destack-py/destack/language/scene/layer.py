from typing import TYPE_CHECKING, Union

from destack.language.core import (
    BuiltinEnum,
    EnumType,
    HasIcon,
    IsOwnable,
    Node,
    NodeType,
    enum_,
    node_,
    property_,
    property_parent_,
)
from destack.pb2 import LayerData

from ..view import ContainerView

if TYPE_CHECKING:
    from destack.language import Scene

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.LAYER_TYPE)
class LayerType(BuiltinEnum):
    """Built-in layer types."""

    GENERAL = 1
    SHAPE = 2
    # RASTER, ...


@node_(NodeType.LAYER)
class Layer(
    ContainerView,
    HasIcon,
    IsOwnable,
    Node[LayerData],
):
    """A Layer is a named container for Views."""

    parent: Union["Scene", None] = property_parent_(node_is_customizable=True)
    type: LayerType = property_(30, default=LayerType.GENERAL)
