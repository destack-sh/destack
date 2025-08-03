from typing import TYPE_CHECKING, Optional, final

from destack.core import (
    Enum,
    EnumType,
    NodeType,
    StructFrozen,
    StructType,
    declare_entity,
    declare_enum,
    declare_property,
    declare_struct,
)

from .fill import Fill
from .style import Style

if TYPE_CHECKING:
    from destack import Length

# pyright: reportIncompatibleVariableOverride=false


@declare_enum(EnumType.FONT_TYPE)
class FontType(Enum):
    SERIF = 10
    SANS = 11
    MONO = 12


@declare_enum(EnumType.FONT_WEIGHT)
class FontWeight(Enum):
    THIN = 100
    EXTRA_LIGHT = 200
    LIGHT = 300
    NORMAL = 400
    MEDIUM = 500
    SEMI_BOLD = 600
    BOLD = 700
    EXTRA_BOLD = 800
    BLACK = 900


@declare_enum(EnumType.FONT_SIZE)
class FontSize(Enum):
    XS = 12
    SM = 14
    BASE = 16
    LG = 18
    XL = 20
    XL2 = 24
    XL3 = 30
    XL4 = 36
    XL5 = 48
    XL6 = 60
    XL7 = 72


@declare_enum(EnumType.TEXT_ALIGN)
class TextAlign(Enum):
    LEFT = 1
    CENTER = 2
    RIGHT = 3
    JUSTIFY = 4


@declare_enum(EnumType.TEXT_DECORATION)
class TextDecoration(Enum):
    NONE = 1
    UNDERLINE = 2
    STRIKETHROUGH = 3


@declare_enum(EnumType.TEXT_TRANSFORM)
class TextTransform(Enum):
    NONE = 1
    UPPERCASE = 2
    LOWERCASE = 3
    CAPITALIZE = 4


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
