from typing import Optional

from bench.language.core import (
    BuiltinEnum,
    BuiltinObjectMutable,
    EnumType,
    IsArchivable,
    IsDeletable,
    IsTracked,
    Node,
    NodeType,
    NumberFormat,
    StructFrozen,
    StructMutable,
    StructType,
    enum_,
    node_,
    object_,
    property_,
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


@struct_(StructType.GRADIENT_STOP, frozen=True)
class GradientStop(StructFrozen):
    """A gradient stop with color and position."""

    color: Optional["Color"] = property_(50, is_repr=True)
    position: float = property_(51, format=NumberFormat.PERCENTAGE, is_repr=True)


@object_()
class GradientBase(BuiltinObjectMutable):
    type: GradientType = property_(30, default=GradientType.LINEAR, is_repr=True)
    style: Optional["GradientStyle"] = property_(40, is_repr=True)
    angle: Optional[float] = property_(50, format=NumberFormat.ANGLE, is_repr=True)
    stops: list[GradientStop] = property_(51, is_repr=True)
    center_anchor: Optional[Axis2] = property_(52, is_repr=True)


@struct_(StructType.GRADIENT)
class Gradient(GradientBase, StructMutable):
    """A gradient value."""

    pass


@node_(NodeType.GRADIENT_STYLE)
class GradientStyle(
    GradientBase,
    IsStyle,
    IsDeletable,
    IsArchivable,
    IsTracked,
    Node[GradientStyleData],
):
    """A gradient style."""

    dark: Gradient | None = property_(60)
