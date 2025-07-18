from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Enum,
    EnumType,
    NodeType,
    NumberFormat,
    StructFrozen,
    StructType,
    builtin_enum,
    builtin_node,
    builtin_property,
    builtin_struct,
)

from .color import Color
from .style import Style

if TYPE_CHECKING:
    from destack.language import Axis2

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

    color: Optional["Color"] = builtin_property(101, is_repr=True)
    position: float = builtin_property(102, format=NumberFormat.PERCENTAGE, is_repr=True)


@builtin_struct(StructType.GRADIENT, frozen=True)
class Gradient(StructFrozen):
    """A gradient value."""

    type: GradientType = builtin_property(100, default=GradientType.LINEAR, is_repr=True)
    style: Optional["GradientStyle"] = builtin_property(101, is_repr=True)
    angle: Optional[float] = builtin_property(102, format=NumberFormat.ANGLE, is_repr=True)
    stops: list[GradientStop] = builtin_property(103, is_repr=True)
    center_anchor: Optional["Axis2"] = builtin_property(104, is_repr=True)


@builtin_node(NodeType.GRADIENT_STYLE)
class GradientStyle(Style):
    """A gradient style."""

    type: GradientType = builtin_property(100, default=GradientType.LINEAR, is_repr=True)
    angle: Optional[float] = builtin_property(102, format=NumberFormat.ANGLE, is_repr=True)
    stops: list[GradientStop] = builtin_property(103, is_repr=True)
    center_anchor: Optional["Axis2"] = builtin_property(104, is_repr=True)
    dark: Gradient | None = builtin_property(105)
