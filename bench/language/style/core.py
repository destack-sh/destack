from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    BuiltinEnum,
    BuiltinObject,
    EnumType,
    NodeType,
    Struct,
    StructType,
    enum_,
    object_,
    p_regular,
    struct_,
)

if TYPE_CHECKING:
    from bench.language import Field

# pyright: reportIncompatibleVariableOverride=false


@object_()
class IsVariable(BuiltinObject):
    """A value that can be tied to a Field."""

    field: Optional["Field"] = p_regular(40, require=False, array=False, references=NodeType.FIELD)


@enum_(EnumType.LAYOUT)
class Layout(BuiltinEnum):
    """The layout of a View."""

    STACK = 1, "Stack", "Stack", "fas fa-objects-align-center-vertical"
    GRID = 2, "Grid", "Grid", "fas fa-grid-2"


@enum_(EnumType.OVERFLOW)
class Overflow(BuiltinEnum):
    """The overflow behavior of a View."""

    HIDDEN = 2, "Hidden", "Hidden", "fas fa-eye-slash"
    VISIBLE = 3, "Visible", "Visible", "fas fa-eye"
    SCROLL = 4, "Scroll", "Scroll", "fas fa-computer-mouse-scrollwheel"


@enum_(EnumType.DIRECTION)
class Direction(BuiltinEnum):
    """The direction of a View."""

    HORIZONTAL = 1, "Horizontal", "Horizontal", "fas fa-left-right"
    VERTICAL = 2, "Vertical", "Vertical", "fas fa-up-down"


@enum_(EnumType.DISTRIBUTE)
class Distribute(BuiltinEnum):
    """The distribution of a View's children."""

    START = 1, "Start", "Start", "fas fa-align-left"
    CENTER = 2, "Center", "Center", "fas fa-align-center"
    END = 3, "End", "End", "fas fa-align-right"
    SPACE_BETWEEN = 4, "Space Between", "Space Between"
    SPACE_AROUND = 5, "Space Around", "Space Around"
    SPACE_EVENLY = 6, "Space Evenly", "Space Evenly"


@enum_(EnumType.ALIGN)
class Align(BuiltinEnum):
    """The alignment of a View."""

    START = 1, "Start", "Start", "fas fa-align-left"
    CENTER = 2, "Center", "Center", "fas fa-align-center"
    END = 3, "End", "End", "fas fa-align-right"


@enum_(EnumType.LENGTH_UNIT)
class LengthUnit(BuiltinEnum):
    """The unit of a length value."""

    PIXEL = 1, "Pixel", "px"
    REM = 2, "Rem", "rem"
    PERCENT = 3, "Percent", "%"
    FR = 4, "Fr", "fr"


@struct_(StructType.LENGTH)
class Length(Struct):
    """A length value."""

    unit: LengthUnit = p_regular(50)
    value: float = p_regular(51)


@enum_(EnumType.POSITION_TYPE)
class PositionType(BuiltinEnum):
    """The position type of a View."""

    RELATIVE = 1, "Relative", "Relative"
    ABSOLUTE = 2, "Absolute", "Absolute"
    FIXED = 3, "Fixed", "Fixed"
    STICKY = 4, "Sticky", "Sticky"


@struct_(StructType.POSITION)
class Position(Struct):
    """A position value."""

    type: PositionType = p_regular(30)
    top: Length | None = p_regular(
        50, require=False, array=False, default=None, struct=StructType.LENGTH
    )
    left: Length | None = p_regular(
        51, require=False, array=False, default=None, struct=StructType.LENGTH
    )
    width: Length | None = p_regular(
        52, require=False, array=False, default=None, struct=StructType.LENGTH
    )
    height: Length | None = p_regular(
        53, require=False, array=False, default=None, struct=StructType.LENGTH
    )


@enum_(EnumType.DIMENSION_TYPE)
class DimensionType(BuiltinEnum):
    FIXED = 2, "Fixed", "Fixed", "fas fa-ruler-horizontal"
    FIT = 3, "Fit", "Fit", "fas fa-arrows-up-to-line"
    FILL = 4, "Fill", "Fill", "fas fa-arrows-from-dotted-line"


@struct_(StructType.DIMENSION)
class Dimension(Struct):
    """A dimension value (like Length but can fit or fill container)."""

    type: DimensionType = p_regular(30)
    unit: LengthUnit = p_regular(50)
    value: float = p_regular(51)


@struct_(StructType.INSETS)
class Insets(Struct):
    """An insets value (base + top/left/right/bottom)."""

    base: int | None = p_regular(50, default=None)
    top: int | None = p_regular(51, default=None)
    left: int | None = p_regular(52, default=None)
    right: int | None = p_regular(53, default=None)
    bottom: int | None = p_regular(54, default=None)


@struct_(StructType.CORNERS)
class Corners(Struct):
    """A corners value (base + top_left/top_right/bottom_left/bottom_right)."""

    base: int | None = p_regular(50, default=None)
    top_left: int | None = p_regular(51, default=None)
    top_right: int | None = p_regular(52, default=None)
    bottom_left: int | None = p_regular(53, default=None)
    bottom_right: int | None = p_regular(54, default=None)


@struct_(StructType.AXIS2)
class Axis2(Struct):
    """A gap value (base + x/y)."""

    base: float | None = p_regular(50, default=None)
    x: float | None = p_regular(51, default=None)
    y: float | None = p_regular(52, default=None)


@struct_(StructType.AXIS3)
class Axis3(Struct):
    """A rotation value (base + x/y/z)."""

    base: float | None = p_regular(50, default=None)
    x: float | None = p_regular(51, default=None)
    y: float | None = p_regular(52, default=None)
    z: float | None = p_regular(53, default=None)


@struct_(StructType.VECTOR2)
class Vector2(Struct):
    """A 2D vector."""

    x: float = p_regular(50)
    y: float = p_regular(51)


def vector2(x: float, y: float) -> "Vector2":
    return Vector2(x=float(x), y=float(y))


@struct_(StructType.VECTOR3)
class Vector3(Struct):
    """A 3D vector."""

    x: float = p_regular(50)
    y: float = p_regular(51)
    z: float = p_regular(52)


def vector3(x: float, y: float, z: float) -> "Vector3":
    return Vector3(x=float(x), y=float(y), z=float(z))


@struct_(StructType.VECTOR4)
class Vector4(Struct):
    """A 4D vector."""

    x: float = p_regular(50)
    y: float = p_regular(51)
    z: float = p_regular(52)
    w: float = p_regular(53)


def vector4(x: float, y: float, z: float, w: float) -> "Vector4":
    return Vector4(x=float(x), y=float(y), z=float(z), w=float(w))


@struct_(StructType.GRID)
class Grid(Struct):
    """A grid configuration value."""

    columns: int = p_regular(50)
    rows: int = p_regular(51)
    column_width: Dimension | None = p_regular(
        52, require=False, array=False, default=None, struct=StructType.DIMENSION
    )
    column_min_width: Dimension | None = p_regular(
        53, require=False, array=False, default=None, struct=StructType.DIMENSION
    )
    row_height: Dimension | None = p_regular(
        54, require=False, array=False, default=None, struct=StructType.DIMENSION
    )


@struct_(StructType.GRID_SPAN)
class GridSpan(Struct):
    """A grid span value."""

    columns: int = p_regular(50)
    rows: int = p_regular(51)
