from datetime import timedelta
from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    BuiltinEnum,
    BuiltinObject,
    EnumType,
    NodeType,
    Struct,
    StructType,
    enum_,
    node_,
    object_,
    p_regular,
    struct_,
)
from bench.pb2 import EffectStyleData

from .core import Axis3, IsVariable, Vector2
from .style import StyleBase
from .transition import Transition

if TYPE_CHECKING:
    pass


@enum_(EnumType.EFFECT_TYPE)
class EffectType(BuiltinEnum):
    """When the effect fires."""

    NONE = 1
    STYLE = 2
    FIELD = 3
    APPEAR = 10, "Appear", "Initial render in"
    ENTER = 11, "Enter", "Enters viewport"
    EXIT = 12, "Exit", "Leaves viewport"
    HOVER = 20, "Hover", "While hover"
    PRESS = 21, "Press", "While tap"
    DRAG = 22, "Drag", "While drag / drag"
    FOCUS = 23, "Focus", "While focus"
    LOOP = 30, "Loop", "Continuous loop"
    # SCROLL, ...


@enum_(EnumType.REPEAT_TYPE)
class RepeatType(BuiltinEnum):
    LOOP = 1, "Loop", "Restart from beginning"
    REVERSE = 2, "Reverse", "Yoyo back and forth"
    MIRROR = 3, "Mirror", "Mirror keyframes"


@enum_(EnumType.TEXT_SPLIT_TYPE)
class TextSplitType(BuiltinEnum):
    CHAR = 1, "Char", "Split by character"
    WORD = 2, "Word", "Split by word"
    LINE = 3, "Line", "Split by line"


@enum_(EnumType.OFFSCREEN_BEHAVIOR)
class OffscreenBehavior(BuiltinEnum):
    """What happens when the element is offscreen."""

    PLAY = 1, "Play", "Play the animation"
    PAUSE = 2, "Pause", "Pause the animation"


@object_()
class EffectBase(IsVariable, BuiltinObject):
    """A base class for effects."""

    type: EffectType = p_regular(30)
    style: Optional["EffectStyle"] = p_regular(41)

    opacity: Optional[float] = p_regular(50)
    offset: Optional[Vector2] = p_regular(51)
    scale: Optional[float] = p_regular(52)
    rotate: Optional[Axis3] = p_regular(53)
    skew: Optional[Vector2] = p_regular(54)
    perspective: Optional[float] = p_regular(55)
    delay: Optional[timedelta] = p_regular(56)
    duration: Optional[float] = p_regular(57)
    threshold: Optional[float] = p_regular(58)
    once: Optional[bool] = p_regular(59)
    repeat: Optional[RepeatType] = p_regular(60)
    split: Optional[TextSplitType] = p_regular(61)
    offscreen: Optional[OffscreenBehavior] = p_regular(62)

    transition: Optional["Transition"] = p_regular(70)


@struct_(StructType.EFFECT)
class Effect(EffectBase, Struct):
    """An effect value."""

    pass


@node_(NodeType.EFFECT_STYLE)
class EffectStyle(EffectBase, StyleBase[EffectStyleData]):
    """An effect style."""

    pass
