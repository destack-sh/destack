from destack.language.core import (
    Enum,
    EnumType,
    NodeType,
    StructFrozen,
    StructType,
    Vector2,
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

    start_type: ArrowHeadType = builtin_property(100)
    start: Vector2 = builtin_property(101)
    end_type: ArrowHeadType = builtin_property(110)
    end: Vector2 = builtin_property(111)


@builtin_node(NodeType.ARROW_SHAPE, pretend_frozen=True)
class ArrowShape(Shape):
    """An ArrowShape is a shape that represents an arrow."""

    # content
    start_type: ArrowHeadType = builtin_property(100)
    start: Vector2 = builtin_property(101)
    end_type: ArrowHeadType = builtin_property(110)
    end: Vector2 = builtin_property(111)
