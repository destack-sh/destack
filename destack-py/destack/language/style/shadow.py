from typing import TYPE_CHECKING, Optional, final

from destack.language.core import (
    Enum,
    EnumType,
    Float32,
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
    from destack.language import Axis2

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


@builtin_struct(
    StructType.SHADOW,
    frozen=True,
    is_final=True,
)
@final
class Shadow(StructFrozen):
    """A shadow value."""

    type: ShadowType = builtin_property(100, default=ShadowType.BOX, is_repr=True)
    style: Optional["ShadowStyle"] = builtin_property(101, is_repr=True)
    color: Optional["Color"] = builtin_property(102, is_repr=True)
    position: ShadowPosition = builtin_property(103, default=ShadowPosition.OUTSIDE, is_repr=True)
    offset: Optional["Axis2"] = builtin_property(104, is_repr=True)
    blur: Optional[Float32] = builtin_property(105, is_repr=True)
    spread: Optional[Float32] = builtin_property(106, is_repr=True)
    diffusion: Optional[Float32] = builtin_property(107, is_repr=True)


@builtin_node(NodeType.SHADOW_STYLE)
class ShadowStyle(Style):
    """A shadow style."""

    type: ShadowType = builtin_property(100, default=ShadowType.BOX, is_repr=True)
    color: Optional["Color"] = builtin_property(200, is_repr=True)
    position: ShadowPosition = builtin_property(201, default=ShadowPosition.OUTSIDE, is_repr=True)
    offset: Optional["Axis2"] = builtin_property(202, is_repr=True)
    blur: Optional[Float32] = builtin_property(203, is_repr=True)
    spread: Optional[Float32] = builtin_property(204, is_repr=True)
    diffusion: Optional[Float32] = builtin_property(205, is_repr=True)
