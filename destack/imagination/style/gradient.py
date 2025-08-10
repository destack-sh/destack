from typing import TYPE_CHECKING, Optional, final

from destack.core import (
    EnumType,
    Float32,
    NodeType,
    OptionEnum,
    Struct,
    StructType,
    declare_entity,
    declare_enum,
    declare_option,
    declare_property,
    declare_struct,
)

from .color import Color
from .style import Style

if TYPE_CHECKING:
    from destack import Axis2


@declare_enum(EnumType.GRADIENT_TYPE)
class GradientType(OptionEnum):
    """Built-in gradient types."""

    LINEAR = declare_option(10, "Linear", description="A linear gradient")
    RADIAL = declare_option(11, "Radial", description="A radial gradient")
    CONIC = declare_option(12, "Conic", description="A conic gradient")
    DIAMOND = declare_option(13, "Diamond", description="A diamond gradient")


@declare_struct(
    StructType.GRADIENT_STOP,
    is_final=True,
)
@final
class GradientStop(Struct):
    """A gradient stop with color and position."""

    color: Optional["Color"] = declare_property(101, is_repr=True)
    position: Float32 = declare_property(102, is_repr=True)


@declare_struct(
    StructType.GRADIENT,
    is_final=True,
    into_node_types=(NodeType.GRADIENT_STYLE,),
)
@final
class Gradient(Struct):
    """A gradient value."""

    type: GradientType = declare_property(100, default=GradientType.LINEAR, is_repr=True)
    template: Optional["GradientStyle"] = declare_property(101, is_repr=True)
    angle: Optional[Float32] = declare_property(102, is_repr=True)
    stops: list[GradientStop] = declare_property(103, is_repr=True)
    center_anchor: Optional["Axis2"] = declare_property(104, is_repr=True)


@declare_entity(
    NodeType.GRADIENT_STYLE,
    base_struct_type=StructType.GRADIENT,
)
class GradientStyle(Style):
    """A gradient style."""

    type: GradientType = declare_property(100, default=GradientType.LINEAR, is_repr=True)
    angle: Optional[Float32] = declare_property(102, is_repr=True)
    stops: list[GradientStop] = declare_property(103, is_repr=True)
    center_anchor: Optional["Axis2"] = declare_property(104, is_repr=True)
    dark: Gradient | None = declare_property(105)
