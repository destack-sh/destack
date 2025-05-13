from typing import Optional

from bench.language.core import (
    BuiltinEnum,
    BuiltinObject,
    EnumType,
    Struct,
    StructType,
    enum_,
    node_component_,
    p_regular,
    struct_,
)


@enum_(EnumType.COLOR_TYPE)
class ColorType(BuiltinEnum):
    """Built-in color types a la SwiftUI or Tailwind."""

    GRAY = 30
    RED = 31
    ORANGE = 32
    AMBER = 33
    YELLOW = 34
    LIME = 35
    GREEN = 36
    EMERALD = 37
    TEAL = 38
    CYAN = 39
    SKY = 40
    BLUE = 41
    INDIGO = 42
    VIOLET = 43
    PURPLE = 44
    FUCHSIA = 45
    PINK = 46
    ROSE = 47


@enum_(EnumType.COLOR_SHADE)
class ColorShade(BuiltinEnum):
    """Built-in color shades a la Tailwind."""

    S50 = 50
    S100 = 100
    S200 = 200
    S300 = 300
    S400 = 400
    S500 = 500
    S600 = 600
    S700 = 700
    S800 = 800
    S900 = 900
    S950 = 950


@node_component_()
class ColorBase(BuiltinObject):
    """A color value."""


@struct_(StructType.COLOR)
class Color(Struct):
    """A color value."""

    type: Optional[ColorType] = p_regular(31, default=None)
    shade: Optional[ColorShade] = p_regular(32, default=None)
    hex: Optional[str] = p_regular(33, default=None)
    opacity: Optional[float] = p_regular(34, default=None)

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
