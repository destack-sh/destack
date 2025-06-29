from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Axis2,
    Enum,
    EnumType,
    Node,
    NodeType,
    StructMutable,
    StructType,
    builtin_enum,
    builtin_node,
    builtin_struct,
    property_,
)

from .color import Color
from .style import Style

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.SHADOW_TYPE)
class ShadowType(Enum):
    """Built-in shadow types."""

    BOX = 10
    REALISTIC = 11


@builtin_enum(EnumType.SHADOW_POSITION)
class ShadowPosition(Enum):
    """Built-in shadow positions."""

    OUTSIDE = 1
    INSIDE = 2


@builtin_struct(StructType.SHADOW)
class Shadow(StructMutable):
    """A shadow value."""

    type: ShadowType = property_(30, default=ShadowType.BOX, is_repr=True)
    style: Optional["ShadowStyle"] = property_(41, is_repr=True)
    color: Optional["Color"] = property_(50, is_repr=True)
    position: ShadowPosition = property_(51, default=ShadowPosition.OUTSIDE, is_repr=True)
    offset: Optional[Axis2] = property_(52, is_repr=True)
    blur: int | None = property_(53, is_repr=True)
    spread: int | None = property_(54, is_repr=True)
    diffusion: float | None = property_(55, is_repr=True)


@builtin_node(NodeType.SHADOW_STYLE)
class ShadowStyle(
    Style,
    Node,
):
    """A shadow style."""

    type: ShadowType = property_(30, default=ShadowType.BOX, is_repr=True)
    color: Optional["Color"] = property_(50, is_repr=True)
    position: ShadowPosition = property_(51, default=ShadowPosition.OUTSIDE, is_repr=True)
    offset: Optional[Axis2] = property_(52, is_repr=True)
    blur: int | None = property_(53, is_repr=True)
    spread: int | None = property_(54, is_repr=True)
    diffusion: float | None = property_(55, is_repr=True)
