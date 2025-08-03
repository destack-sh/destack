from typing import TYPE_CHECKING, final

from destack.core import (
    EnumDeclaration,
    EnumType,
    Float32,
    StructFrozen,
    StructType,
    UInt16,
    declare_enum,
    declare_property,
    declare_struct,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@declare_enum(EnumType.LAYOUT)
class Layout(EnumDeclaration):
    """The layout of elements."""

    STACK = 1, "Stack", "Stack"
    GRID = 2, "Grid", "Grid"


@declare_enum(EnumType.OVERFLOW)
class Overflow(EnumDeclaration):
    """The overflow behavior of elements."""

    HIDDEN = 2, "Hidden", "Hidden"
    VISIBLE = 3, "Visible", "Visible"
    SCROLL = 4, "Scroll", "Scroll"


@declare_enum(EnumType.DIRECTION)
class Direction(EnumDeclaration):
    """The direction of elements."""

    HORIZONTAL = 1, "Horizontal", "Horizontal"
    VERTICAL = 2, "Vertical", "Vertical"


@declare_enum(EnumType.DISTRIBUTE)
class Distribute(EnumDeclaration):
    """The distribution of elements."""

    START = 1, "Start", "Start"
    CENTER = 2, "Center", "Center"
    END = 3, "End", "End"
    SPACE_BETWEEN = 4, "Viewport Between", "Viewport Between"
    SPACE_AROUND = 5, "Viewport Around", "Viewport Around"
    SPACE_EVENLY = 6, "Viewport Evenly", "Viewport Evenly"


@declare_enum(EnumType.ALIGN)
class Align(EnumDeclaration):
    """The alignment of elements."""

    START = 1, "Start", "Start"
    CENTER = 2, "Center", "Center"
    END = 3, "End", "End"


@declare_enum(EnumType.ANCHOR)
class Anchor(EnumDeclaration):
    """The position of elements."""

    RELATIVE = 1, "Relative", "Relative to parent"
    ABSOLUTE = 2, "Absolute", "Absolute in parent"
    FIXED = 3, "Fixed", "Fixed to root"
    STICKY = 4, "Sticky", "Sticky to parent"


@declare_enum(EnumType.LENGTH_TYPE)
class LengthType(EnumDeclaration):
    """The unit of a length value."""

    PIXEL = 1, "Pixel", "px"
    REM = 2, "Rem", "rem"
    PERCENT = 3, "Percent", "%"
    FR = 4, "Fr", "fr"
    FIT = 10, "Fit", "fit"
    FILL = 11, "Fill", "fill"


@declare_struct(
    StructType.LENGTH,
    frozen=True,
    is_final=True,
)
@final
class Length(StructFrozen):
    """An absolute or relative length value."""

    unit: LengthType = declare_property(101, is_repr=True)
    value: Float32 = declare_property(102, is_repr=True)


@declare_struct(
    StructType.OFFSET2,
    frozen=True,
    is_final=True,
)
@final
class Offset2(StructFrozen):
    """A 2-dimensional position value (relative or absolute)."""

    type: Anchor = declare_property(100, is_repr=True)
    top: Length | None = declare_property(101, is_repr=True)
    left: Length | None = declare_property(102, is_repr=True)
    width: Length | None = declare_property(103, is_repr=True)
    height: Length | None = declare_property(104, is_repr=True)


@declare_struct(
    StructType.INSET2,
    frozen=True,
    is_final=True,
)
@final
class Inset2(StructFrozen):
    """A 2-dimensional insets value (base + side overrides)."""

    base: UInt16 = declare_property(101, is_repr=True, default=0)
    top: UInt16 | None = declare_property(102, is_repr=True)
    left: UInt16 | None = declare_property(103, is_repr=True)
    right: UInt16 | None = declare_property(104, is_repr=True)
    bottom: UInt16 | None = declare_property(105, is_repr=True)


@declare_struct(
    StructType.CORNER2,
    frozen=True,
    is_final=True,
)
@final
class Corner2(StructFrozen):
    """A 2-dimensional corners value (base + corner overrides)."""

    base: UInt16 = declare_property(101, is_repr=True, default=0)
    top_left: UInt16 | None = declare_property(102, is_repr=True)
    top_right: UInt16 | None = declare_property(103, is_repr=True)
    bottom_left: UInt16 | None = declare_property(104, is_repr=True)
    bottom_right: UInt16 | None = declare_property(105, is_repr=True)


@declare_struct(
    StructType.AXIS2,
    frozen=True,
    is_final=True,
)
@final
class Axis2(StructFrozen):
    """A 2-dimensional axis value (base + x/y overrides)."""

    base: Float32 = declare_property(101, is_repr=True, default=0)
    x: Float32 | None = declare_property(102, is_repr=True)
    y: Float32 | None = declare_property(103, is_repr=True)


@declare_struct(
    StructType.AXIS3,
    frozen=True,
    is_final=True,
)
@final
class Axis3(StructFrozen):
    """A 3-dimensional axis value (base + x/y/z overrides)."""

    base: Float32 = declare_property(101, is_repr=True, default=0)
    x: Float32 | None = declare_property(102, is_repr=True)
    y: Float32 | None = declare_property(103, is_repr=True)
    z: Float32 | None = declare_property(104, is_repr=True)


@declare_struct(
    StructType.GRID2,
    frozen=True,
    is_final=True,
)
@final
class Grid2(StructFrozen):
    """A 2-dimensional grid configuration value."""

    columns: UInt16 = declare_property(101, is_repr=True)
    rows: UInt16 = declare_property(102, is_repr=True)
    column_width: Length | None = declare_property(103, is_repr=True)
    column_min_width: Length | None = declare_property(104, is_repr=True)
    row_height: Length | None = declare_property(105, is_repr=True)


@declare_struct(
    StructType.GRID_SPAN2,
    frozen=True,
    is_final=True,
)
@final
class GridSpan2(StructFrozen):
    """A 2-dimensional grid span value."""

    columns: UInt16 = declare_property(101, is_repr=True)
    rows: UInt16 = declare_property(102, is_repr=True)
