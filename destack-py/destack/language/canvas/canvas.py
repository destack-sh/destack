from typing import TYPE_CHECKING

from destack.language.core import (
    BuiltinEnum,
    EnumType,
    Node,
    NodeType,
    enum_,
    node_,
    property_,
)
from destack.pb2 import CanvasData

from ..view import ContainerView

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.CANVAS_TYPE)
class CanvasType(BuiltinEnum):
    """Built-in canvas types."""

    SHAPE = 1
    # RASTER, ...


@node_(NodeType.CANVAS, pretend_frozen=True)
class Canvas(ContainerView, Node[CanvasData]):
    """A Canvas is a container for only Shapes (other than that it's just a ContainerView)."""

    type: CanvasType = property_(30, default=CanvasType.SHAPE)
