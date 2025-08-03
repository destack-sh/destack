from typing import TYPE_CHECKING, Optional, final

from destack.core import (
    Enum,
    EnumType,
    Float32,
    NodeType,
    StructFrozen,
    StructType,
    declare_entity,
    declare_enum,
    declare_property,
    declare_struct,
)

from .color import Color
from .style import Style

if TYPE_CHECKING:
    from destack import Axis2

# pyright: reportIncompatibleVariableOverride=false


@declare_enum(EnumType.SHADOW_TYPE)
class ShadowType(Enum):
    """Built-in shadow types."""

    BOX = 10
    REALISTIC = 11


@declare_enum(EnumType.SHADOW_POSITION)
class ShadowPosition(Enum):
    """Built-in shadow positions."""

    OUTSIDE = 1
    INSIDE = 2


@declare_struct(
    StructType.SHADOW,
    frozen=True,
    is_final=True,
)
@final
class Shadow(StructFrozen):
    """A shadow value."""

    type: ShadowType = declare_property(100, default=ShadowType.BOX, is_repr=True)
    style: Optional["ShadowStyle"] = declare_property(101, is_repr=True)
    color: Optional["Color"] = declare_property(102, is_repr=True)
    position: ShadowPosition = declare_property(103, default=ShadowPosition.OUTSIDE, is_repr=True)
    offset: Optional["Axis2"] = declare_property(104, is_repr=True)
    blur: Optional[Float32] = declare_property(105, is_repr=True)
    spread: Optional[Float32] = declare_property(106, is_repr=True)
    diffusion: Optional[Float32] = declare_property(107, is_repr=True)


@declare_entity(NodeType.SHADOW_STYLE)
class ShadowStyle(Style):
    """A shadow style."""

    type: ShadowType = declare_property(100, default=ShadowType.BOX, is_repr=True)
    color: Optional["Color"] = declare_property(200, is_repr=True)
    position: ShadowPosition = declare_property(201, default=ShadowPosition.OUTSIDE, is_repr=True)
    offset: Optional["Axis2"] = declare_property(202, is_repr=True)
    blur: Optional[Float32] = declare_property(203, is_repr=True)
    spread: Optional[Float32] = declare_property(204, is_repr=True)
    diffusion: Optional[Float32] = declare_property(205, is_repr=True)
