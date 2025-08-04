from datetime import timedelta
from typing import TYPE_CHECKING, Optional

from destack.core import (
    EnumType,
    Float32,
    NodeType,
    OptionEnum,
    StructFrozen,
    StructType,
    declare_entity,
    declare_enum,
    declare_option,
    declare_property,
    declare_struct,
)

from .transition import Transition

if TYPE_CHECKING:
    from destack import Axis3, Vector2


# pyright: reportIncompatibleVariableOverride=false


@declare_enum(EnumType.EFFECT_TYPE)
class EffectType(OptionEnum):
    """When the effect fires."""

    APPEAR = declare_option(10, description="Initial render in")
    ENTER = declare_option(11, description="Enters viewport")
    EXIT = declare_option(12, description="Leaves viewport")
    HOVER = declare_option(20, description="While hover")
    PRESS = declare_option(21, description="While tap")
    DRAG = declare_option(22, description="While drag / drag")
    FOCUS = declare_option(23, description="While focus")
    LOOP = declare_option(30, description="Continuous loop")
    # SCROLL, ...


@declare_enum(EnumType.REPEAT_TYPE)
class RepeatType(OptionEnum):
    LOOP = declare_option(1, description="Restart from beginning")
    REVERSE = declare_option(2, description="Yoyo back and forth")
    MIRROR = declare_option(3, description="Mirror keyframes")


@declare_enum(EnumType.TEXT_SPLIT_TYPE)
class TextSplitType(OptionEnum):
    CHAR = declare_option(1, description="Split by character")
    WORD = declare_option(2, description="Split by word")
    LINE = declare_option(3, description="Split by line")


@declare_enum(EnumType.OFFSCREEN_BEHAVIOR)
class OffscreenBehavior(OptionEnum):
    """What happens when the element is offscreen."""

    PLAY = declare_option(1, description="Play the animation")
    PAUSE = declare_option(2, description="Pause the animation")


@declare_struct(StructType.EFFECT, frozen=True)
class Effect(StructFrozen):
    """An effect value."""

    type: EffectType = declare_property(100, is_repr=True)
    style: Optional["EffectStyle"] = declare_property(101, is_repr=True)
    opacity: Optional[Float32] = declare_property(102, is_repr=True)
    offset: Optional["Vector2"] = declare_property(103, is_repr=True)
    scale: Optional[Float32] = declare_property(104, is_repr=True)
    rotate: Optional["Axis3"] = declare_property(105, is_repr=True)
    skew: Optional["Vector2"] = declare_property(106, is_repr=True)
    perspective: Optional[Float32] = declare_property(107, is_repr=True)
    delay: Optional[timedelta] = declare_property(108, is_repr=True)
    duration: Optional[Float32] = declare_property(109, is_repr=True)
    threshold: Optional[Float32] = declare_property(110, is_repr=True)
    once: Optional[bool] = declare_property(111, is_repr=True)
    repeat: Optional[RepeatType] = declare_property(112, is_repr=True)
    split: Optional[TextSplitType] = declare_property(113, is_repr=True)
    offscreen: Optional[OffscreenBehavior] = declare_property(114, is_repr=True)
    transition: Optional["Transition"] = declare_property(115, is_repr=True)


@declare_entity(NodeType.EFFECT_STYLE)
class EffectStyle(Style):
    """An effect style."""

    type: EffectType = declare_property(100, is_repr=True)
    opacity: Optional[Float32] = declare_property(200, is_repr=True)
    offset: Optional["Vector2"] = declare_property(201, is_repr=True)
    scale: Optional[Float32] = declare_property(202, is_repr=True)
    rotate: Optional["Axis3"] = declare_property(203, is_repr=True)
    skew: Optional["Vector2"] = declare_property(204, is_repr=True)
    perspective: Optional[Float32] = declare_property(205, is_repr=True)
    delay: Optional[timedelta] = declare_property(206, is_repr=True)
    duration: Optional[Float32] = declare_property(207, is_repr=True)
    threshold: Optional[Float32] = declare_property(208, is_repr=True)
    once: Optional[bool] = declare_property(209, is_repr=True)
    repeat: Optional[RepeatType] = declare_property(210, is_repr=True)
    split: Optional[TextSplitType] = declare_property(211, is_repr=True)
    offscreen: Optional[OffscreenBehavior] = declare_property(212, is_repr=True)
    transition: Optional["Transition"] = declare_property(213, is_repr=True)
