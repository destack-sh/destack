from typing import TYPE_CHECKING, Optional

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
from .core import Axis2
from .style import StyleBase

if TYPE_CHECKING:
    from bench.language import Field


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


@node_component_()
class ShadowBase(BuiltinObject):
    type: ShadowType = p_regular(30, default=ShadowType.BOX)
    style: Optional["ShadowStyle"] = p_regular(
        40, default=None, require=False, array=False, references=NodeType.SHADOW_STYLE
    )
    field: Optional["Field"] = p_regular(
        41, default=None, require=False, array=False, references=NodeType.FIELD
    )
    color: Optional["Color"] = p_regular(
        50, array=False, default=None, require=False, struct=StructType.COLOR
    )
    position: ShadowPosition = p_regular(51, default=ShadowPosition.OUTSIDE)
    offset: Optional[Axis2] = p_regular(52, default=None, struct=StructType.AXIS_2)
    blur: int | None = p_regular(53, default=None)
    spread: int | None = p_regular(54, default=None)
    diffusion: float | None = p_regular(55, default=None)


@struct_(StructType.SHADOW)
class Shadow(ShadowBase, Struct):
    """A shadow value."""

    pass


@node_(NodeType.SHADOW_STYLE)
class ShadowStyle(ShadowBase, StyleBase[ShadowStyleData]):
    """A shadow style."""

    pass
