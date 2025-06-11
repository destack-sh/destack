from datetime import timedelta
from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Axis3,
    BuiltinEnum,
    BuiltinObjectMutable,
    EnumType,
    IsArchivable,
    IsDeletable,
    IsTracked,
    Node,
    NodeType,
    StructMutable,
    StructType,
    Vector2,
    enum_,
    node_,
    object_,
    property_,
    struct_,
)
from destack.pb2 import EffectStyleData

from .style import IsStyle
from .transition import Transition

if TYPE_CHECKING:
    pass


# pyright: reportIncompatibleVariableOverride=false


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
class EffectBase(BuiltinObjectMutable):
    """A base class for effects."""

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


@struct_(StructType.EFFECT)
class Effect(EffectBase, StructMutable):
    """An effect value."""

    pass


@node_(NodeType.EFFECT_STYLE)
class EffectStyle(
    EffectBase,
    IsStyle,
    IsDeletable,
    IsArchivable,
    IsTracked,
    Node[EffectStyleData],
):
    """An effect style."""

    pass
