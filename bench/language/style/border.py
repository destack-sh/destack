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
    STYLE = 2
    SOLID = 10
    DASHED = 11
    DOTTED = 12
    DOUBLE = 13


@node_component_()
class BorderBase(BuiltinObject):
    type: BorderType = p_regular(30, default=BorderType.SOLID)
    style: Optional["BorderStyle"] = p_regular(
        40, default=None, require=False, array=False, references=NodeType.BORDER_STYLE
    )
    color: Optional["Color"] = p_regular(42, array=False, default=None, struct=StructType.COLOR)
    width: Optional[int] = p_regular(43, default=None)
    width_top: Optional[int] = p_regular(44, default=None)
    width_right: Optional[int] = p_regular(45, default=None)
    width_bottom: Optional[int] = p_regular(46, default=None)
    width_left: Optional[int] = p_regular(47, default=None)
    radius: Optional[int] = p_regular(50, default=None)
    radius_top_left: Optional[int] = p_regular(51, default=None)
    radius_top_right: Optional[int] = p_regular(52, default=None)
    radius_bottom_right: Optional[int] = p_regular(53, default=None)
    radius_bottom_left: Optional[int] = p_regular(54, default=None)


@struct_(StructType.BORDER)
class Border(BorderBase, Struct):
    """A border value."""

    pass


@node_(NodeType.BORDER_STYLE)
class BorderStyle(BorderBase, StyleBase[BorderStyleData]):
    """A border style."""

    pass
