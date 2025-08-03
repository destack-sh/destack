from datetime import timedelta
from typing import TYPE_CHECKING, Optional

from destack.core import (
    Enum,
    EnumType,
    Float32,
    NodeType,
    StructFrozen,
    StructType,
    builtin_entity,
    builtin_enum,
    builtin_property,
    builtin_struct,
)

from ..style.style import Style
from .transition import Transition

if TYPE_CHECKING:
    from destack import Axis3, Vector2


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

    type: EffectType = builtin_property(100, is_repr=True)
    style: Optional["EffectStyle"] = builtin_property(101, is_repr=True)
    opacity: Optional[Float32] = builtin_property(102, is_repr=True)
    offset: Optional["Vector2"] = builtin_property(103, is_repr=True)
    scale: Optional[Float32] = builtin_property(104, is_repr=True)
    rotate: Optional["Axis3"] = builtin_property(105, is_repr=True)
    skew: Optional["Vector2"] = builtin_property(106, is_repr=True)
    perspective: Optional[Float32] = builtin_property(107, is_repr=True)
    delay: Optional[timedelta] = builtin_property(108, is_repr=True)
    duration: Optional[Float32] = builtin_property(109, is_repr=True)
    threshold: Optional[Float32] = builtin_property(110, is_repr=True)
    once: Optional[bool] = builtin_property(111, is_repr=True)
    repeat: Optional[RepeatType] = builtin_property(112, is_repr=True)
    split: Optional[TextSplitType] = builtin_property(113, is_repr=True)
    offscreen: Optional[OffscreenBehavior] = builtin_property(114, is_repr=True)
    transition: Optional["Transition"] = builtin_property(115, is_repr=True)


@builtin_entity(NodeType.EFFECT_STYLE)
class EffectStyle(Style):
    """An effect style."""

    type: EffectType = builtin_property(100, is_repr=True)
    opacity: Optional[Float32] = builtin_property(200, is_repr=True)
    offset: Optional["Vector2"] = builtin_property(201, is_repr=True)
    scale: Optional[Float32] = builtin_property(202, is_repr=True)
    rotate: Optional["Axis3"] = builtin_property(203, is_repr=True)
    skew: Optional["Vector2"] = builtin_property(204, is_repr=True)
    perspective: Optional[Float32] = builtin_property(205, is_repr=True)
    delay: Optional[timedelta] = builtin_property(206, is_repr=True)
    duration: Optional[Float32] = builtin_property(207, is_repr=True)
    threshold: Optional[Float32] = builtin_property(208, is_repr=True)
    once: Optional[bool] = builtin_property(209, is_repr=True)
    repeat: Optional[RepeatType] = builtin_property(210, is_repr=True)
    split: Optional[TextSplitType] = builtin_property(211, is_repr=True)
    offscreen: Optional[OffscreenBehavior] = builtin_property(212, is_repr=True)
    transition: Optional["Transition"] = builtin_property(213, is_repr=True)
