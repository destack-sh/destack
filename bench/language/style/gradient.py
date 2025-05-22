from typing import Optional

from bench.language.core import (
    BuiltinEnum,
    BuiltinObject,
    EnumType,
    Node,
    NodeType,
    Struct,
    StructType,
    enum_,
    node_,
    object_,
    p_regular,
    struct_,
)
from bench.pb2 import GradientStyleData

from .color import Color
from .core import Axis2
from .style import IsStyle

# pyright: reportIncompatibleVariableOverride=false


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

    color: Optional["Color"] = p_regular(50)
    position: float = p_regular(51)


@object_()
class GradientBase(BuiltinObject):
    type: GradientType = p_regular(30, default=GradientType.LINEAR)
    style: Optional["GradientStyle"] = p_regular(40)
    angle: Optional[float] = p_regular(50)
    stops: list[GradientStop] = p_regular(51)
    center_anchor: Optional[Axis2] = p_regular(52)


@struct_(StructType.GRADIENT)
class Gradient(GradientBase, Struct):
    """A gradient value."""

    pass


@node_(NodeType.GRADIENT_STYLE)
class GradientStyle(GradientBase, IsStyle, Node[GradientStyleData]):
    """A gradient style."""

    dark: Gradient | None = p_regular(60)
