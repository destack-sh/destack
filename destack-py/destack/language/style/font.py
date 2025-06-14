from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    BuiltinObjectMutable,
    Enum,
    EnumType,
    Length,
    Node,
    NodeType,
    StructMutable,
    StructType,
    builtin_enum,
    builtin_node,
    builtin_struct,
    object_,
    property_,
)
from destack.pb2 import FontStyleData

from .fill import Fill
from .style import Style

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.FONT_TYPE)
class FontType(Enum):
    STYLE = 2
    SERIF = 10
    SANS = 11
    MONO = 12


@builtin_enum(EnumType.FONT_WEIGHT)
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


@builtin_enum(EnumType.FONT_SIZE)
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


@builtin_enum(EnumType.TEXT_ALIGN)
class TextAlign(Enum):
    LEFT = 1
    CENTER = 2
    RIGHT = 3
    JUSTIFY = 4


@builtin_enum(EnumType.TEXT_DECORATION)
class TextDecoration(Enum):
    NONE = 1
    UNDERLINE = 2
    STRIKETHROUGH = 3


@builtin_enum(EnumType.TEXT_TRANSFORM)
class TextTransform(Enum):
    NONE = 1
    UPPERCASE = 2
    LOWERCASE = 3
    CAPITALIZE = 4


@object_()
class FontBase(BuiltinObjectMutable):
    """A text style value."""

    type: FontType = property_(30, default=FontType.SANS, is_repr=True)
    weight: Optional[FontWeight] = property_(50, default=FontWeight.NORMAL, is_repr=True)
    color: Optional[Fill] = property_(51, is_repr=True)
    size: Optional[FontSize] = property_(52, default=FontSize.BASE, is_repr=True)
    align: Optional[TextAlign] = property_(53, default=TextAlign.LEFT, is_repr=True)
    line_height: Optional[Length] = property_(54, is_repr=True)
    letter_spacing: Optional[Length] = property_(55, is_repr=True)
    decoration: Optional[TextDecoration] = property_(56, default=TextDecoration.NONE, is_repr=True)
    transform: Optional[TextTransform] = property_(57, default=TextTransform.NONE, is_repr=True)


@builtin_struct(StructType.FONT)
class Font(FontBase, StructMutable):
    """A font value."""

    style: Optional["FontStyle"] = property_(41, is_repr=True)


@builtin_node(NodeType.FONT_STYLE)
class FontStyle(
    Style,
    FontBase,
    Node[FontStyleData],
):
    """A font style."""

    pass
