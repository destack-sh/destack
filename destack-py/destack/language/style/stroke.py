from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Enum,
    EnumType,
    Node,
    NodeType,
    StructFrozen,
    StructType,
    Vector2,
    Vector3,
    builtin_enum,
    builtin_node,
    builtin_struct,
    property_,
)
from destack.proto import StrokeStyleProto

from .easing import Easing
from .style import Style

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.STROKE_TYPE)
class StrokeType(Enum):
    SOLID = 1
    DASHED = 2
    DOTTED = 3


@builtin_struct(StructType.STROKE, frozen=True)
class Stroke(StructFrozen):
    """A Stroke."""

    type: StrokeType = property_(30)
    size: int = property_(50, description="The stroke size/width.")
    thinning: float = property_(51, description="The amount of pressure-based thinning (0-1).")
    smoothing: float = property_(52, description="The amount of path smoothing (0-1).")
    streamline: float = property_(
        53, description="The amount of streamlining applied to path (0-1)."
    )
    simulate_pressure: bool = property_(
        54, description="Whether to simulate pressure if not provided."
    )
    easing: Easing = property_(55, description="The easing function for pressure mapping.")
    start: Optional["StrokeCap"] = property_(60, description="The start cap configuration.")
    end: Optional["StrokeCap"] = property_(61, description="The end cap configuration.")


@builtin_struct(StructType.STROKE_CAP, frozen=True)
class StrokeCap(StructFrozen):
    """A stroke cap."""

    cap: bool = property_(50, description="Whether to cap the stroke.")
    taper: float | None = property_(51, description="The taper amount (0-1).")
    easing: Easing = property_(52, description="The easing function for taper.")


@builtin_node(NodeType.STROKE_STYLE)
class StrokeStyle(Style, Node[StrokeStyleProto]):
    """A StrokeStyle."""

    type: StrokeType = property_(30)
    size: int = property_(50, description="The stroke size/width.")
    thinning: float = property_(51, description="The amount of pressure-based thinning (0-1).")
    smoothing: float = property_(52, description="The amount of path smoothing (0-1).")
    streamline: float = property_(
        53, description="The amount of streamlining applied to path (0-1)."
    )
    simulate_pressure: bool = property_(
        54, description="Whether to simulate pressure if not provided."
    )
    easing: Easing = property_(55, description="The easing function for pressure mapping.")
    start: Optional["StrokeCap"] = property_(60, description="The start cap configuration.")
    end: Optional["StrokeCap"] = property_(61, description="The end cap configuration.")


@builtin_struct(StructType.STROKE_POINT, frozen=True)
class StrokePoint(StructFrozen):
    """A computed point in a stroke."""

    point: Vector2 = property_(50, is_repr=True, description="The adjusted point position.")
    original_point: Vector3 = property_(51, is_repr=True, description="The original input point.")
    pressure: float = property_(52, description="The pressure value at this point (0-1).")
    direction: Vector3 = property_(
        53, description="The normalized direction vector from previous point."
    )
    distance: float = property_(54, description="Distance from the previous point.")
    running_length: float = property_(55, description="Total distance from stroke start.")
    radius: float = property_(56, description="The computed radius at this point.")


@builtin_struct(StructType.STROKE_PATH, frozen=True)
class StrokePath(StructFrozen):
    """A stroke path."""

    points: list[StrokePoint] = property_(100, is_repr=True)
