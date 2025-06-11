from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Axis2,
    BuiltinEnum,
    BuiltinObjectMutable,
    EnumType,
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
from destack.pb2 import ShadowStyleData

from .color import Color
from .style import Style

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
class ShadowBase(BuiltinObjectMutable):
    type: ShadowType = property_(30, default=ShadowType.BOX, is_repr=True)
    style: Optional["ShadowStyle"] = property_(41, is_repr=True)
    color: Optional["Color"] = property_(50, is_repr=True)
    position: ShadowPosition = property_(51, default=ShadowPosition.OUTSIDE, is_repr=True)
    offset: Optional[Axis2] = property_(52, is_repr=True)
    blur: int | None = property_(53, is_repr=True)
    spread: int | None = property_(54, is_repr=True)
    diffusion: float | None = property_(55, is_repr=True)


@struct_(StructType.SHADOW)
class Shadow(ShadowBase, StructMutable):
    """A shadow value."""

    pass


@node_(NodeType.SHADOW_STYLE)
class ShadowStyle(
    Style,
    ShadowBase,
    Node[ShadowStyleData],
):
    """A shadow style."""

    pass
