from typing import TYPE_CHECKING, Optional, final

from destack.core import (
    EnumType,
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
    from destack import Inset2


@declare_enum(EnumType.BORDER_TYPE)
class BorderType(OptionEnum):
    """Built-in border types."""

    STYLE = declare_option(2, "Style", description="A border style")
    SOLID = declare_option(10, "Solid", description="A solid border")
    DASHED = declare_option(11, "Dashed", description="A dashed border")
    DOTTED = declare_option(12, "Dotted", description="A dotted border")
    DOUBLE = declare_option(13, "Double", description="A double border")


@declare_struct(
    StructType.BORDER,
    is_final=True,
    into_node_types=(NodeType.BORDER_STYLE,),
)
@final
class Border(Struct):
    """A border value."""

    type: BorderType = declare_property(
        100,
        default=BorderType.SOLID,
        is_repr=True,
        tag=None,
    )
    color: Optional["Color"] = declare_property(101, is_repr=True, tag=None)
    width: Optional["Inset2"] = declare_property(102, is_repr=True, tag=None)
    template: Optional["BorderStyle"] = declare_property(
        103,
        is_repr=True,
        reference_type=ReferenceType.SPATIAL,
        tag=None,
    )


@declare_entity(
    NodeType.BORDER_STYLE,
    base_struct_type=StructType.BORDER,
)
class BorderStyle(Style):
    """A border style."""

    type: BorderType = declare_property(
        100,
        default=BorderType.SOLID,
        is_repr=True,
        tag=None,
    )
    color: Optional["Color"] = declare_property(101, is_repr=True, tag=None)
    width: Optional["Inset2"] = declare_property(102, is_repr=True, tag=None)
