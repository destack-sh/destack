from destack.language.core import (
    Enum,
    EnumType,
    NodeType,
    StructFrozen,
    StructType,
    Vector2f,
    builtin_enum,
    builtin_node,
    builtin_property,
    builtin_struct,
)

from .shape import Shape

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.ARROW_HEAD_TYPE)
class ArrowHeadType(Enum):
    ARROW = 1
    TRIANGLE = 2
    DOT = 3


@builtin_struct(StructType.ARROW, frozen=True)
class Arrow(StructFrozen):
    """An Arrow is a shape that represents an arrow."""

    start_type: ArrowHeadType = builtin_property(200)
    start: Vector2f = builtin_property(201)
    end_type: ArrowHeadType = builtin_property(210)
    end: Vector2f = builtin_property(211)


@builtin_node(NodeType.ARROW_SHAPE)
class ArrowShape(Shape):
    """An ArrowShape is a shape that represents an arrow."""

    # content
    start_type: ArrowHeadType = builtin_property(200)
    start: Vector2f = builtin_property(201)
    end_type: ArrowHeadType = builtin_property(210)
    end: Vector2f = builtin_property(211)
