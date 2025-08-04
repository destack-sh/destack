from typing import TYPE_CHECKING, Optional, final

from destack.core import (
    EnumType,
    Float32,
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

from .style import Style

if TYPE_CHECKING:
    pass


@declare_enum(EnumType.COLOR_TYPE)
class ColorType(OptionEnum):
    """Built-in color formats."""

    BUILTIN = declare_option(1, "Builtin", description="A builtin color")
    RGB = declare_option(10, "RGB", description="An RGB color")
    HSL = declare_option(11, "HSL", description="An HSL color")
    P3 = declare_option(12, "P3", description="A P3 color")


@declare_enum(EnumType.COLOR_HUE)
class ColorHue(OptionEnum):
    """Built-in colors a la SwiftUI or Tailwind."""

    GRAY = declare_option(30, "Gray", description="A Gray color")
    RED = declare_option(31, "Red", description="A Red color")
    ORANGE = declare_option(32, "Orange", description="An Orange color")
    AMBER = declare_option(33, "Amber", description="An Amber color")
    YELLOW = declare_option(34, "Yellow", description="A Yellow color")
    LIME = declare_option(35, "Lime", description="A Lime color")
    GREEN = declare_option(36, "Green", description="A Green color")
    EMERALD = declare_option(37, "Emerald", description="An Emerald color")
    TEAL = declare_option(38, "Teal", description="A Teal color")
    CYAN = declare_option(39, "Cyan", description="A Cyan color")
    SKY = declare_option(40, "Sky", description="A Sky color")
    BLUE = declare_option(41, "Blue", description="A Blue color")
    INDIGO = declare_option(42, "Indigo", description="An Indigo color")
    VIOLET = declare_option(43, "Violet", description="A Violet color")
    PURPLE = declare_option(44, "Purple", description="A Purple color")
    FUCHSIA = declare_option(45, "Fuchsia", description="A Fuchsia color")
    PINK = declare_option(46, "Pink", description="A Pink color")
    ROSE = declare_option(47, "Rose", description="A Rose color")


@declare_enum(EnumType.COLOR_SHADE)
class ColorShade(OptionEnum):
    """Built-in color shades a la Tailwind."""

    S25 = declare_option(25, "25", description="A 25% shade")
    S50 = declare_option(50, "50", description="A 50% shade")
    S100 = declare_option(100, "100", description="A 100% shade")
    S200 = declare_option(200, "200", description="A 200% shade")
    S300 = declare_option(300, "300", description="A 300% shade")
    S400 = declare_option(400, "400", description="A 400% shade")
    S500 = declare_option(500, "500", description="A 500% shade")
    S600 = declare_option(600, "600", description="A 600% shade")
    S700 = declare_option(700, "700", description="A 700% shade")
    S800 = declare_option(800, "800", description="A 800% shade")
    S900 = declare_option(900, "900", description="A 900% shade")
    S950 = declare_option(950, "950", description="A 950% shade")


@declare_enum(EnumType.COLOR_INTENT)
class ColorIntent(OptionEnum):
    """Built-in color intents."""

    PRIMARY = declare_option(1, "Primary", description="A Primary intent")
    SECONDARY = declare_option(2, "Secondary", description="A Secondary intent")
    NEUTRAL = declare_option(3, "Neutral", description="A Neutral intent")
    SUCCESS = declare_option(10, "Success", description="A Success intent")
    INFO = declare_option(11, "Info", description="An Info intent")
    WARNING = declare_option(12, "Warning", description="A Warning intent")
    ERROR = declare_option(13, "Error", description="An Error intent")


@declare_struct(StructType.COLOR, frozen=True, is_final=True)
@final
class Color(StructFrozen):
    """A color value."""

    r: Float32 = declare_property(101, is_repr=True)
    g: Float32 = declare_property(102, is_repr=True)
    b: Float32 = declare_property(103, is_repr=True)
    a: Float32 = declare_property(104, is_repr=True)

    @staticmethod
    def from_hex(hex: str) -> "Color":
        r, g, b, a = hex_to_rgb(hex)
        return Color(r=r, g=g, b=b, a=a or 1.0)


@declare_entity(NodeType.COLOR_STYLE)
class ColorStyle(Style):
    """A color style, with an optional dark variant."""

    type: ColorType = declare_property(100, is_repr=True)
    hue: Optional[ColorHue] = declare_property(200, is_repr=True)
    shade: Optional[ColorShade] = declare_property(201, is_repr=True)
    intent: Optional[ColorIntent] = declare_property(202, is_repr=True)
    r: Optional[Float32] = declare_property(203, is_repr=True)
    g: Optional[Float32] = declare_property(204, is_repr=True)
    b: Optional[Float32] = declare_property(205, is_repr=True)
    a: Optional[Float32] = declare_property(206, is_repr=True)
    dark: Color | None = declare_property(207)

    @staticmethod
    def from_color(name: str, color: Color, dark: Color | None = None) -> "ColorStyle":
        return ColorStyle(
            name=name,
            type=ColorType.RGB,
            r=color.r,
            g=color.g,
            b=color.b,
            a=color.a,
            dark=dark,
        )

    @staticmethod
    def from_hex(name: str, hex: str, dark: str | None = None) -> "ColorStyle":
        return ColorStyle.from_color(
            name,
            Color.from_hex(hex),
            Color.from_hex(dark) if dark else None,
        )


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
