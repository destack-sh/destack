from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Enum,
    EnumType,
    NodeType,
    StructFrozen,
    StructType,
    builtin_enum,
    builtin_node,
    builtin_struct,
    property_,
)

from .style import Style

if TYPE_CHECKING:
    pass


# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.TRANSITION_TYPE)
class TransitionType(Enum):
    """Built-in transition types."""

    TWEEN = 10
    SPRING = 11


@builtin_enum(EnumType.SPRING_TYPE)
class SpringType(Enum):
    """Built-in spring types."""

    TIME = 1
    PHYSICS = 2


@builtin_struct(StructType.TRANSITION, frozen=True)
class Transition(StructFrozen):
    """A transition value."""

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


@builtin_node(NodeType.TRANSITION_STYLE)
class TransitionStyle(Style):
    """A transition style."""

    type: TransitionType = property_(30, default=TransitionType.TWEEN, is_repr=True)
    delay: float | None = property_(50, is_repr=True)
    duration: float | None = property_(51, is_repr=True)
    ease: list[float] = property_(52, is_repr=True)
    stiffness: float | None = property_(53, is_repr=True)
    damping: float | None = property_(54, is_repr=True)
    mass: float | None = property_(55, is_repr=True)
    bounce: float | None = property_(56, is_repr=True)
    spring_type: SpringType | None = property_(57, is_repr=True)
