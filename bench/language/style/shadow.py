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
from bench.pb2 import ShadowStyleData

from .color import Color
from .style import StyleBase


@enum_(EnumType.SHADOW_TYPE)
class ShadowType(BuiltinEnum):
    """Built-in shadow types."""

    STYLE = 2
    BOX = 10
    REALISTIC = 11


@enum_(EnumType.SHADOW_POSITION)
class ShadowPosition(BuiltinEnum):
    """Built-in shadow positions."""

    OUTSIDE = 1
    INSIDE = 2


@node_component_()
class ShadowBase(BuiltinObject):
    type: ShadowType = p_regular(30, default=ShadowType.BOX)
    style: Optional["ShadowStyle"] = p_regular(
        40, default=None, require=False, array=False, references=NodeType.SHADOW_STYLE
    )
    position: ShadowPosition = p_regular(41, default=ShadowPosition.OUTSIDE)
    offset_x: int = p_regular(42, default=0)
    offset_y: int = p_regular(43, default=0)
    blur: int = p_regular(44, default=2)
    spread: int = p_regular(45, default=0)
    color: Optional["Color"] = p_regular(46, array=False, default=None, struct=StructType.COLOR)


@struct_(StructType.SHADOW)
class Shadow(ShadowBase, Struct):
    """A shadow value."""

    pass


@node_(NodeType.SHADOW_STYLE)
class ShadowStyle(ShadowBase, StyleBase[ShadowStyleData]):
    """A shadow style."""

    pass
