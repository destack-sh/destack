from typing import TYPE_CHECKING

from destack.language.core import (
    Enum,
    EnumType,
    StructFrozen,
    StructType,
    builtin_enum,
    builtin_property,
    builtin_struct,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.LAYOUT)
class Layout(Enum):
    """The layout of a View."""

    STACK = 1, "Stack", "Stack", "fas fa-objects-align-center-vertical"
    GRID = 2, "Grid", "Grid", "fas fa-grid-2"


@builtin_enum(EnumType.OVERFLOW)
class Overflow(Enum):
    """The overflow behavior of a View."""

    HIDDEN = 2, "Hidden", "Hidden", "fas fa-eye-slash"
    VISIBLE = 3, "Visible", "Visible", "fas fa-eye"
    SCROLL = 4, "Scroll", "Scroll", "fas fa-machine-mouse-scrollwheel"


@builtin_enum(EnumType.DIRECTION)
class Direction(Enum):
    """The direction of a View."""

    HORIZONTAL = 1, "Horizontal", "Horizontal", "fas fa-left-right"
    VERTICAL = 2, "Vertical", "Vertical", "fas fa-up-down"


@builtin_enum(EnumType.DISTRIBUTE)
class Distribute(Enum):
    """The distribution of a View's children."""

    START = 1, "Start", "Start", "fas fa-align-left"
    CENTER = 2, "Center", "Center", "fas fa-align-center"
    END = 3, "End", "End", "fas fa-align-right"
    SPACE_BETWEEN = 4, "Viewport Between", "Viewport Between"
    SPACE_AROUND = 5, "Viewport Around", "Viewport Around"
    SPACE_EVENLY = 6, "Viewport Evenly", "Viewport Evenly"


@builtin_enum(EnumType.ALIGN)
class Align(Enum):
    """The alignment of a View."""

    START = 1, "Start", "Start", "fas fa-align-left"
    CENTER = 2, "Center", "Center", "fas fa-align-center"
    END = 3, "End", "End", "fas fa-align-right"


@builtin_enum(EnumType.LENGTH_UNIT)
class LengthUnit(Enum):
    """The unit of a length value."""

    PIXEL = 1, "Pixel", "px"
    REM = 2, "Rem", "rem"
    PERCENT = 3, "Percent", "%"
    FR = 4, "Fr", "fr"


@builtin_struct(StructType.LENGTH, frozen=True)
class Length(StructFrozen):
    """A length value."""

    unit: LengthUnit = builtin_property(101, is_repr=True)
    value: float = builtin_property(102, is_repr=True)


@builtin_enum(EnumType.POSITION_TYPE)
class PositionType(Enum):
    """The position type of a View."""

    RELATIVE = 1, "Relative", "Relative"
    ABSOLUTE = 2, "Absolute", "Absolute"
    FIXED = 3, "Fixed", "Fixed"
    STICKY = 4, "Sticky", "Sticky"


@builtin_struct(StructType.POSITION, frozen=True)
class Position(StructFrozen):
    """A position value."""

    type: PositionType = builtin_property(100, is_repr=True)
    top: Length | None = builtin_property(101, is_repr=True)
    left: Length | None = builtin_property(102, is_repr=True)
    width: Length | None = builtin_property(103, is_repr=True)
    height: Length | None = builtin_property(104, is_repr=True)


@builtin_enum(EnumType.DIMENSION_TYPE)
class DimensionType(Enum):
    FIXED = 2, "Fixed", "Fixed", "fas fa-ruler-horizontal"
    FIT = 3, "Fit", "Fit", "fas fa-arrows-up-to-line"
    FILL = 4, "Fill", "Fill", "fas fa-arrows-from-dotted-line"


@builtin_struct(StructType.DIMENSION, frozen=True)
class Dimension(StructFrozen):
    """A dimension value (like Length but can fit or fill container)."""

    type: DimensionType = builtin_property(100, is_repr=True)
    unit: LengthUnit = builtin_property(101, is_repr=True)
    value: float = builtin_property(102, is_repr=True)


@builtin_struct(StructType.INSETS, frozen=True)
class Insets(StructFrozen):
    """An insets value (base + side overrides)."""

    base: int | None = builtin_property(101, is_repr=True)
    top: int | None = builtin_property(102, is_repr=True)
    left: int | None = builtin_property(103, is_repr=True)
    right: int | None = builtin_property(104, is_repr=True)
    bottom: int | None = builtin_property(105, is_repr=True)


@builtin_struct(StructType.CORNERS, frozen=True)
class Corners(StructFrozen):
    """A corners value (base + corner overrides)."""

    base: int | None = builtin_property(101, is_repr=True)
    top_left: int | None = builtin_property(102, is_repr=True)
    top_right: int | None = builtin_property(103, is_repr=True)
    bottom_left: int | None = builtin_property(104, is_repr=True)
    bottom_right: int | None = builtin_property(105, is_repr=True)


@builtin_struct(StructType.AXIS2, frozen=True)
class Axis2(StructFrozen):
    """A gap value (base + x/y overrides)."""

    base: float | None = builtin_property(101, is_repr=True)
    x: float | None = builtin_property(102, is_repr=True)
    y: float | None = builtin_property(103, is_repr=True)


@builtin_struct(StructType.AXIS3, frozen=True)
class Axis3(StructFrozen):
    """A rotation value (base + x/y/z overrides)."""

    base: float | None = builtin_property(101, is_repr=True)
    x: float | None = builtin_property(102, is_repr=True)
    y: float | None = builtin_property(103, is_repr=True)
    z: float | None = builtin_property(104, is_repr=True)


@builtin_struct(StructType.GRID, frozen=True)
class Grid(StructFrozen):
    """A grid configuration value."""

    columns: int = builtin_property(101, is_repr=True)
    rows: int = builtin_property(102, is_repr=True)
    column_width: Dimension | None = builtin_property(103, is_repr=True)
    column_min_width: Dimension | None = builtin_property(104, is_repr=True)
    row_height: Dimension | None = builtin_property(105, is_repr=True)


@builtin_struct(StructType.GRID_SPAN, frozen=True)
class GridSpan(StructFrozen):
    """A grid span value."""

    columns: int = builtin_property(101, is_repr=True)
    rows: int = builtin_property(102, is_repr=True)
