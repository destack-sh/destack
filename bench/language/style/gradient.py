from typing import Optional

from bench.language.core import (
    BuiltinEnum,
    BuiltinObject,
    EnumType,
    NodeType,
    Struct,
    StructType,
    enum_,
    node_,
    node_component_,
    p_regular,
    struct_,
)
from bench.pb2 import GradientStyleData

from .color import Color
from .core import Axis2
from .style import StyleBase


@enum_(EnumType.GRADIENT_TYPE)
class GradientType(BuiltinEnum):
    """Built-in gradient types."""

    STYLE = 2
    LINEAR = 10
    RADIAL = 11
    CONIC = 12


@struct_(StructType.GRADIENT_STOP)
class GradientStop(Struct):
    """A gradient stop with color and position."""

    color: "Color | None" = p_regular(50, require=False, struct=StructType.COLOR)
    position: float = p_regular(51)


@node_component_()
class GradientBase(BuiltinObject):
    type: GradientType = p_regular(30, default=GradientType.LINEAR)
    style: Optional["GradientStyle"] = p_regular(
        40, require=False, references=NodeType.GRADIENT_STYLE
    )
    angle: Optional[float] = p_regular(50, default=None)  # degrees
    stops: list[GradientStop] = p_regular(
        51, array=True, require=False, struct=StructType.GRADIENT_STOP
    )
    center_anchor: Optional[Axis2] = p_regular(
        52, require=False, array=False, default=None, struct=StructType.AXIS2
    )


@struct_(StructType.GRADIENT)
class Gradient(GradientBase, Struct):
    """A gradient value."""

    pass


@node_(NodeType.GRADIENT_STYLE)
class GradientStyle(GradientBase, StyleBase[GradientStyleData]):
    """A gradient style."""

    dark: Gradient | None = p_regular(60, require=False, array=False, struct=StructType.GRADIENT)
