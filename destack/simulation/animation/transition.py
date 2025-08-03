from typing import TYPE_CHECKING, Optional

from destack.core import (
    Enum,
    EnumType,
    Float32,
    NodeType,
    StructFrozen,
    StructType,
    declare_entity,
    declare_enum,
    declare_property,
    declare_struct,
)

from ..style.style import Style

if TYPE_CHECKING:
    pass


# pyright: reportIncompatibleVariableOverride=false


@declare_enum(EnumType.TRANSITION_TYPE)
class TransitionType(Enum):
    """Built-in transition types."""

    TWEEN = 10
    SPRING = 11


@declare_enum(EnumType.SPRING_TYPE)
class SpringType(Enum):
    """Built-in spring types."""

    TIME = 1
    PHYSICAL = 2


@declare_struct(StructType.TRANSITION, frozen=True)
class Transition(StructFrozen):
    """A transition value."""

    type: TransitionType = declare_property(100, default=TransitionType.TWEEN, is_repr=True)
    style: Optional["TransitionStyle"] = declare_property(101, is_repr=True)
    delay: Optional[Float32] = declare_property(102, is_repr=True)
    duration: Optional[Float32] = declare_property(103, is_repr=True)
    ease: list[Float32] = declare_property(104, is_repr=True)
    stiffness: Optional[Float32] = declare_property(105, is_repr=True)
    damping: Optional[Float32] = declare_property(106, is_repr=True)
    mass: Optional[Float32] = declare_property(107, is_repr=True)
    bounce: Optional[Float32] = declare_property(108, is_repr=True)
    spring_type: Optional[SpringType] = declare_property(109, is_repr=True)


@declare_entity(NodeType.TRANSITION_STYLE)
class TransitionStyle(Style):
    """A transition style."""

    type: TransitionType = declare_property(100, default=TransitionType.TWEEN, is_repr=True)
    delay: Optional[Float32] = declare_property(102, is_repr=True)
    duration: Optional[Float32] = declare_property(103, is_repr=True)
    ease: list[Float32] = declare_property(104, is_repr=True)
    stiffness: Optional[Float32] = declare_property(105, is_repr=True)
    damping: Optional[Float32] = declare_property(106, is_repr=True)
    mass: Optional[Float32] = declare_property(107, is_repr=True)
    bounce: Optional[Float32] = declare_property(108, is_repr=True)
    spring_type: Optional[SpringType] = declare_property(109, is_repr=True)
