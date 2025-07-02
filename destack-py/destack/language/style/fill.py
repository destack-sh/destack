from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Enum,
    EnumType,
    NodeType,
    StructFrozen,
    StructType,
    builtin_enum,
    builtin_node,
    builtin_property,
    builtin_struct,
)

from .color import Color
from .gradient import Gradient
from .style import Style

if TYPE_CHECKING:
    from destack.language import File

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.FILL_TYPE)
class FillType(Enum):
    SOLID = 10
    GRADIENT = 11
    IMAGE = 12


@builtin_enum(EnumType.FILL_POSITION)
class FillPosition(Enum):
    TOP_LEFT = 1
    TOP_CENTER = 2
    TOP_RIGHT = 3
    LEFT = 10
    CENTER = 11
    RIGHT = 12
    BOTTOM_LEFT = 20
    BOTTOM_CENTER = 21
    BOTTOM_RIGHT = 22


@builtin_enum(EnumType.FILL_SIZE)
class FillSize(Enum):
    FILL = 1
    STRETCH = 2
    FIT = 3
    TILE = 4


@builtin_struct(StructType.FILL, frozen=True)
class Fill(StructFrozen):
    """A fill value."""

    type: FillType = builtin_property(100, is_repr=True)
    style: Optional["FillStyle"] = builtin_property(101, is_repr=True)
    color: Color | None = builtin_property(102, is_repr=True)
    gradient: Optional[Gradient] = builtin_property(103, is_repr=True)
    image: "File | None" = builtin_property(104, is_repr=True)
    position: FillPosition | None = builtin_property(105, is_repr=True)
    size: FillSize | None = builtin_property(106, is_repr=True)

    @staticmethod
    def from_color(color: Color) -> "Fill":
        return Fill(type=FillType.SOLID, color=color)

    @staticmethod
    def from_gradient(gradient: Gradient) -> "Fill":
        return Fill(type=FillType.GRADIENT, gradient=gradient)


@builtin_node(NodeType.FILL_STYLE)
class FillStyle(Style):
    """A fill style."""

    type: FillType = builtin_property(100, is_repr=True)
    color: Color | None = builtin_property(200, is_repr=True)
    gradient: Optional[Gradient] = builtin_property(201, is_repr=True)
    image: "File | None" = builtin_property(202, is_repr=True)
    position: FillPosition | None = builtin_property(203, is_repr=True)
    size: FillSize | None = builtin_property(204, is_repr=True)

    @staticmethod
    def from_fill(fill: Fill) -> "FillStyle":
        return FillStyle(
            type=fill.type,
            color=fill.color,
            gradient=fill.gradient,
            image=fill.image,
            position=fill.position,
            size=fill.size,
        )

    @staticmethod
    def from_color(color: Color) -> "FillStyle":
        return FillStyle(type=FillType.SOLID, color=color)

    @staticmethod
    def from_gradient(gradient: Gradient) -> "FillStyle":
        return FillStyle(type=FillType.GRADIENT, gradient=gradient)
