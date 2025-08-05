from typing import TYPE_CHECKING, Optional, final

from destack.core import (
    EnumType,
    Float32,
    NodeType,
    OptionEnum,
    StructFrozen,
    StructType,
    UInt8,
    declare_entity,
    declare_enum,
    declare_option,
    declare_property,
    declare_struct,
)

from .style import Style

if TYPE_CHECKING:
    from destack import Color, Easing, Vector2


@declare_enum(EnumType.STROKE_TYPE)
class StrokeType(OptionEnum):
    SOLID = declare_option(1, "Solid", description="A solid stroke")
    DASHED = declare_option(2, "Dashed", description="A dashed stroke")
    DOTTED = declare_option(3, "Dotted", description="A dotted stroke")
    FREEHAND = declare_option(4, "Freehand", description="A freehand stroke")


@declare_struct(
    StructType.STROKE,
    frozen=True,
    is_final=True,
    into_node_types=(NodeType.STROKE_STYLE,),
)
@final
class Stroke(StructFrozen):
    """A Stroke."""

    type: StrokeType = declare_property(100)
    size: UInt8 = declare_property(101, description="The stroke size/width.")
    thinning: Float32 = declare_property(
        102, description="The amount of pressure-based thinning (0-1)."
    )
    smoothing: Float32 = declare_property(103, description="The amount of path smoothing (0-1).")
    streamline: Float32 = declare_property(
        104, description="The amount of streamlining applied to path (0-1)."
    )
    easing: "Easing" = declare_property(
        105, description="The easing function for pressure mapping."
    )
    color: Optional["Color"] = declare_property(106, description="The stroke color.")
    start: Optional["StrokeCap"] = declare_property(110, description="The start cap configuration.")
    end: Optional["StrokeCap"] = declare_property(111, description="The end cap configuration.")


@declare_struct(StructType.STROKE_CAP, frozen=True, is_final=True)
@final
class StrokeCap(StructFrozen):
    """A stroke cap."""

    cap: bool = declare_property(101, description="Whether to cap the stroke.")
    taper: bool = declare_property(102, description="Whether to taper the stroke.")
    easing: "Easing" = declare_property(103, description="The easing function for taper.")


@declare_struct(StructType.STROKE_POINT, frozen=True, is_final=True)
@final
class StrokePoint(StructFrozen):
    """A computed point in a stroke."""

    point: "Vector2" = declare_property(
        101, is_repr=True, description="The adjusted point position."
    )
    original_point: "Vector2" = declare_property(
        102, is_repr=True, description="The original input point."
    )
    pressure: Float32 = declare_property(103, description="The pressure value at this point (0-1).")
    direction: "Vector2" = declare_property(
        104, description="The normalized direction vector from previous point."
    )
    distance: Float32 = declare_property(105, description="Distance from the previous point.")
    running_length: Float32 = declare_property(106, description="Total distance from stroke start.")
    radius: Float32 = declare_property(107, description="The computed radius at this point.")


@declare_struct(StructType.STROKE_PATH, frozen=True, is_final=True)
@final
class StrokePath(StructFrozen):
    """A stroke path."""

    points: list[StrokePoint] = declare_property(101, is_repr=True)


@declare_entity(
    NodeType.STROKE_STYLE,
    base_struct_type=StructType.STROKE,
)
@final
class StrokeStyle(Style):
    """A StrokeStyle."""

    type: StrokeType = declare_property(100)
    size: UInt8 = declare_property(101, description="The stroke size/width.")
    thinning: Float32 = declare_property(
        102, description="The amount of pressure-based thinning (0-1)."
    )
    smoothing: Float32 = declare_property(103, description="The amount of path smoothing (0-1).")
    streamline: Float32 = declare_property(
        104, description="The amount of streamlining applied to path (0-1)."
    )
    easing: "Easing" = declare_property(
        105, description="The easing function for pressure mapping."
    )
    color: Optional["Color"] = declare_property(106, description="The stroke color.")
    start: Optional["StrokeCap"] = declare_property(110, description="The start cap configuration.")
    end: Optional["StrokeCap"] = declare_property(111, description="The end cap configuration.")
