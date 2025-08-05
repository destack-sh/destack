from typing import TYPE_CHECKING, Optional, final

from destack.core import (
    EnumType,
    Float32,
    NodeType,
    ObjectStability,
    OptionEnum,
    StructFrozen,
    StructType,
    UInt32,
    declare_entity,
    declare_enum,
    declare_method,
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

    RGB = declare_option(10, "RGB")
    HSL = declare_option(11, "HSL")
    P3 = declare_option(12, "P3")


@declare_enum(EnumType.COLOR_HUE)
class ColorHue(OptionEnum):
    """Built-in colors a la SwiftUI or Tailwind."""

    GRAY = declare_option(30, "Gray")
    RED = declare_option(31, "Red")
    ORANGE = declare_option(32, "Orange")
    AMBER = declare_option(33, "Amber")
    YELLOW = declare_option(34, "Yellow")
    LIME = declare_option(35, "Lime")
    GREEN = declare_option(36, "Green")
    EMERALD = declare_option(37, "Emerald")
    TEAL = declare_option(38, "Teal")
    CYAN = declare_option(39, "Cyan")
    SKY = declare_option(40, "Sky")
    BLUE = declare_option(41, "Blue")
    INDIGO = declare_option(42, "Indigo")
    VIOLET = declare_option(43, "Violet")
    PURPLE = declare_option(44, "Purple")
    FUCHSIA = declare_option(45, "Fuchsia")
    PINK = declare_option(46, "Pink")
    ROSE = declare_option(47, "Rose")


@declare_enum(EnumType.COLOR_SHADE)
class ColorShade(OptionEnum):
    """Built-in color shades a la Tailwind."""

    S25 = declare_option(25, "25")
    S50 = declare_option(50, "50")
    S100 = declare_option(100, "100")
    S200 = declare_option(200, "200")
    S300 = declare_option(300, "300")
    S400 = declare_option(400, "400")
    S500 = declare_option(500, "500")
    S600 = declare_option(600, "600")
    S700 = declare_option(700, "700")
    S800 = declare_option(800, "800")
    S900 = declare_option(900, "900")
    S950 = declare_option(950, "950")


@declare_enum(EnumType.COLOR_INTENT)
class ColorIntent(OptionEnum):
    """Built-in color intents."""

    PRIMARY = declare_option(1, "Primary", description="A Primary intent")
    SECONDARY = declare_option(2, "Secondary", description="A Secondary intent")
    NEUTRAL = declare_option(3, "Neutral", description="A Neutral intent")
    MUTED = declare_option(4, "Muted", description="A Muted intent")
    SUCCESS = declare_option(10, "Success", description="A Success intent")
    INFO = declare_option(11, "Info", description="An Info intent")
    WARNING = declare_option(12, "Warning", description="A Warning intent")
    ERROR = declare_option(13, "Error", description="An Error intent")
    CRITICAL = declare_option(14, "Critical", description="A Critical intent")


@declare_struct(
    StructType.COLOR,
    stability=ObjectStability.STATIC,
    frozen=True,
    is_final=True,
    into_node_types=(NodeType.COLOR_STYLE,),
)
@final
class Color(StructFrozen):
    """A color value."""

    r: Float32 = declare_property(101, is_repr=True)
    g: Float32 = declare_property(102, is_repr=True)
    b: Float32 = declare_property(103, is_repr=True)
    a: Float32 = declare_property(104, is_repr=True)

    @declare_method(201)
    @classmethod
    def from_hex(cls, hex: str) -> "Color":
        """Create a Color from a hex string."""
        ...


@declare_entity(
    NodeType.COLOR_STYLE,
    base_struct_type=StructType.COLOR,
)
class ColorStyle(Style):
    """A color style, with an optional dark variant."""

    type: ColorType = declare_property(100, is_repr=True)
    hue: Optional[ColorHue] = declare_property(200, is_repr=True)
    shade: Optional[ColorShade] = declare_property(201, is_repr=True)
    intent: Optional[ColorIntent] = declare_property(202, is_repr=True)
    r: Float32 = declare_property(203, is_repr=True)
    g: Float32 = declare_property(204, is_repr=True)
    b: Float32 = declare_property(205, is_repr=True)
    a: Float32 = declare_property(206, is_repr=True)
    dark: "ColorStyle | None" = declare_property(207)

    @declare_method(201)
    @classmethod
    def from_color(cls, name: str, color: Color, dark: Color | None = None) -> "ColorStyle":
        """Create a ColorStyle from a Color."""
        ...

    @declare_method(202)
    @classmethod
    def from_hex(cls, name: str, hex: str, dark: str | None = None) -> "ColorStyle":
        """Create a ColorStyle from a hex string."""
        ...


@declare_method(301)
def hex_to_rgb(hex: str) -> tuple[Float32, Float32, Float32, Float32 | None]:
    """Convert hex color string to linear-space RGB floats with optional alpha."""
    ...


@declare_method(302)
def rgb_to_hex(r: UInt32, g: UInt32, b: UInt32, a: UInt32 | None = None) -> str:
    """Convert 8-bit sRGB values to hex color string."""
    ...


@declare_method(303)
def rgb_to_hsl(r: UInt32, g: UInt32, b: UInt32) -> tuple[Float32, Float32, Float32]:
    """Convert 8-bit sRGB values to HSL color space."""
    ...


@declare_method(304)
def hsl_to_rgb(h: Float32, s: Float32, l: Float32) -> tuple[Float32, Float32, Float32]:  # noqa: E741
    """Convert HSL values to linear-space RGB floats."""
    ...


@declare_method(305)
def rgb_to_p3(r: Float32, g: Float32, b: Float32) -> tuple[Float32, Float32, Float32]:
    """Convert gamma-encoded sRGB to gamma-encoded Display-P3."""
    ...


@declare_method(306)
def p3_to_rgb(rp3: Float32, gp3: Float32, bp3: Float32) -> tuple[Float32, Float32, Float32]:
    """Convert gamma-encoded Display-P3 to gamma-encoded sRGB."""
    ...


@declare_method(307)
def hsl_to_p3(h: Float32, s: Float32, l: Float32) -> tuple[Float32, Float32, Float32]:  # noqa: E741
    """Convert HSL to gamma-encoded Display-P3."""
    ...


@declare_method(308)
def p3_to_hsl(rp3: Float32, gp3: Float32, bp3: Float32) -> tuple[Float32, Float32, Float32]:
    """Convert gamma-encoded Display-P3 to HSL."""
    ...
