from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    BuiltinEnum,
    BuiltinObject,
    EnumType,
    Node,
    NodeType,
    Struct,
    StructType,
    enum_,
    node_,
    object_,
    p_regular,
    struct_,
)
from bench.pb2 import TransitionStyleData

from .core import IsVariable
from .style import IsStyle

if TYPE_CHECKING:
    pass


# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.TRANSITION_TYPE)
class TransitionType(BuiltinEnum):
    """Built-in transition types."""

    STYLE = 2
    FIELD = 3
    TWEEN = 10
    SPRING = 11


@enum_(EnumType.SPRING_TYPE)
class SpringType(BuiltinEnum):
    """Built-in spring types."""

    TIME = 1
    PHYSICS = 2


@object_()
class TransitionBase(IsVariable, BuiltinObject):
    type: TransitionType = p_regular(30, default=TransitionType.TWEEN)
    style: Optional["TransitionStyle"] = p_regular(41)
    delay: float | None = p_regular(50)
    duration: float | None = p_regular(51)
    ease: list[float] | None = p_regular(52)
    stiffness: float | None = p_regular(53)
    damping: float | None = p_regular(54)
    mass: float | None = p_regular(55)
    bounce: float | None = p_regular(56)
    spring_type: SpringType | None = p_regular(57)


@struct_(StructType.TRANSITION)
class Transition(TransitionBase, Struct):
    """A transition value."""

    pass


@node_(NodeType.TRANSITION_STYLE)
class TransitionStyle(TransitionBase, IsStyle, Node[TransitionStyleData]):
    """A transition style."""

    pass
