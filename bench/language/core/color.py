from typing import Optional

from .const import ColorShade, ColorType, StructType
from .property import p_regular
from .struct import Struct, struct_


@struct_(StructType.COLOR)
class Color(Struct):
    """A color value."""

    type: Optional[ColorType] = p_regular(31, default=None)
    shade: Optional[ColorShade] = p_regular(32, default=None)
    hex: Optional[str] = p_regular(33, default=None)

    @staticmethod
    def new(color: "ColorIn") -> "Color":
        return to_color(color)


ColorIn = Color | ColorType | str


def to_color(color: ColorIn) -> Color:
    if isinstance(color, Color):
        return color
    elif isinstance(color, ColorType):
        return Color(type=color)
    else:
        return Color(hex=color)
