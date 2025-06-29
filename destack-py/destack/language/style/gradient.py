from typing import Optional

from destack.language.core import (
    Axis2,
    Enum,
    EnumType,
    Node,
    NodeType,
    NumberFormat,
    StructFrozen,
    StructMutable,
    StructType,
    builtin_enum,
    builtin_node,
    builtin_struct,
    property_,
)

from .color import Color
from .style import Style

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.GRADIENT_TYPE)
class GradientType(Enum):
    """Built-in gradient types."""

    LINEAR = 10
    RADIAL = 11
    CONIC = 12


@builtin_struct(StructType.GRADIENT_STOP, frozen=True)
class GradientStop(StructFrozen):
    """A gradient stop with color and position."""

    color: Optional["Color"] = property_(50, is_repr=True)
    position: float = property_(51, format=NumberFormat.PERCENTAGE, is_repr=True)


@builtin_struct(StructType.GRADIENT)
class Gradient(StructMutable):
    """A gradient value."""

    type: GradientType = property_(30, default=GradientType.LINEAR, is_repr=True)
    style: Optional["GradientStyle"] = property_(40, is_repr=True)
    angle: Optional[float] = property_(50, format=NumberFormat.ANGLE, is_repr=True)
    stops: list[GradientStop] = property_(51, is_repr=True)
    center_anchor: Optional[Axis2] = property_(52, is_repr=True)


@builtin_node(NodeType.GRADIENT_STYLE)
class GradientStyle(
    Style,
    Node,
):
    """A gradient style."""

    type: GradientType = property_(30, default=GradientType.LINEAR, is_repr=True)
    angle: Optional[float] = property_(50, format=NumberFormat.ANGLE, is_repr=True)
    stops: list[GradientStop] = property_(51, is_repr=True)
    center_anchor: Optional[Axis2] = property_(52, is_repr=True)
    dark: Gradient | None = property_(60)
