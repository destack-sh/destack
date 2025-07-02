from typing import TYPE_CHECKING, Optional

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

from .easing import Easing
from .style import Style

if TYPE_CHECKING:
    from destack.language import Color

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.STROKE_TYPE)
class StrokeType(Enum):
    SOLID = 1
    DASHED = 2
    DOTTED = 3
    FREEHAND = 4


@builtin_struct(StructType.STROKE, frozen=True)
class Stroke(StructFrozen):
    """A Stroke."""

    type: StrokeType = builtin_property(100)
    size: int = builtin_property(101, description="The stroke size/width.")
    thinning: float = builtin_property(
        102, description="The amount of pressure-based thinning (0-1)."
    )
    smoothing: float = builtin_property(103, description="The amount of path smoothing (0-1).")
    streamline: float = builtin_property(
        104, description="The amount of streamlining applied to path (0-1)."
    )
    simulate_pressure: bool = builtin_property(
        105, description="Whether to simulate pressure if not provided."
    )
    easing: Easing = builtin_property(106, description="The easing function for pressure mapping.")
    start: Optional["StrokeCap"] = builtin_property(107, description="The start cap configuration.")
    end: Optional["StrokeCap"] = builtin_property(108, description="The end cap configuration.")
    color: Optional["Color"] = builtin_property(109, description="The stroke color.")


@builtin_struct(StructType.STROKE_CAP, frozen=True)
class StrokeCap(StructFrozen):
    """A stroke cap."""

    cap: bool = builtin_property(101, description="Whether to cap the stroke.")
    taper: bool = builtin_property(102, description="Whether to taper the stroke.")
    easing: Easing = builtin_property(103, description="The easing function for taper.")


@builtin_node(NodeType.STROKE_STYLE)
class StrokeStyle(Style):
    """A StrokeStyle."""

    type: StrokeType = builtin_property(100)
    size: int = builtin_property(200, description="The stroke size/width.")
    thinning: float = builtin_property(
        201, description="The amount of pressure-based thinning (0-1)."
    )
    smoothing: float = builtin_property(202, description="The amount of path smoothing (0-1).")
    streamline: float = builtin_property(
        203, description="The amount of streamlining applied to path (0-1)."
    )
    easing: Easing = builtin_property(204, description="The easing function for pressure mapping.")
    start: Optional["StrokeCap"] = builtin_property(205, description="The start cap configuration.")
    end: Optional["StrokeCap"] = builtin_property(206, description="The end cap configuration.")


@builtin_struct(StructType.STROKE_POINT, frozen=True)
class StrokePoint(StructFrozen):
    """A computed point in a stroke."""

    point: Vector2 = builtin_property(101, is_repr=True, description="The adjusted point position.")
    original_point: Vector2 = builtin_property(
        102, is_repr=True, description="The original input point."
    )
    pressure: float = builtin_property(103, description="The pressure value at this point (0-1).")
    direction: Vector2 = builtin_property(
        104, description="The normalized direction vector from previous point."
    )
    distance: float = builtin_property(105, description="Distance from the previous point.")
    running_length: float = builtin_property(106, description="Total distance from stroke start.")
    radius: float = builtin_property(107, description="The computed radius at this point.")


@builtin_struct(StructType.STROKE_PATH, frozen=True)
class StrokePath(StructFrozen):
    """A stroke path."""

    points: list[StrokePoint] = builtin_property(101, is_repr=True)
