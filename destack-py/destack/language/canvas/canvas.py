from typing import TYPE_CHECKING

from destack.language.core import (
    Enum,
    EnumType,
    NodeType,
    builtin_enum,
    builtin_node,
    builtin_property,
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
class Canvas(ContainerView):
    """A Canvas is a container for only Shapes (other than that it's just a ContainerView)."""

    type: CanvasType = builtin_property(100, default=CanvasType.SHAPE)
