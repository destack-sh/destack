from typing import TYPE_CHECKING

from destack.language.core import (
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

from .shape import Shape2D

if TYPE_CHECKING:
    from destack.language import Vector2

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.ARROW_HEAD_TYPE)
class ArrowHeadType(Enum):
    ARROW = 1
    TRIANGLE = 2
    DOT = 3


@builtin_struct(StructType.ARROW2D, frozen=True)
class Arrow2D(StructFrozen):
    """An Arrow is a shape that represents an arrow."""

    start_type: ArrowHeadType = builtin_property(200)
    start: "Vector2" = builtin_property(201)
    end_type: ArrowHeadType = builtin_property(210)
    end: "Vector2" = builtin_property(211)


@builtin_node(NodeType.ARROW_SHAPE2D)
class ArrowShape2D(Shape2D):
    """An ArrowShape is a shape that represents an arrow."""

    # content
    start_type: ArrowHeadType = builtin_property(200)
    start: "Vector2" = builtin_property(201)
    end_type: ArrowHeadType = builtin_property(210)
    end: "Vector2" = builtin_property(211)
