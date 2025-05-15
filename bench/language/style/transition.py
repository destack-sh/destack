from typing import Optional

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
from bench.pb2 import TransitionStyleData

from .style import StyleBase


@enum_(EnumType.TRANSITION_TYPE)
class TransitionType(BuiltinEnum):
    """Built-in transition types."""

    STYLE = 2
    TWEEN = 10
    SPRING = 11


@enum_(EnumType.SPRING_TYPE)
class SpringType(BuiltinEnum):
    """Built-in spring types."""

    TIME = 1
    PHYSICS = 2


@node_component_()
class TransitionBase(BuiltinObject):
    type: TransitionType = p_regular(30, default=TransitionType.TWEEN)
    style: Optional["TransitionStyle"] = p_regular(
        40, default=None, require=False, array=False, references=NodeType.TRANSITION_STYLE
    )
    delay: float | None = p_regular(41, default=None)
    duration: float | None = p_regular(42, default=None)
    ease: list[float] | None = p_regular(43, array=True)
    stiffness: float | None = p_regular(44, default=None)
    damping: float | None = p_regular(45, default=None)
    mass: float | None = p_regular(46, default=None)
    bounce: float | None = p_regular(47, default=None)
    spring_type: SpringType | None = p_regular(48, default=None)


@struct_(StructType.TRANSITION)
class Transition(TransitionBase, Struct):
    """A transition value."""

    pass


@node_(NodeType.TRANSITION_STYLE)
class TransitionStyle(TransitionBase, StyleBase[TransitionStyleData]):
    """A transition style."""

    pass
