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
    builtin_struct,
    property_,
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

    type: EffectType = property_(30, is_repr=True)
    style: Optional["EffectStyle"] = property_(41, is_repr=True)
    opacity: Optional[float] = property_(50, is_repr=True)
    offset: Optional[Vector2] = property_(51, is_repr=True)
    scale: Optional[float] = property_(52, is_repr=True)
    rotate: Optional[Axis3] = property_(53, is_repr=True)
    skew: Optional[Vector2] = property_(54, is_repr=True)
    perspective: Optional[float] = property_(55, is_repr=True)
    delay: Optional[timedelta] = property_(56, is_repr=True)
    duration: Optional[float] = property_(57, is_repr=True)
    threshold: Optional[float] = property_(58, is_repr=True)
    once: Optional[bool] = property_(59, is_repr=True)
    repeat: Optional[RepeatType] = property_(60, is_repr=True)
    split: Optional[TextSplitType] = property_(61, is_repr=True)
    offscreen: Optional[OffscreenBehavior] = property_(62, is_repr=True)
    transition: Optional["Transition"] = property_(70, is_repr=True)


@builtin_node(NodeType.EFFECT_STYLE)
class EffectStyle(Style):
    """An effect style."""

    type: EffectType = property_(30, is_repr=True)
    opacity: Optional[float] = property_(50, is_repr=True)
    offset: Optional[Vector2] = property_(51, is_repr=True)
    scale: Optional[float] = property_(52, is_repr=True)
    rotate: Optional[Axis3] = property_(53, is_repr=True)
    skew: Optional[Vector2] = property_(54, is_repr=True)
    perspective: Optional[float] = property_(55, is_repr=True)
    delay: Optional[timedelta] = property_(56, is_repr=True)
    duration: Optional[float] = property_(57, is_repr=True)
    threshold: Optional[float] = property_(58, is_repr=True)
    once: Optional[bool] = property_(59, is_repr=True)
    repeat: Optional[RepeatType] = property_(60, is_repr=True)
    split: Optional[TextSplitType] = property_(61, is_repr=True)
    offscreen: Optional[OffscreenBehavior] = property_(62, is_repr=True)
    transition: Optional["Transition"] = property_(70, is_repr=True)
