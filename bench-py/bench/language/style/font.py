from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    BuiltinEnum,
    BuiltinObjectMutable,
    EnumType,
    IsArchivable,
    IsDeletable,
    IsTracked,
    Node,
    NodeType,
    StructMutable,
    StructType,
    enum_,
    node_,
    object_,
    property_,
    struct_,
)
from bench.pb2 import FontStyleData

from .core import Length
from .fill import Fill
from .style import IsStyle

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


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


@object_()
class FontBase(BuiltinObjectMutable):
    """A text style value."""

    type: FontType = property_(30, default=FontType.SANS, is_repr=True)
    style: Optional["FontStyle"] = property_(41, is_repr=True)
    weight: Optional[FontWeight] = property_(50, default=FontWeight.NORMAL, is_repr=True)
    color: Optional[Fill] = property_(51, is_repr=True)
    size: Optional[FontSize] = property_(52, default=FontSize.BASE, is_repr=True)
    align: Optional[TextAlign] = property_(53, default=TextAlign.LEFT, is_repr=True)
    line_height: Optional[Length] = property_(54, is_repr=True)
    letter_spacing: Optional[Length] = property_(55, is_repr=True)
    decoration: Optional[TextDecoration] = property_(56, default=TextDecoration.NONE, is_repr=True)
    transform: Optional[TextTransform] = property_(57, default=TextTransform.NONE, is_repr=True)


@struct_(StructType.FONT)
class Font(FontBase, StructMutable):
    """A font value."""

    pass


@node_(NodeType.FONT_STYLE)
class FontStyle(
    FontBase,
    IsStyle,
    IsDeletable,
    IsArchivable,
    IsTracked,
    Node[FontStyleData],
):
    """A font style."""

    pass
