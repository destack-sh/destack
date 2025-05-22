from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    BuiltinEnum,
    BuiltinObject,
    EnumType,
    Node,
    NodeType,
    Struct,
    StructType,
    enum_,
    node_,
    object_,
    p_regular,
    struct_,
)
from bench.pb2 import ShadowStyleData

from .color import Color
from .core import Axis2, IsVariable
from .style import IsStyle

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.SHADOW_TYPE)
class ShadowType(BuiltinEnum):
    """Built-in shadow types."""

    STYLE = 2
    FIELD = 3
    BOX = 10
    REALISTIC = 11


@enum_(EnumType.SHADOW_POSITION)
class ShadowPosition(BuiltinEnum):
    """Built-in shadow positions."""

    OUTSIDE = 1
    INSIDE = 2


@object_()
class ShadowBase(IsVariable, BuiltinObject):
    type: ShadowType = p_regular(30, default=ShadowType.BOX)
    style: Optional["ShadowStyle"] = p_regular(41)
    color: Optional["Color"] = p_regular(50)
    position: ShadowPosition = p_regular(51, default=ShadowPosition.OUTSIDE)
    offset: Optional[Axis2] = p_regular(52)
    blur: int | None = p_regular(53)
    spread: int | None = p_regular(54)
    diffusion: float | None = p_regular(55)


@struct_(StructType.SHADOW)
class Shadow(ShadowBase, Struct):
    """A shadow value."""

    pass


@node_(NodeType.SHADOW_STYLE)
class ShadowStyle(ShadowBase, IsStyle, Node[ShadowStyleData]):
    """A shadow style."""

    pass
