from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Axis2,
    Enum,
    EnumType,
    NodeType,
    StructFrozen,
    StructType,
    builtin_enum,
    builtin_node,
    builtin_property,
    builtin_struct,
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


@builtin_struct(StructType.SHADOW, frozen=True)
class Shadow(StructFrozen):
    """A shadow value."""

    type: ShadowType = builtin_property(30, default=ShadowType.BOX, is_repr=True)
    style: Optional["ShadowStyle"] = builtin_property(41, is_repr=True)
    color: Optional["Color"] = builtin_property(50, is_repr=True)
    position: ShadowPosition = builtin_property(51, default=ShadowPosition.OUTSIDE, is_repr=True)
    offset: Optional[Axis2] = builtin_property(52, is_repr=True)
    blur: int | None = builtin_property(53, is_repr=True)
    spread: int | None = builtin_property(54, is_repr=True)
    diffusion: float | None = builtin_property(55, is_repr=True)


@builtin_node(NodeType.SHADOW_STYLE)
class ShadowStyle(Style):
    """A shadow style."""

    type: ShadowType = builtin_property(30, default=ShadowType.BOX, is_repr=True)
    color: Optional["Color"] = builtin_property(50, is_repr=True)
    position: ShadowPosition = builtin_property(51, default=ShadowPosition.OUTSIDE, is_repr=True)
    offset: Optional[Axis2] = builtin_property(52, is_repr=True)
    blur: int | None = builtin_property(53, is_repr=True)
    spread: int | None = builtin_property(54, is_repr=True)
    diffusion: float | None = builtin_property(55, is_repr=True)
