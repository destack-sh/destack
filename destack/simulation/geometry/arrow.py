from typing import TYPE_CHECKING, final

from destack.core import (
    EnumDeclaration,
    EnumType,
    NodeType,
    StructType,
    declare_entity,
    declare_enum,
    declare_property,
    declare_struct,
)

from .shape import Form2D, Shape2D

if TYPE_CHECKING:
    from destack import Vector2

# pyright: reportIncompatibleVariableOverride=false


@declare_enum(EnumType.ARROW_HEAD_TYPE)
class ArrowHeadType(EnumDeclaration):
    ARROW = 1
    TRIANGLE = 2
    DOT = 3


@declare_struct(
    StructType.ARROW2D,
    frozen=True,
    is_final=True,
)
@final
class Arrow2D(Form2D):
    """An Arrow is a shape that represents an arrow."""

    start_type: ArrowHeadType = declare_property(200, is_repr=True)
    start: "Vector2" = declare_property(201, is_repr=True)
    end_type: ArrowHeadType = declare_property(210, is_repr=True)
    end: "Vector2" = declare_property(211, is_repr=True)


@declare_entity(NodeType.ARROW_SHAPE2D)
class ArrowShape2D(Shape2D):
    """An ArrowShape is a shape that represents an arrow."""

    # content
    start_type: ArrowHeadType = declare_property(200, is_repr=True)
    start: "Vector2" = declare_property(201, is_repr=True)
    end_type: ArrowHeadType = declare_property(210, is_repr=True)
    end: "Vector2" = declare_property(211, is_repr=True)
