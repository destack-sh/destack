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

from .color import Color
from .style import StyleBase


@enum_(EnumType.SHADOW_TYPE)
class ShadowType(BuiltinEnum):
    """Built-in shadow types."""

    BOX = 1
    REALISTIC = 2


@enum_(EnumType.SHADOW_POSITION)
class ShadowPosition(BuiltinEnum):
    """Built-in shadow positions."""

    OUTSIDE = 1
    INSIDE = 2


@node_component_()
class ShadowBase(BuiltinObject):
    type: ShadowType = p_regular(30, default=ShadowType.BOX)
    position: ShadowPosition = p_regular(40, default=ShadowPosition.OUTSIDE)
    offset_x: str = p_regular(41)
    offset_y: str = p_regular(42)
    blur: str = p_regular(43)
    spread: str = p_regular(44)
    color: Optional["Color"] = p_regular(45, array=False, default=None, struct=StructType.COLOR)


@struct_(StructType.SHADOW)
class Shadow(ShadowBase, Struct):
    """A shadow value."""

    pass


@node_(NodeType.SHADOW_STYLE)
class ShadowStyle(ShadowBase, StyleBase):
    """A shadow style."""

    pass
