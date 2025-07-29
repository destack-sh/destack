from typing import TYPE_CHECKING, final

from destack.language.core import (
    Enum,
    EnumType,
    Float32,
    StructFrozen,
    StructType,
    UInt16,
    builtin_enum,
    builtin_property,
    builtin_struct,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.LAYOUT)
class Layout(Enum):
    """The layout of elements."""

    STACK = 1, "Stack", "Stack", "fas fa-objects-align-center-vertical"
    GRID = 2, "Grid", "Grid", "fas fa-grid-2"


@builtin_enum(EnumType.OVERFLOW)
class Overflow(Enum):
    """The overflow behavior of elements."""

    HIDDEN = 2, "Hidden", "Hidden", "fas fa-eye-slash"
    VISIBLE = 3, "Visible", "Visible", "fas fa-eye"
    SCROLL = 4, "Scroll", "Scroll", "fas fa-machine-mouse-scrollwheel"


@builtin_enum(EnumType.DIRECTION)
class Direction(Enum):
    """The direction of elements."""

    HORIZONTAL = 1, "Horizontal", "Horizontal", "fas fa-left-right"
    VERTICAL = 2, "Vertical", "Vertical", "fas fa-up-down"


@builtin_enum(EnumType.DISTRIBUTE)
class Distribute(Enum):
    """The distribution of elements."""

    START = 1, "Start", "Start", "fas fa-align-left"
    CENTER = 2, "Center", "Center", "fas fa-align-center"
    END = 3, "End", "End", "fas fa-align-right"
    SPACE_BETWEEN = 4, "Viewport Between", "Viewport Between"
    SPACE_AROUND = 5, "Viewport Around", "Viewport Around"
    SPACE_EVENLY = 6, "Viewport Evenly", "Viewport Evenly"


@builtin_enum(EnumType.ALIGN)
class Align(Enum):
    """The alignment of elements."""

    START = 1, "Start", "Start", "fas fa-align-left"
    CENTER = 2, "Center", "Center", "fas fa-align-center"
    END = 3, "End", "End", "fas fa-align-right"


@builtin_enum(EnumType.ANCHOR)
class Anchor(Enum):
    """The position of elements."""

    RELATIVE = 1, "Relative", "Relative to parent"
    ABSOLUTE = 2, "Absolute", "Absolute in parent"
    FIXED = 3, "Fixed", "Fixed to root"
    STICKY = 4, "Sticky", "Sticky to parent"


@builtin_enum(EnumType.LENGTH_TYPE)
class LengthType(Enum):
    """The unit of a length value."""

    PIXEL = 1, "Pixel", "px"
    REM = 2, "Rem", "rem"
    PERCENT = 3, "Percent", "%"
    FR = 4, "Fr", "fr"
    FIT = 10, "Fit", "fit"
    FILL = 11, "Fill", "fill"


@builtin_struct(
    StructType.LENGTH,
    frozen=True,
    is_final=True,
)
@final
class Length(StructFrozen):
    """An absolute or relative length value."""

    unit: LengthType = builtin_property(101, is_repr=True)
    value: Float32 = builtin_property(102, is_repr=True)


@builtin_struct(
    StructType.OFFSET2,
    frozen=True,
    is_final=True,
)
@final
class Offset2(StructFrozen):
    """A 2-dimensional position value (relative or absolute)."""

    type: Anchor = builtin_property(100, is_repr=True)
    top: Length | None = builtin_property(101, is_repr=True)
    left: Length | None = builtin_property(102, is_repr=True)
    width: Length | None = builtin_property(103, is_repr=True)
    height: Length | None = builtin_property(104, is_repr=True)


@builtin_struct(
    StructType.INSET2,
    frozen=True,
    is_final=True,
)
@final
class Inset2(StructFrozen):
    """A 2-dimensional insets value (base + side overrides)."""

    base: UInt16 = builtin_property(101, is_repr=True, default=0)
    top: UInt16 | None = builtin_property(102, is_repr=True)
    left: UInt16 | None = builtin_property(103, is_repr=True)
    right: UInt16 | None = builtin_property(104, is_repr=True)
    bottom: UInt16 | None = builtin_property(105, is_repr=True)


@builtin_struct(
    StructType.CORNER2,
    frozen=True,
    is_final=True,
)
@final
class Corner2(StructFrozen):
    """A 2-dimensional corners value (base + corner overrides)."""

    base: UInt16 = builtin_property(101, is_repr=True, default=0)
    top_left: UInt16 | None = builtin_property(102, is_repr=True)
    top_right: UInt16 | None = builtin_property(103, is_repr=True)
    bottom_left: UInt16 | None = builtin_property(104, is_repr=True)
    bottom_right: UInt16 | None = builtin_property(105, is_repr=True)


@builtin_struct(
    StructType.AXIS2,
    frozen=True,
    is_final=True,
)
@final
class Axis2(StructFrozen):
    """A 2-dimensional axis value (base + x/y overrides)."""

    base: Float32 = builtin_property(101, is_repr=True, default=0)
    x: Float32 | None = builtin_property(102, is_repr=True)
    y: Float32 | None = builtin_property(103, is_repr=True)


@builtin_struct(
    StructType.AXIS3,
    frozen=True,
    is_final=True,
)
@final
class Axis3(StructFrozen):
    """A 3-dimensional axis value (base + x/y/z overrides)."""

    base: Float32 = builtin_property(101, is_repr=True, default=0)
    x: Float32 | None = builtin_property(102, is_repr=True)
    y: Float32 | None = builtin_property(103, is_repr=True)
    z: Float32 | None = builtin_property(104, is_repr=True)


@builtin_struct(
    StructType.GRID2,
    frozen=True,
    is_final=True,
)
@final
class Grid2(StructFrozen):
    """A 2-dimensional grid configuration value."""

    columns: UInt16 = builtin_property(101, is_repr=True)
    rows: UInt16 = builtin_property(102, is_repr=True)
    column_width: Length | None = builtin_property(103, is_repr=True)
    column_min_width: Length | None = builtin_property(104, is_repr=True)
    row_height: Length | None = builtin_property(105, is_repr=True)


@builtin_struct(
    StructType.GRID_SPAN2,
    frozen=True,
    is_final=True,
)
@final
class GridSpan2(StructFrozen):
    """A 2-dimensional grid span value."""

    columns: UInt16 = builtin_property(101, is_repr=True)
    rows: UInt16 = builtin_property(102, is_repr=True)
