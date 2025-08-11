from typing import TYPE_CHECKING, final

from destack.core import (
    EnumType,
    Int16,
    OptionEnum,
    Struct,
    StructType,
    declare_enum,
    declare_option,
    declare_property,
    declare_struct,
)

if TYPE_CHECKING:
    pass


@declare_enum(EnumType.LAYOUT)
class Layout(OptionEnum):
    """The layout of elements."""

    STACK = declare_option(1, description="Stack")
    GRID = declare_option(2, description="Grid")


@declare_enum(EnumType.OVERFLOW)
class Overflow(OptionEnum):
    """The overflow behavior of elements."""

    HIDDEN = declare_option(2, description="Hidden")
    VISIBLE = declare_option(3, description="Visible")
    SCROLL = declare_option(4, description="Scroll")


@declare_enum(EnumType.DIRECTION)
class Direction(OptionEnum):
    """The direction of elements."""

    HORIZONTAL = declare_option(1, description="Horizontal")
    VERTICAL = declare_option(2, description="Vertical")


@declare_enum(EnumType.DISTRIBUTE)
class Distribute(OptionEnum):
    """The distribution of elements."""

    START = declare_option(1, description="Start")
    CENTER = declare_option(2, description="Center")
    END = declare_option(3, description="End")
    SPACE_BETWEEN = declare_option(4, description="Viewport Between")
    SPACE_AROUND = declare_option(5, description="Viewport Around")
    SPACE_EVENLY = declare_option(6, description="Viewport Evenly")


@declare_enum(EnumType.ALIGN)
class Align(OptionEnum):
    """The alignment of elements."""

    START = declare_option(1, description="Start")
    CENTER = declare_option(2, description="Center")
    END = declare_option(3, description="End")


@declare_enum(EnumType.ANCHOR)
class Anchor(OptionEnum):
    """The position of elements."""

    RELATIVE = declare_option(1, description="Relative to parent")
    ABSOLUTE = declare_option(2, description="Absolute in parent")
    FIXED = declare_option(3, description="Fixed to root")
    STICKY = declare_option(4, description="Sticky to parent")


@declare_enum(EnumType.LENGTH_TYPE)
class LengthType(OptionEnum):
    """The unit of a length value."""

    PIXEL = declare_option(1, description="Pixel")
    PERCENT = declare_option(2, description="Percent")
    FIT = declare_option(3, description="Fit")
    FILL = declare_option(4, description="Fill")


@declare_struct(
    StructType.LENGTH,
    is_final=True,
)
@final
class Length(Struct):
    """An absolute or relative length value."""

    unit: LengthType = declare_property(101, is_repr=True)
    value: Int16 = declare_property(102, is_repr=True)


@declare_struct(
    StructType.OFFSET2,
    is_final=True,
)
@final
class Offset2(Struct):
    """A 2-dimensional position value (relative or absolute)."""

    type: Anchor = declare_property(100, is_repr=True)
    top: Length | None = declare_property(101, is_repr=True)
    left: Length | None = declare_property(102, is_repr=True)
    width: Length | None = declare_property(103, is_repr=True)
    height: Length | None = declare_property(104, is_repr=True)


@declare_struct(
    StructType.INSET2,
    is_final=True,
)
@final
class Inset2(Struct):
    """A 2-dimensional insets value (base + side overrides)."""

    base: Int16 = declare_property(101, is_repr=True, default=0)
    top: Int16 | None = declare_property(102, is_repr=True)
    left: Int16 | None = declare_property(103, is_repr=True)
    right: Int16 | None = declare_property(104, is_repr=True)
    bottom: Int16 | None = declare_property(105, is_repr=True)


@declare_struct(
    StructType.CORNER2,
    is_final=True,
)
@final
class Corner2(Struct):
    """A 2-dimensional corners value (base + corner overrides)."""

    base: Int16 = declare_property(101, is_repr=True, default=0)
    top_left: Int16 | None = declare_property(102, is_repr=True)
    top_right: Int16 | None = declare_property(103, is_repr=True)
    bottom_left: Int16 | None = declare_property(104, is_repr=True)
    bottom_right: Int16 | None = declare_property(105, is_repr=True)


@declare_struct(
    StructType.AXIS2,
    is_final=True,
)
@final
class Axis2(Struct):
    """A 2-dimensional axis value (base + x/y overrides)."""

    base: Int16 = declare_property(101, is_repr=True, default=0)
    x: Int16 | None = declare_property(102, is_repr=True)
    y: Int16 | None = declare_property(103, is_repr=True)


@declare_struct(
    StructType.AXIS3,
    is_final=True,
)
@final
class Axis3(Struct):
    """A 3-dimensional axis value (base + x/y/z overrides)."""

    base: Int16 = declare_property(101, is_repr=True, default=0)
    x: Int16 | None = declare_property(102, is_repr=True)
    y: Int16 | None = declare_property(103, is_repr=True)
    z: Int16 | None = declare_property(104, is_repr=True)


@declare_struct(
    StructType.GRID2,
    is_final=True,
)
@final
class Grid2(Struct):
    """A 2-dimensional grid configuration value."""

    columns: Int16 = declare_property(101, is_repr=True)
    rows: Int16 = declare_property(102, is_repr=True)
    column_width: Length | None = declare_property(103, is_repr=True)
    column_min_width: Length | None = declare_property(104, is_repr=True)
    row_height: Length | None = declare_property(105, is_repr=True)


@declare_struct(
    StructType.GRID_SPAN2,
    is_final=True,
)
@final
class GridSpan2(Struct):
    """A 2-dimensional grid span value."""

    columns: Int16 = declare_property(101, is_repr=True)
    rows: Int16 = declare_property(102, is_repr=True)
