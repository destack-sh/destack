from typing import Optional

from bench.language.core import (
    BuiltinEnum,
    BuiltinObject,
    EnumType,
    NodeType,
    StructType,
    enum_,
    node_,
    node_component_,
    p_regular,
)
from bench.pb2 import TextStyleData

from .color import Color
from .length import Length
from .style import StyleBase


@enum_(EnumType.FONT_TYPE)
class FontType(BuiltinEnum):
    SERIF = 1
    SANS = 2
    MONO = 3


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
class TextStyleBase(BuiltinObject):
    type: Optional[FontType] = p_regular(30, default=FontType.SERIF)
    weight: Optional[FontWeight] = p_regular(41, default=FontWeight.NORMAL)
    color: Optional[Color] = p_regular(45, default=None, struct=StructType.COLOR)
    size: Optional[FontSize] = p_regular(46, default=FontSize.BASE)
    align: Optional[TextAlign] = p_regular(47, default=TextAlign.LEFT)
    line_height: Optional[Length] = p_regular(48, default=None, struct=StructType.LENGTH)
    letter_spacing: Optional[Length] = p_regular(49, default=None, struct=StructType.LENGTH)
    decoration: Optional[TextDecoration] = p_regular(50, default=TextDecoration.NONE)
    transform: Optional[TextTransform] = p_regular(51, default=TextTransform.NONE)


@node_(NodeType.TEXT_STYLE)
class TextStyle(TextStyleBase, StyleBase[TextStyleData]):
    pass
