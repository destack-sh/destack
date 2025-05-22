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
    object_,
    p_regular,
    struct_,
)
from bench.pb2 import BorderStyleData

from .color import Color
from .core import Insets, IsVariable
from .style import StyleBase

if TYPE_CHECKING:
    pass


@enum_(EnumType.BORDER_TYPE)
class BorderType(BuiltinEnum):
    """Built-in border types."""

    NONE = 1
    STYLE = 2
    FIELD = 3
    SOLID = 10
    DASHED = 11
    DOTTED = 12
    DOUBLE = 13


@object_()
class BorderBase(IsVariable, BuiltinObject):
    type: BorderType = p_regular(30, default=BorderType.SOLID)
    style: Optional["BorderStyle"] = p_regular(41)
    color: Optional["Color"] = p_regular(50)
    width: Optional[Insets] = p_regular(51)


@struct_(StructType.BORDER)
class Border(BorderBase, Struct):
    """A border value."""

    pass


@node_(NodeType.BORDER_STYLE)
class BorderStyle(BorderBase, StyleBase[BorderStyleData]):
    """A border style."""

    pass
