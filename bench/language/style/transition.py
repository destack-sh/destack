from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    BuiltinEnum,
    BuiltinObject,
    EnumType,
    IsArchivable,
    IsDeletable,
    Node,
    NodeType,
    Struct,
    StructType,
    enum_,
    node_,
    object_,
    property_,
    struct_,
)
from bench.pb2 import TransitionStyleData

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
class TransitionBase(BuiltinObject):
    type: TransitionType = property_(30, default=TransitionType.TWEEN)
    style: Optional["TransitionStyle"] = property_(41)
    delay: float | None = property_(50)
    duration: float | None = property_(51)
    ease: list[float] = property_(52)
    stiffness: float | None = property_(53)
    damping: float | None = property_(54)
    mass: float | None = property_(55)
    bounce: float | None = property_(56)
    spring_type: SpringType | None = property_(57)


@struct_(StructType.TRANSITION)
class Transition(TransitionBase, Struct):
    """A transition value."""

    pass


@node_(NodeType.TRANSITION_STYLE)
class TransitionStyle(
    TransitionBase,
    IsStyle,
    IsDeletable,
    IsArchivable,
    Node[TransitionStyleData],
):
    """A transition style."""

    pass
