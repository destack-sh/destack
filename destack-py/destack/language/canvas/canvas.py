from typing import TYPE_CHECKING

from destack.language.core import (
    Enum,
    EnumType,
    Node,
    NodeType,
    builtin_enum,
    builtin_node,
    property_,
)

from ..view import ContainerView

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.CANVAS_TYPE)
class CanvasType(Enum):
    """Built-in canvas types."""

    SHAPE = 1
    # RASTER, ...


@builtin_node(NodeType.CANVAS, pretend_frozen=True)
class Canvas(ContainerView, Node):
    """A Canvas is a container for only Shapes (other than that it's just a ContainerView)."""

    type: CanvasType = property_(30, default=CanvasType.SHAPE)
