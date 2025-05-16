from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    BuiltinEnum,
    BuiltinObject,
    EnumType,
    NodeType,
    Struct,
    StructType,
    enum_,
    node_,
    node_component_,
    p_regular,
    struct_,
)
from bench.pb2 import FontStyleData

from .core import IsVariable, Length
from .fill import Fill
from .style import StyleBase

if TYPE_CHECKING:
    pass


@enum_(EnumType.FONT_TYPE)
class FontType(BuiltinEnum):
    STYLE = 2
    FIELD = 3
    SERIF = 10
    SANS = 11
    MONO = 12


@enum_(EnumType.FONT_WEIGHT)
class FontWeight(BuiltinEnum):
    THIN = 100
    EXTRA_LIGHT = 200
    LIGHT = 300
    NORMAL = 400
    MEDIUM = 500
    SEMI_BOLD = 600
    BOLD = 700
    EXTRA_BOLD = 800
    BLACK = 900


@enum_(EnumType.FONT_SIZE)
class FontSize(BuiltinEnum):
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


@enum_(EnumType.TEXT_ALIGN)
class TextAlign(BuiltinEnum):
    LEFT = 1
    CENTER = 2
    RIGHT = 3
    JUSTIFY = 4


@enum_(EnumType.TEXT_DECORATION)
class TextDecoration(BuiltinEnum):
    NONE = 1
    UNDERLINE = 2
    STRIKETHROUGH = 3


@enum_(EnumType.TEXT_TRANSFORM)
class TextTransform(BuiltinEnum):
    NONE = 1
    UPPERCASE = 2
    LOWERCASE = 3
    CAPITALIZE = 4


@node_component_()
class FontBase(IsVariable, BuiltinObject):
    """A text style value."""

    type: FontType = p_regular(30, default=FontType.SANS)
    style: Optional["FontStyle"] = p_regular(
        41, default=None, require=False, array=False, references=NodeType.FONT_STYLE
    )
    weight: Optional[FontWeight] = p_regular(50, default=FontWeight.NORMAL)
    color: Optional[Fill] = p_regular(51, default=None, struct=StructType.FILL)
    size: Optional[FontSize] = p_regular(52, default=FontSize.BASE)
    align: Optional[TextAlign] = p_regular(53, default=TextAlign.LEFT)
    line_height: Optional[Length] = p_regular(54, default=None, struct=StructType.LENGTH)
    letter_spacing: Optional[Length] = p_regular(55, default=None, struct=StructType.LENGTH)
    decoration: Optional[TextDecoration] = p_regular(56, default=TextDecoration.NONE)
    transform: Optional[TextTransform] = p_regular(57, default=TextTransform.NONE)


@struct_(StructType.FONT)
class Font(FontBase, Struct):
    """A font value."""

    pass


@node_(NodeType.FONT_STYLE)
class FontStyle(FontBase, StyleBase[FontStyleData]):
    """A font style."""

    pass
