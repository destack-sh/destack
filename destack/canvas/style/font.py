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

from .fill import Fill
from .style import Style

if TYPE_CHECKING:
    from destack import Length


@declare_enum(EnumType.FONT_TYPE)
class FontType(OptionEnum):
    SERIF = declare_option(10, "Serif", description="A serif font")
    SANS = declare_option(11, "Sans", description="A sans-serif font")
    MONO = declare_option(12, "Mono", description="A monospace font")


@declare_enum(EnumType.FONT_WEIGHT)
class FontWeight(OptionEnum):
    THIN = declare_option(100, "Thin", description="A thin font weight")
    EXTRA_LIGHT = declare_option(200, "Extra Light", description="An extra light font weight")
    LIGHT = declare_option(300, "Light", description="A light font weight")
    NORMAL = declare_option(400, "Normal", description="A normal font weight")
    MEDIUM = declare_option(500, "Medium", description="A medium font weight")
    SEMI_BOLD = declare_option(600, "Semi Bold", description="A semi-bold font weight")
    BOLD = declare_option(700, "Bold", description="A bold font weight")
    EXTRA_BOLD = declare_option(800, "Extra Bold", description="An extra bold font weight")
    BLACK = declare_option(900, "Black", description="A black font weight")


@declare_enum(EnumType.FONT_SIZE)
class FontSize(OptionEnum):
    XS = declare_option(12, "XS", description="An XS font size")
    SM = declare_option(14, "SM", description="An SM font size")
    BASE = declare_option(16, "Base", description="A base font size")
    LG = declare_option(18, "LG", description="An LG font size")
    XL = declare_option(20, "XL", description="An XL font size")
    XL2 = declare_option(24, "XL2", description="An XL2 font size")
    XL3 = declare_option(30, "XL3", description="An XL3 font size")
    XL4 = declare_option(36, "XL4", description="An XL4 font size")
    XL5 = declare_option(48, "XL5", description="An XL5 font size")
    XL6 = declare_option(60, "XL6", description="An XL6 font size")
    XL7 = declare_option(72, "XL7", description="An XL7 font size")


@declare_enum(EnumType.TEXT_ALIGN)
class TextAlign(OptionEnum):
    LEFT = declare_option(1, "Left", description="A left text alignment")
    CENTER = declare_option(2, "Center", description="A center text alignment")
    RIGHT = declare_option(3, "Right", description="A right text alignment")
    JUSTIFY = declare_option(4, "Justify", description="A justify text alignment")


@declare_enum(EnumType.TEXT_DECORATION)
class TextDecoration(OptionEnum):
    NONE = declare_option(1, "None", description="No text decoration")
    UNDERLINE = declare_option(2, "Underline", description="An underline text decoration")
    STRIKETHROUGH = declare_option(
        3, "Strikethrough", description="A strikethrough text decoration"
    )


@declare_enum(EnumType.TEXT_TRANSFORM)
class TextTransform(OptionEnum):
    NONE = declare_option(1, "None", description="No text transform")
    UPPERCASE = declare_option(2, "Uppercase", description="An uppercase text transform")
    LOWERCASE = declare_option(3, "Lowercase", description="A lowercase text transform")
    CAPITALIZE = declare_option(4, "Capitalize", description="A capitalize text transform")


@declare_struct(
    StructType.FONT,
    frozen=True,
    is_final=True,
)
@final
class Font(StructFrozen):
    """A font value."""

    type: FontType = declare_property(100, default=FontType.SANS, is_repr=True)
    style: Optional["FontStyle"] = declare_property(101, is_repr=True)
    weight: Optional[FontWeight] = declare_property(102, default=FontWeight.NORMAL, is_repr=True)
    color: Optional[Fill] = declare_property(103, is_repr=True)
    size: Optional[FontSize] = declare_property(104, default=FontSize.BASE, is_repr=True)
    align: Optional[TextAlign] = declare_property(105, default=TextAlign.LEFT, is_repr=True)
    line_height: Optional["Length"] = declare_property(106, is_repr=True)
    letter_spacing: Optional["Length"] = declare_property(107, is_repr=True)
    decoration: Optional[TextDecoration] = declare_property(
        108, default=TextDecoration.NONE, is_repr=True
    )
    transform: Optional[TextTransform] = declare_property(
        109, default=TextTransform.NONE, is_repr=True
    )


@declare_entity(NodeType.FONT_STYLE)
class FontStyle(Style):
    """A font style."""

    type: FontType = declare_property(100, default=FontType.SANS, is_repr=True)
    weight: Optional[FontWeight] = declare_property(102, default=FontWeight.NORMAL, is_repr=True)
    color: Optional[Fill] = declare_property(103, is_repr=True)
    size: Optional[FontSize] = declare_property(104, default=FontSize.BASE, is_repr=True)
    align: Optional[TextAlign] = declare_property(105, default=TextAlign.LEFT, is_repr=True)
    line_height: Optional["Length"] = declare_property(106, is_repr=True)
    letter_spacing: Optional["Length"] = declare_property(107, is_repr=True)
    decoration: Optional[TextDecoration] = declare_property(
        108, default=TextDecoration.NONE, is_repr=True
    )
    transform: Optional[TextTransform] = declare_property(
        109, default=TextTransform.NONE, is_repr=True
    )
