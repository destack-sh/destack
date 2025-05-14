from typing import Optional

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
from bench.pb2 import BorderStyleData

from .color import Color
from .style import StyleBase


@enum_(EnumType.BORDER_TYPE)
class BorderType(BuiltinEnum):
    """Built-in border types."""

    NONE = 1
    SOLID = 2
    DASHED = 3
    DOTTED = 4
    DOUBLE = 5


@node_component_()
class BorderBase(BuiltinObject):
    type: BorderType = p_regular(30, default=BorderType.SOLID)
    width: Optional[int] = p_regular(40, default=None)
    color: Optional["Color"] = p_regular(50, array=False, default=None, struct=StructType.COLOR)
    radius: Optional[int] = p_regular(60, default=None)
    radius_top_left: Optional[int] = p_regular(61, default=None)
    radius_top_right: Optional[int] = p_regular(62, default=None)
    radius_bottom_right: Optional[int] = p_regular(63, default=None)
    radius_bottom_left: Optional[int] = p_regular(64, default=None)


@struct_(StructType.BORDER)
class Border(BorderBase, Struct):
    """A border value."""

    pass


@node_(NodeType.BORDER_STYLE)
class BorderStyle(BorderBase, StyleBase[BorderStyleData]):
    """A border style."""

    pass
