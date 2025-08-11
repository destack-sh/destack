from typing import TYPE_CHECKING, Optional, final

from destack.core import (
    EnumType,
    Float32,
    NodeType,
    OptionEnum,
    ReferenceType,
    Struct,
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
    is_final=True,
    into_node_types=(NodeType.STROKE_STYLE,),
)
@final
class Stroke(Struct):
    """A stroke value."""

    type: StrokeType = declare_property(
        100,
        is_repr=True,
        tag=None,
    )
    template: Optional["StrokeStyle"] = declare_property(
        101,
        is_repr=True,
        reference_type=ReferenceType.LOCATION,
        tag=None,
    )
    size: UInt8 = declare_property(
        102,
        description="The stroke size/width.",
        is_repr=True,
        tag=None,
    )
    thinning: Float32 = declare_property(
        103,
        description="The amount of pressure-based thinning (0-1).",
        is_repr=True,
        tag=None,
    )
    smoothing: Float32 = declare_property(
        104,
        description="The amount of path smoothing (0-1).",
        is_repr=True,
        tag=None,
    )
    streamline: Float32 = declare_property(
        105,
        description="The amount of streamlining applied to path (0-1).",
        is_repr=True,
        tag=None,
    )
    easing: "Easing" = declare_property(
        106,
        description="The easing function for pressure mapping.",
        is_repr=True,
        tag=None,
    )
    color: Optional["Color"] = declare_property(
        107,
        description="The stroke color.",
        is_repr=True,
        tag=None,
    )
    start: Optional["StrokeCap"] = declare_property(
        110,
        description="The start cap configuration.",
        is_repr=True,
        tag=None,
    )
    end: Optional["StrokeCap"] = declare_property(
        111,
        description="The end cap configuration.",
        is_repr=True,
        tag=None,
    )


@declare_struct(StructType.STROKE_CAP, is_final=True)
@final
class StrokeCap(Struct):
    """A stroke cap."""

    cap: bool = declare_property(
        101,
        description="Whether to cap the stroke.",
        tag=None,
    )
    taper: bool = declare_property(
        102,
        description="Whether to taper the stroke.",
        tag=None,
    )
    easing: "Easing" = declare_property(
        103,
        description="The easing function for taper.",
        tag=None,
    )


@declare_struct(StructType.STROKE_POINT, is_final=True)
@final
class StrokePoint(Struct):
    """A computed point in a stroke."""

    point: "Vector2" = declare_property(
        101,
        is_repr=True,
        description="The adjusted point position.",
        tag=None,
    )
    original_point: "Vector2" = declare_property(
        102,
        is_repr=True,
        description="The original input point.",
        tag=None,
    )
    pressure: Float32 = declare_property(
        103,
        description="The pressure value at this point (0-1).",
        tag=None,
    )
    direction: "Vector2" = declare_property(
        104,
        description="The normalized direction vector from previous point.",
        tag=None,
    )
    distance: Float32 = declare_property(
        105,
        description="Distance from the previous point.",
        tag=None,
    )
    running_length: Float32 = declare_property(
        106,
        description="Total distance from stroke start.",
        tag=None,
    )
    radius: Float32 = declare_property(
        107,
        description="The computed radius at this point.",
        tag=None,
    )


@declare_struct(StructType.STROKE_PATH, is_final=True)
@final
class StrokePath(Struct):
    """A stroke path."""

    points: list[StrokePoint] = declare_property(
        101,
        is_repr=True,
        tag=None,
    )


@declare_entity(
    NodeType.STROKE_STYLE,
    base_struct_type=StructType.STROKE,
)
class StrokeStyle(Style):
    """A stroke style."""

    type: StrokeType = declare_property(
        100,
        is_repr=True,
        tag=None,
    )
    size: UInt8 = declare_property(
        101,
        description="The stroke size/width.",
        is_repr=True,
        tag=None,
    )
    thinning: Float32 = declare_property(
        102,
        description="The amount of pressure-based thinning (0-1).",
        is_repr=True,
        tag=None,
    )
    smoothing: Float32 = declare_property(
        103,
        description="The amount of path smoothing (0-1).",
        is_repr=True,
        tag=None,
    )
    streamline: Float32 = declare_property(
        104,
        description="The amount of streamlining applied to path (0-1).",
        is_repr=True,
        tag=None,
    )
    easing: "Easing" = declare_property(
        105,
        description="The easing function for pressure mapping.",
        is_repr=True,
        tag=None,
    )
    color: Optional["Color"] = declare_property(
        106,
        description="The stroke color.",
        is_repr=True,
        tag=None,
    )
    start: Optional["StrokeCap"] = declare_property(
        107,
        description="The start cap configuration.",
        is_repr=True,
        tag=None,
    )
    end: Optional["StrokeCap"] = declare_property(
        108,
        description="The end cap configuration.",
        is_repr=True,
        tag=None,
    )
