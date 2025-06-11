from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    BuiltinEnum,
    BuiltinObjectMutable,
    EnumType,
    Node,
    NodeType,
    StructMutable,
    StructType,
    enum_,
    node_,
    object_,
    property_,
    struct_,
)
from destack.pb2 import TransitionStyleData

from .style import Style

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
class TransitionBase(BuiltinObjectMutable):
    type: TransitionType = property_(30, default=TransitionType.TWEEN, is_repr=True)
    style: Optional["TransitionStyle"] = property_(41, is_repr=True)
    delay: float | None = property_(50, is_repr=True)
    duration: float | None = property_(51, is_repr=True)
    ease: list[float] = property_(52, is_repr=True)
    stiffness: float | None = property_(53, is_repr=True)
    damping: float | None = property_(54, is_repr=True)
    mass: float | None = property_(55, is_repr=True)
    bounce: float | None = property_(56, is_repr=True)
    spring_type: SpringType | None = property_(57, is_repr=True)


@struct_(StructType.TRANSITION)
class Transition(TransitionBase, StructMutable):
    """A transition value."""

    pass


@node_(NodeType.TRANSITION_STYLE)
class TransitionStyle(
    Style,
    TransitionBase,
    Node[TransitionStyleData],
):
    """A transition style."""

    pass
