from typing import TYPE_CHECKING, Optional, assert_never

from bench.language.core import (
    BuiltinEnum,
    BuiltinObject,
    EnumType,
    NodeType,
    Struct,
    StructType,
    enum_,
    node_,
    object_,
    p_regular,
    struct_,
)
from bench.pb2 import ColorStyleData

from .core import IsVariable
from .style import StyleBase

if TYPE_CHECKING:
    pass


@enum_(EnumType.COLOR_TYPE)
class ColorType(BuiltinEnum):
    """Built-in color formats."""

    BUILTIN = 1
    STYLE = 2
    FIELD = 3
    RGB = 10
    HSL = 11
    P3 = 12


@enum_(EnumType.COLOR_HUE)
class ColorHue(BuiltinEnum):
    """Built-in colors a la SwiftUI or Tailwind."""

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

    S25 = 25
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


@object_()
class ColorBase(IsVariable, BuiltinObject):
    """A color value (x, y, z, alpha in 0-1)."""

    type: ColorType = p_regular(30)
    style: Optional["ColorStyle"] = p_regular(42)

    hue: Optional[ColorHue] = p_regular(50)
    shade: Optional[ColorShade] = p_regular(51)
    x: Optional[float] = p_regular(52)
    y: Optional[float] = p_regular(53)
    z: Optional[float] = p_regular(54)
    alpha: Optional[float] = p_regular(55)


@struct_(StructType.COLOR)
class Color(ColorBase, Struct):
    """A color value."""

    @staticmethod
    def new(color: "ColorIn") -> "Color":
        return to_color(color)

    @staticmethod
    def from_hex(hex: str) -> "Color":
        r, g, b, a = hex_to_rgb(hex)
        return Color(type=ColorType.RGB, x=r, y=g, z=b, alpha=a)

    @staticmethod
    def from_hue(hue: ColorHue, shade: ColorShade | None = None) -> "Color":
        return Color(type=ColorType.BUILTIN, hue=hue, shade=shade)


@node_(NodeType.COLOR_STYLE)
class ColorStyle(ColorBase, StyleBase[ColorStyleData]):
    """A color style, with an optional dark variant."""

    dark: Color | None = p_regular(60)

    @staticmethod
    def from_color(color: Color, dark: Color | None = None) -> "ColorStyle":
        return ColorStyle(
            type=color.type,
            hue=color.hue,
            shade=color.shade,
            style=color.style,
            x=color.x,
            y=color.y,
            z=color.z,
            alpha=color.alpha,
            dark=dark,
        )

    @staticmethod
    def from_hex(hex: str, dark: str | None = None) -> "ColorStyle":
        return ColorStyle.from_color(
            Color.from_hex(hex),
            Color.from_hex(dark) if dark else None,
        )

    @staticmethod
    def from_hue(
        hue: ColorHue,
        shade: ColorShade | None = None,
        dark_shade: ColorShade | None = None,
    ) -> "ColorStyle":
        return ColorStyle.from_color(
            Color.from_hue(hue, shade),
            Color.from_hue(hue, dark_shade) if dark_shade else None,
        )


ColorIn = Color | ColorHue | ColorStyle | str


def to_color(color: ColorIn) -> Color:
    if isinstance(color, Color):
        return color
    elif isinstance(color, ColorHue):
        return Color(type=ColorType.BUILTIN, hue=color)
    elif isinstance(color, ColorStyle):
        return Color(type=ColorType.STYLE, style=color)
    elif isinstance(color, str):
        r, g, b, a = hex_to_rgb(color)
        return Color(type=ColorType.RGB, x=r, y=g, z=b, alpha=a)
    else:
        assert_never(color)


def to_color_style(color: ColorIn) -> ColorStyle:
    if isinstance(color, Color):
        return ColorStyle.from_color(color)
    elif isinstance(color, ColorHue):
        return ColorStyle(type=ColorType.BUILTIN, hue=color)
    elif isinstance(color, ColorStyle):
        return color
    elif isinstance(color, str):
        r, g, b, a = hex_to_rgb(color)
        return ColorStyle(type=ColorType.RGB, x=r, y=g, z=b, alpha=a)
    else:
        assert_never(color)


#
# Utility
#


def _srgb_to_linear(c: float) -> float:
    """y-encoded sRGB → linear."""
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def _linear_to_srgb(c: float) -> float:
    """linear → y-encoded sRGB."""
    return c * 12.92 if c <= 0.0031308 else 1.055 * c ** (1 / 2.4) - 0.055


def _clamp01(x: float) -> float:  # avoid tiny negatives after matrices
    return max(0.0, min(1.0, x))


_SRGB_TO_XYZ = (
    (0.4124564, 0.3575761, 0.1804375),
    (0.2126729, 0.7151522, 0.0721750),
    (0.0193339, 0.1191920, 0.9503041),
)

_XYZ_TO_SRGB = (
    (3.2406, -1.5372, -0.4986),
    (-0.9689, 1.8758, 0.0415),
    (0.0557, -0.2040, 1.0570),
)

_P3_TO_XYZ = (
    (0.48657095, 0.26566769, 0.19821729),
    (0.22897456, 0.69173852, 0.07928691),
    (0.00000000, 0.04511338, 1.04394437),
)

_XYZ_TO_P3 = (
    (2.49349691, -0.93138362, -0.40271078),
    (-0.82948897, 1.76266400, 0.02362468),
    (0.03584583, -0.07617239, 0.95688452),
)


def hex_to_rgb(hex: str) -> tuple[float, float, float, float | None]:
    """Hex → linear-space floats 0-1 (optional alpha)."""
    if len(hex) == 6:
        r = int(hex[0:2], 16) / 255.0
        g = int(hex[2:4], 16) / 255.0
        b = int(hex[4:6], 16) / 255.0
        return (r, g, b, None)
    elif len(hex) == 8:
        r = int(hex[0:2], 16) / 255.0
        g = int(hex[2:4], 16) / 255.0
        b = int(hex[4:6], 16) / 255.0
        a = int(hex[6:8], 16) / 255.0
        return (r, g, b, a)
    else:
        raise ValueError(f"invalid hex color: {hex}")


def rgb_to_hex(r: int, g: int, b: int, a: int | None = None) -> str:
    """8-bit sRGB → hex."""
    if a is None:
        return f"{r:02x}{g:02x}{b:02x}"
    else:
        return f"{r:02x}{g:02x}{b:02x}{a:02x}"


def rgb_to_hsl(r: int, g: int, b: int) -> tuple[float, float, float]:
    """8-bit sRGB → HSL (h° 0-360, s|l 0-1)."""
    r_f, g_f, b_f = [v / 255.0 for v in (r, g, b)]
    c_max, c_min = max(r_f, g_f, b_f), min(r_f, g_f, b_f)
    delta = c_max - c_min
    if delta == 0:
        h = 0.0
    elif c_max == r_f:
        h = ((g_f - b_f) / delta) % 6
    elif c_max == g_f:
        h = (b_f - r_f) / delta + 2
    else:
        h = (r_f - g_f) / delta + 4
    h *= 60.0
    l = (c_max + c_min) / 2.0  # noqa: E741
    s = 0.0 if delta == 0 else delta / (1.0 - abs(2.0 * l - 1.0))
    return h, s, l


def hsl_to_rgb(h: float, s: float, l: float) -> tuple[float, float, float]:  # noqa: E741
    """HSL (h° 0-360, s|l 0-1) → linear-space floats 0-1."""
    c = (1.0 - abs(2.0 * l - 1.0)) * s
    x = c * (1.0 - abs((h / 60.0) % 2.0 - 1.0))
    m = l - c / 2.0
    if 0 <= h < 60:
        r1, g1, b1 = c, x, 0
    elif 60 <= h < 120:
        r1, g1, b1 = x, c, 0
    elif 120 <= h < 180:
        r1, g1, b1 = 0, c, x
    elif 180 <= h < 240:
        r1, g1, b1 = 0, x, c
    elif 240 <= h < 300:
        r1, g1, b1 = x, 0, c
    else:
        r1, g1, b1 = c, 0, x
    return r1 + m, g1 + m, b1 + m


def _mat_mul(
    v: tuple[float, float, float], m: tuple[tuple[float, float, float], ...]
) -> tuple[float, float, float]:
    x = v[0] * m[0][0] + v[1] * m[0][1] + v[2] * m[0][2]
    y = v[0] * m[1][0] + v[1] * m[1][1] + v[2] * m[1][2]
    z = v[0] * m[2][0] + v[1] * m[2][1] + v[2] * m[2][2]
    return x, y, z


def rgb_to_p3(r: float, g: float, b: float) -> tuple[float, float, float]:
    """y-encoded sRGB (0-1) → y-encoded Display-P3 (0-1)."""
    # sRGB y → linear
    rl, gl, bl = map(_srgb_to_linear, (r, g, b))
    # linear sRGB → XYZ → linear P3
    X, Y, Z = _mat_mul((rl, gl, bl), _SRGB_TO_XYZ)
    rp3_l, gp3_l, bp3_l = _mat_mul((X, Y, Z), _XYZ_TO_P3)
    # linear P3 → y; clamp
    return (
        _clamp01(_linear_to_srgb(rp3_l)),
        _clamp01(_linear_to_srgb(gp3_l)),
        _clamp01(_linear_to_srgb(bp3_l)),
    )


def p3_to_rgb(rp3: float, gp3: float, bp3: float) -> tuple[float, float, float]:
    """y-encoded Display-P3 (0-1) → y-encoded sRGB (0-1)."""
    # P3 y → linear
    rp3_l, gp3_l, bp3_l = map(_srgb_to_linear, (rp3, gp3, bp3))
    # linear P3 → XYZ → linear sRGB
    X, Y, Z = _mat_mul((rp3_l, gp3_l, bp3_l), _P3_TO_XYZ)
    r_l, g_l, b_l = _mat_mul((X, Y, Z), _XYZ_TO_SRGB)
    # linear sRGB → y; clamp
    return (
        _clamp01(_linear_to_srgb(r_l)),
        _clamp01(_linear_to_srgb(g_l)),
        _clamp01(_linear_to_srgb(b_l)),
    )


def hsl_to_p3(h: float, s: float, l: float) -> tuple[float, float, float]:  # noqa: E741
    """HSL → y-encoded Display-P3 (0-1)."""
    return rgb_to_p3(*hsl_to_rgb(h, s, l))


def p3_to_hsl(rp3: float, gp3: float, bp3: float) -> tuple[float, float, float]:
    """y-encoded Display-P3 (0-1) → HSL."""
    r, g, b = p3_to_rgb(rp3, gp3, bp3)
    return rgb_to_hsl(int(r * 255 + 0.5), int(g * 255 + 0.5), int(b * 255 + 0.5))
