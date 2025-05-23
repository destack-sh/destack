from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    BuiltinEnum,
    BuiltinObject,
    EnumType,
    IsArchivable,
    IsDeletable,
    Node,
    NodeType,
    Struct,
    StructType,
    enum_,
    node_,
    object_,
    property_,
    struct_,
)
from bench.pb2 import ShadowStyleData

from .color import Color
from .core import Axis2
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
class ShadowBase(BuiltinObject):
    type: ShadowType = property_(30, default=ShadowType.BOX)
    style: Optional["ShadowStyle"] = property_(41)
    color: Optional["Color"] = property_(50)
    position: ShadowPosition = property_(51, default=ShadowPosition.OUTSIDE)
    offset: Optional[Axis2] = property_(52)
    blur: int | None = property_(53)
    spread: int | None = property_(54)
    diffusion: float | None = property_(55)


@struct_(StructType.SHADOW)
class Shadow(ShadowBase, Struct):
    """A shadow value."""

    pass


@node_(NodeType.SHADOW_STYLE)
class ShadowStyle(
    ShadowBase,
    IsStyle,
    IsDeletable,
    IsArchivable,
    Node[ShadowStyleData],
):
    """A shadow style."""

    pass
