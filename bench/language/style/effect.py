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
    node_component_,
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


@node_component_()
class EffectBase(IsVariable, BuiltinObject):
    """A base class for effects."""

    type: EffectType = p_regular(30)
    style: Optional["EffectStyle"] = p_regular(
        41, default=None, require=False, array=False, references=NodeType.EFFECT_STYLE
    )

    opacity: Optional[float] = p_regular(50, default=None)
    offset: Optional[Vector2] = p_regular(51, default=None, struct=StructType.VECTOR2)
    scale: Optional[float] = p_regular(52, default=None)
    rotate: Optional[Axis3] = p_regular(53, default=None, struct=StructType.AXIS3)
    skew: Optional[Vector2] = p_regular(54, default=None, struct=StructType.AXIS2)
    perspective: Optional[float] = p_regular(55, default=None)
    delay: Optional[timedelta] = p_regular(56, default=None)
    duration: Optional[float] = p_regular(57, default=None)
    threshold: Optional[float] = p_regular(58, default=None)
    once: Optional[bool] = p_regular(59, default=None)
    repeat: Optional[RepeatType] = p_regular(60, default=None)
    split: Optional[TextSplitType] = p_regular(61, default=None)
    offscreen: Optional[OffscreenBehavior] = p_regular(62, default=None)

    transition: Optional["Transition"] = p_regular(
        70, default=None, require=False, array=False, struct=StructType.TRANSITION
    )


@struct_(StructType.EFFECT)
class Effect(EffectBase, Struct):
    """An effect value."""

    pass


@node_(NodeType.EFFECT_STYLE)
class EffectStyle(EffectBase, StyleBase[EffectStyleData]):
    """An effect style."""

    pass
