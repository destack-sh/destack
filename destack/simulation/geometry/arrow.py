from typing import TYPE_CHECKING, final

from destack.core import (
    EnumType,
    NodeType,
    OptionEnum,
    StructType,
    declare_entity,
    declare_enum,
    declare_option,
    declare_property,
    declare_struct,
)

from .shape import Form2D, Shape2D

if TYPE_CHECKING:
    from destack import Vector2


@declare_enum(EnumType.ARROW_HEAD_TYPE)
class ArrowHeadType(OptionEnum):
    ARROW = declare_option(1)
    TRIANGLE = declare_option(2)
    DOT = declare_option(3)


@declare_struct(
    StructType.ARROW2D,
    is_final=True,
    into_node_types=(NodeType.ARROW_SHAPE2D,),
)
@final
class Arrow2D(Form2D):
    """An Arrow is a shape that represents an arrow."""

    start_type: ArrowHeadType = declare_property(200, is_repr=True, tag=None)
    start: "Vector2" = declare_property(201, is_repr=True, tag=None)
    end_type: ArrowHeadType = declare_property(210, is_repr=True, tag=None)
    end: "Vector2" = declare_property(211, is_repr=True, tag=None)


@declare_entity(NodeType.ARROW_SHAPE2D)
class ArrowShape2D(Shape2D):
    """An ArrowShape is a shape that represents an arrow."""

    # content
    start_type: ArrowHeadType = declare_property(200, is_repr=True, tag=None)
    start: "Vector2" = declare_property(201, is_repr=True, tag=None)
    end_type: ArrowHeadType = declare_property(210, is_repr=True, tag=None)
    end: "Vector2" = declare_property(211, is_repr=True, tag=None)
