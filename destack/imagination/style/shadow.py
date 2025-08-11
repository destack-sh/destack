from typing import TYPE_CHECKING, Optional, final

from destack.core import (
    EnumType,
    Float32,
    NodeType,
    OptionEnum,
    ReferenceType,
    Struct,
    StructType,
    declare_entity,
    declare_enum,
    declare_option,
    declare_property,
    declare_struct,
)

from .color import Color
from .style import Style

if TYPE_CHECKING:
    from destack import Axis2


@declare_enum(EnumType.SHADOW_TYPE)
class ShadowType(OptionEnum):
    """Built-in shadow types."""

    BOX = declare_option(10, "Box", description="A box shadow")
    REALISTIC = declare_option(11, "Realistic", description="A realistic shadow")


@declare_enum(EnumType.SHADOW_POSITION)
class ShadowPosition(OptionEnum):
    """Built-in shadow positions."""

    OUTSIDE = declare_option(1, "Outside", description="An outside shadow")
    INSIDE = declare_option(2, "Inside", description="An inside shadow")


@declare_struct(
    StructType.SHADOW,
    is_final=True,
    into_node_types=(NodeType.SHADOW_STYLE,),
)
@final
class Shadow(Struct):
    """A shadow value."""

    type: ShadowType = declare_property(100, default=ShadowType.BOX, is_repr=True, tag=None)
    template: Optional["ShadowStyle"] = declare_property(
        101,
        is_repr=True,
        reference_type=ReferenceType.LOCATION,
        tag=None,
    )
    color: Optional["Color"] = declare_property(102, is_repr=True, tag=None)
    position: ShadowPosition = declare_property(
        103, default=ShadowPosition.OUTSIDE, is_repr=True, tag=None
    )
    offset: Optional["Axis2"] = declare_property(104, is_repr=True, tag=None)
    blur: Optional[Float32] = declare_property(105, is_repr=True, tag=None)
    spread: Optional[Float32] = declare_property(106, is_repr=True, tag=None)
    diffusion: Optional[Float32] = declare_property(107, is_repr=True, tag=None)


@declare_entity(
    NodeType.SHADOW_STYLE,
    base_struct_type=StructType.SHADOW,
)
class ShadowStyle(Style):
    """A shadow style."""

    type: ShadowType = declare_property(100, default=ShadowType.BOX, is_repr=True, tag=None)
    color: Optional["Color"] = declare_property(101, is_repr=True, tag=None)
    position: ShadowPosition = declare_property(
        102, default=ShadowPosition.OUTSIDE, is_repr=True, tag=None
    )
    offset: Optional["Axis2"] = declare_property(103, is_repr=True, tag=None)
    blur: Optional[Float32] = declare_property(104, is_repr=True, tag=None)
    spread: Optional[Float32] = declare_property(105, is_repr=True, tag=None)
    diffusion: Optional[Float32] = declare_property(106, is_repr=True, tag=None)
