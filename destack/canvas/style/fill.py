from typing import TYPE_CHECKING, Optional, final

from destack.core import (
    EnumDeclaration,
    EnumType,
    NodeType,
    StructFrozen,
    StructType,
    declare_entity,
    declare_enum,
    declare_property,
    declare_struct,
)

from .color import Color
from .gradient import Gradient
from .style import Style

if TYPE_CHECKING:
    from destack import File

# pyright: reportIncompatibleVariableOverride=false


@declare_enum(EnumType.FILL_TYPE)
class FillType(EnumDeclaration):
    SOLID = 10
    GRADIENT = 11
    IMAGE = 12


@declare_enum(EnumType.FILL_POSITION)
class FillPosition(EnumDeclaration):
    TOP_LEFT = 1
    TOP_CENTER = 2
    TOP_RIGHT = 3
    LEFT = 10
    CENTER = 11
    RIGHT = 12
    BOTTOM_LEFT = 20
    BOTTOM_CENTER = 21
    BOTTOM_RIGHT = 22


@declare_enum(EnumType.FILL_SIZE)
class FillSize(EnumDeclaration):
    FILL = 1
    STRETCH = 2
    FIT = 3
    TILE = 4


@declare_struct(
    StructType.FILL,
    frozen=True,
    is_final=True,
)
@final
class Fill(StructFrozen):
    """A fill value."""

    type: FillType = declare_property(100, is_repr=True)
    style: Optional["FillStyle"] = declare_property(101, is_repr=True)
    color: Color | None = declare_property(102, is_repr=True)
    gradient: Optional[Gradient] = declare_property(103, is_repr=True)
    image: "File | None" = declare_property(104, is_repr=True)
    position: FillPosition | None = declare_property(105, is_repr=True)
    size: FillSize | None = declare_property(106, is_repr=True)

    @staticmethod
    def from_color(color: Color) -> "Fill":
        return Fill(type=FillType.SOLID, color=color)

    @staticmethod
    def from_gradient(gradient: Gradient) -> "Fill":
        return Fill(type=FillType.GRADIENT, gradient=gradient)


@declare_entity(NodeType.FILL_STYLE)
class FillStyle(Style):
    """A fill style."""

    type: FillType = declare_property(100, is_repr=True)
    color: Color | None = declare_property(200, is_repr=True)
    gradient: Optional[Gradient] = declare_property(201, is_repr=True)
    image: "File | None" = declare_property(202, is_repr=True)
    position: FillPosition | None = declare_property(203, is_repr=True)
    size: FillSize | None = declare_property(204, is_repr=True)

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
