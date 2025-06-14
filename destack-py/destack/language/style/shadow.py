from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Axis2,
    BuiltinObjectMutable,
    Enum,
    EnumType,
    Node,
    NodeType,
    StructMutable,
    StructType,
    builtin_enum,
    builtin_node,
    builtin_struct,
    object_,
    property_,
)
from destack.pb2 import ShadowStyleData

from .color import Color
from .style import Style

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.SHADOW_TYPE)
class ShadowType(Enum):
    """Built-in shadow types."""

    STYLE = 2
    BOX = 10
    REALISTIC = 11


@builtin_enum(EnumType.SHADOW_POSITION)
class ShadowPosition(Enum):
    """Built-in shadow positions."""

    OUTSIDE = 1
    INSIDE = 2


@object_()
class ShadowBase(BuiltinObjectMutable):
    type: ShadowType = property_(30, default=ShadowType.BOX, is_repr=True)
    color: Optional["Color"] = property_(50, is_repr=True)
    position: ShadowPosition = property_(51, default=ShadowPosition.OUTSIDE, is_repr=True)
    offset: Optional[Axis2] = property_(52, is_repr=True)
    blur: int | None = property_(53, is_repr=True)
    spread: int | None = property_(54, is_repr=True)
    diffusion: float | None = property_(55, is_repr=True)


@builtin_struct(StructType.SHADOW)
class Shadow(ShadowBase, StructMutable):
    """A shadow value."""

    style: Optional["ShadowStyle"] = property_(41, is_repr=True)


@builtin_node(NodeType.SHADOW_STYLE)
class ShadowStyle(
    Style,
    ShadowBase,
    Node[ShadowStyleData],
):
    """A shadow style."""

    pass
