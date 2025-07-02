from datetime import timedelta
from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Axis3,
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

from .style import Style
from .transition import Transition

if TYPE_CHECKING:
    pass


# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.EFFECT_TYPE)
class EffectType(Enum):
    """When the effect fires."""

    APPEAR = 10, "Appear", "Initial render in"
    ENTER = 11, "Enter", "Enters viewport"
    EXIT = 12, "Exit", "Leaves viewport"
    HOVER = 20, "Hover", "While hover"
    PRESS = 21, "Press", "While tap"
    DRAG = 22, "Drag", "While drag / drag"
    FOCUS = 23, "Focus", "While focus"
    LOOP = 30, "Loop", "Continuous loop"
    # SCROLL, ...


@builtin_enum(EnumType.REPEAT_TYPE)
class RepeatType(Enum):
    LOOP = 1, "Loop", "Restart from beginning"
    REVERSE = 2, "Reverse", "Yoyo back and forth"
    MIRROR = 3, "Mirror", "Mirror keyframes"


@builtin_enum(EnumType.TEXT_SPLIT_TYPE)
class TextSplitType(Enum):
    CHAR = 1, "Char", "Split by character"
    WORD = 2, "Word", "Split by word"
    LINE = 3, "Line", "Split by line"


@builtin_enum(EnumType.OFFSCREEN_BEHAVIOR)
class OffscreenBehavior(Enum):
    """What happens when the element is offscreen."""

    PLAY = 1, "Play", "Play the animation"
    PAUSE = 2, "Pause", "Pause the animation"


@builtin_struct(StructType.EFFECT, frozen=True)
class Effect(StructFrozen):
    """An effect value."""

    type: EffectType = builtin_property(30, is_repr=True)
    style: Optional["EffectStyle"] = builtin_property(41, is_repr=True)
    opacity: Optional[float] = builtin_property(50, is_repr=True)
    offset: Optional[Vector2] = builtin_property(51, is_repr=True)
    scale: Optional[float] = builtin_property(52, is_repr=True)
    rotate: Optional[Axis3] = builtin_property(53, is_repr=True)
    skew: Optional[Vector2] = builtin_property(54, is_repr=True)
    perspective: Optional[float] = builtin_property(55, is_repr=True)
    delay: Optional[timedelta] = builtin_property(56, is_repr=True)
    duration: Optional[float] = builtin_property(57, is_repr=True)
    threshold: Optional[float] = builtin_property(58, is_repr=True)
    once: Optional[bool] = builtin_property(59, is_repr=True)
    repeat: Optional[RepeatType] = builtin_property(60, is_repr=True)
    split: Optional[TextSplitType] = builtin_property(61, is_repr=True)
    offscreen: Optional[OffscreenBehavior] = builtin_property(62, is_repr=True)
    transition: Optional["Transition"] = builtin_property(70, is_repr=True)


@builtin_node(NodeType.EFFECT_STYLE)
class EffectStyle(Style):
    """An effect style."""

    type: EffectType = builtin_property(30, is_repr=True)
    opacity: Optional[float] = builtin_property(50, is_repr=True)
    offset: Optional[Vector2] = builtin_property(51, is_repr=True)
    scale: Optional[float] = builtin_property(52, is_repr=True)
    rotate: Optional[Axis3] = builtin_property(53, is_repr=True)
    skew: Optional[Vector2] = builtin_property(54, is_repr=True)
    perspective: Optional[float] = builtin_property(55, is_repr=True)
    delay: Optional[timedelta] = builtin_property(56, is_repr=True)
    duration: Optional[float] = builtin_property(57, is_repr=True)
    threshold: Optional[float] = builtin_property(58, is_repr=True)
    once: Optional[bool] = builtin_property(59, is_repr=True)
    repeat: Optional[RepeatType] = builtin_property(60, is_repr=True)
    split: Optional[TextSplitType] = builtin_property(61, is_repr=True)
    offscreen: Optional[OffscreenBehavior] = builtin_property(62, is_repr=True)
    transition: Optional["Transition"] = builtin_property(70, is_repr=True)
