from typing import TYPE_CHECKING, Optional, final

from destack.core import (
    EnumType,
    NodeType,
    OptionEnum,
    StructFrozen,
    StructType,
    declare_entity,
    declare_enum,
    declare_option,
    declare_property,
    declare_struct,
)

from .color import Color
from .gradient import Gradient
from .style import Style

if TYPE_CHECKING:
    from destack import File


@declare_enum(EnumType.FILL_TYPE)
class FillType(OptionEnum):
    SOLID = declare_option(10, "Solid", description="A solid fill")
    GRADIENT = declare_option(11, "Gradient", description="A gradient fill")
    IMAGE = declare_option(12, "Image", description="An image fill")


@declare_enum(EnumType.FILL_POSITION)
class FillPosition(OptionEnum):
    TOP_LEFT = declare_option(1, "Top Left", description="A top left fill position")
    TOP_CENTER = declare_option(2, "Top Center", description="A top center fill position")
    TOP_RIGHT = declare_option(3, "Top Right", description="A top right fill position")
    LEFT = declare_option(10, "Left", description="A left fill position")
    CENTER = declare_option(11, "Center", description="A center fill position")
    RIGHT = declare_option(12, "Right", description="A right fill position")
    BOTTOM_LEFT = declare_option(20, "Bottom Left", description="A bottom left fill position")
    BOTTOM_CENTER = declare_option(21, "Bottom Center", description="A bottom center fill position")
    BOTTOM_RIGHT = declare_option(22, "Bottom Right", description="A bottom right fill position")


@declare_enum(EnumType.FILL_SIZE)
class FillSize(OptionEnum):
    FILL = declare_option(1, "Fill", description="A fill size")
    STRETCH = declare_option(2, "Stretch", description="A stretch size")
    FIT = declare_option(3, "Fit", description="A fit size")
    TILE = declare_option(4, "Tile", description="A tile size")


@declare_struct(
    StructType.FILL,
    frozen=True,
    is_final=True,
    into_node_types=(NodeType.FILL_STYLE,),
)
@final
class Fill(StructFrozen):
    """A fill value."""

    type: FillType = declare_property(100, is_repr=True)
    template: Optional["FillStyle"] = declare_property(101, is_repr=True)
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


@declare_entity(
    NodeType.FILL_STYLE,
    base_struct_type=StructType.FILL,
)
class FillStyle(Style):
    """A fill style."""

    type: FillType = declare_property(100, is_repr=True)
    color: Optional["Color"] = declare_property(101, is_repr=True)
    gradient: Optional["Gradient"] = declare_property(102, is_repr=True)
    image: "File | None" = declare_property(103, is_repr=True)
    position: FillPosition | None = declare_property(104, is_repr=True)
    size: FillSize | None = declare_property(105, is_repr=True)

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
