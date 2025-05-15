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
    FIXED = 3
    STICKY = 4


@enum_(EnumType.LENGTH_UNIT)
class LengthUnit(BuiltinEnum):
    PIXEL = 1
    REM = 2
    PERCENT = 3
    FR = 4


@struct_(StructType.LENGTH)
class Length(Struct):
    """A length value."""

    unit: LengthUnit = p_regular(31)
    value: float = p_regular(40)


@struct_(StructType.POSITION)
class Position(Struct):
    """A position value."""

    type: PositionType = p_regular(40)
    top: Length | None = p_regular(
        41, require=False, array=False, default=None, struct=StructType.LENGTH
    )
    left: Length | None = p_regular(
        42, require=False, array=False, default=None, struct=StructType.LENGTH
    )
    width: Length | None = p_regular(
        43, require=False, array=False, default=None, struct=StructType.LENGTH
    )
    height: Length | None = p_regular(
        44, require=False, array=False, default=None, struct=StructType.LENGTH
    )


@enum_(EnumType.DIMENSION_TYPE)
class DimensionType(BuiltinEnum):
    FIXED = 1
    RELATIVE = 2
    FIT_CONTENT = 3
    FILL = 4
    ASPECT_RATIO = 5


@struct_(StructType.DIMENSION)
class Dimension(Struct):
    """A dimension value."""

    type: DimensionType = p_regular(30)
    value: float = p_regular(40)


@struct_(StructType.PADDING)
class Padding(Struct):
    """A padding value."""

    value: int | None = p_regular(40, default=None)
    top: int | None = p_regular(41, default=None)
    left: int | None = p_regular(42, default=None)
    right: int | None = p_regular(43, default=None)
    bottom: int | None = p_regular(44, default=None)


@struct_(StructType.ROTATION)
class Rotation(Struct):
    """A rotation value."""

    value: float | None = p_regular(40, default=None)
    x: float | None = p_regular(41, default=None)
    y: float | None = p_regular(42, default=None)
    z: float | None = p_regular(43, default=None)


@struct_(StructType.GAP)
class Gap(Struct):
    """A gap value."""

    value: float | None = p_regular(40, default=None)
    x: float | None = p_regular(41, default=None)
    y: float | None = p_regular(42, default=None)


@struct_(StructType.GRID)
class Grid(Struct):
    """A grid value."""

    columns: int = p_regular(40)
    rows: int = p_regular(41)
    column_width: float | None = p_regular(
        42, require=False, array=False, default=None, struct=StructType.DIMENSION
    )
    column_min_width: float | None = p_regular(
        43, require=False, array=False, default=None, struct=StructType.DIMENSION
    )
    column_width_type: DimensionType | None = p_regular(45, require=False)


@struct_(StructType.GRID_SPAN)
class GridSpan(Struct):
    """A grid span value."""

    columns: int = p_regular(40)
    rows: int = p_regular(41)
