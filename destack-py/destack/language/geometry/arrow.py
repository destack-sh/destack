from typing import TYPE_CHECKING, final

from destack.language.core import (
    Enum,
    EnumType,
    NodeType,
    StructType,
    builtin_entity,
    builtin_enum,
    builtin_property,
    builtin_struct,
)

from .shape import Form2D, Shape2D

if TYPE_CHECKING:
    from destack.language import Vector2

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.ARROW_HEAD_TYPE)
class ArrowHeadType(Enum):
    ARROW = 1
    TRIANGLE = 2
    DOT = 3


@builtin_struct(
    StructType.ARROW2D,
    frozen=True,
    is_final=True,
)
@final
class Arrow2D(Form2D):
    """An Arrow is a shape that represents an arrow."""

    start_type: ArrowHeadType = builtin_property(200)
    start: "Vector2" = builtin_property(201)
    end_type: ArrowHeadType = builtin_property(210)
    end: "Vector2" = builtin_property(211)


@builtin_entity(NodeType.ARROW_SHAPE2D)
class ArrowShape2D(Shape2D):
    """An ArrowShape is a shape that represents an arrow."""

    # content
    start_type: ArrowHeadType = builtin_property(200)
    start: "Vector2" = builtin_property(201)
    end_type: ArrowHeadType = builtin_property(210)
    end: "Vector2" = builtin_property(211)
