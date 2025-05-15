from bench.language.core import BuiltinEnum, EnumType, Struct, StructType, enum_, p_regular, struct_


@enum_(EnumType.LAYOUT)
class Layout(BuiltinEnum):
    STACK = 1
    GRID = 2


@enum_(EnumType.DIRECTION)
class Direction(BuiltinEnum):
    HORIZONTAL = 1
    VERTICAL = 2


@enum_(EnumType.DISTRIBUTE)
class Distribute(BuiltinEnum):
    START = 1
    CENTER = 2
    END = 3
    SPACE_BETWEEN = 4
    SPACE_AROUND = 5
    SPACE_EVENLY = 6


@enum_(EnumType.ALIGN)
class Align(BuiltinEnum):
    START = 1
    CENTER = 2
    END = 3


@enum_(EnumType.POSITION_TYPE)
class PositionType(BuiltinEnum):
    """The position of a View."""

    RELATIVE = 1
    ABSOLUTE = 2
    STICKY = 3


@enum_(EnumType.LENGTH_UNIT)
class LengthUnit(BuiltinEnum):
    PIXEL = 1
    REM = 2


@struct_(StructType.LENGTH)
class Length(Struct):
    unit: LengthUnit = p_regular(31)
    value: float = p_regular(32)


@struct_(StructType.POSITION)
class Position(Struct):
    """The position of a View."""

    type: PositionType = p_regular(40)
    top: float | None = p_regular(41, require=False, default=None)
    left: float | None = p_regular(42, require=False, default=None)
    width: float | None = p_regular(43, require=False, default=None)
    height: float | None = p_regular(44, require=False, default=None)
