from typing import TYPE_CHECKING, Optional, final

from destack.core import (
    EnumDeclaration,
    EnumType,
    Float32,
    NodeType,
    StructFrozen,
    StructType,
    declare_entity,
    declare_enum,
    declare_property,
    declare_struct,
)

from .color import Color
from .style import Style

if TYPE_CHECKING:
    from destack import Axis2

# pyright: reportIncompatibleVariableOverride=false


@declare_enum(EnumType.GRADIENT_TYPE)
class GradientType(EnumDeclaration):
    """Built-in gradient types."""

    LINEAR = 10
    RADIAL = 11
    CONIC = 12


@declare_struct(
    StructType.GRADIENT_STOP,
    frozen=True,
    is_final=True,
)
@final
class GradientStop(StructFrozen):
    """A gradient stop with color and position."""

    color: Optional["Color"] = declare_property(101, is_repr=True)
    position: Float32 = declare_property(102, is_repr=True)


@declare_struct(
    StructType.GRADIENT,
    frozen=True,
    is_final=True,
)
@final
class Gradient(StructFrozen):
    """A gradient value."""

    type: GradientType = declare_property(100, default=GradientType.LINEAR, is_repr=True)
    style: Optional["GradientStyle"] = declare_property(101, is_repr=True)
    angle: Optional[Float32] = declare_property(102, is_repr=True)
    stops: list[GradientStop] = declare_property(103, is_repr=True)
    center_anchor: Optional["Axis2"] = declare_property(104, is_repr=True)


@declare_entity(NodeType.GRADIENT_STYLE)
class GradientStyle(Style):
    """A gradient style."""

    type: GradientType = declare_property(100, default=GradientType.LINEAR, is_repr=True)
    angle: Optional[Float32] = declare_property(102, is_repr=True)
    stops: list[GradientStop] = declare_property(103, is_repr=True)
    center_anchor: Optional["Axis2"] = declare_property(104, is_repr=True)
    dark: Gradient | None = declare_property(105)
