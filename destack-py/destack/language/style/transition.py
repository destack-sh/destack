from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Enum,
    EnumType,
    NodeType,
    StructFrozen,
    StructType,
    builtin_enum,
    builtin_node,
    builtin_property,
    builtin_struct,
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

    type: TransitionType = builtin_property(100, default=TransitionType.TWEEN, is_repr=True)
    style: Optional["TransitionStyle"] = builtin_property(101, is_repr=True)
    delay: float | None = builtin_property(102, is_repr=True)
    duration: float | None = builtin_property(103, is_repr=True)
    ease: list[float] = builtin_property(104, is_repr=True)
    stiffness: float | None = builtin_property(105, is_repr=True)
    damping: float | None = builtin_property(106, is_repr=True)
    mass: float | None = builtin_property(107, is_repr=True)
    bounce: float | None = builtin_property(108, is_repr=True)
    spring_type: SpringType | None = builtin_property(109, is_repr=True)


@builtin_node(NodeType.TRANSITION_STYLE)
class TransitionStyle(Style):
    """A transition style."""

    type: TransitionType = builtin_property(100, default=TransitionType.TWEEN, is_repr=True)
    delay: float | None = builtin_property(102, is_repr=True)
    duration: float | None = builtin_property(103, is_repr=True)
    ease: list[float] = builtin_property(104, is_repr=True)
    stiffness: float | None = builtin_property(105, is_repr=True)
    damping: float | None = builtin_property(106, is_repr=True)
    mass: float | None = builtin_property(107, is_repr=True)
    bounce: float | None = builtin_property(108, is_repr=True)
    spring_type: SpringType | None = builtin_property(109, is_repr=True)
