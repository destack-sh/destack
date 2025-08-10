from typing import TYPE_CHECKING, Optional

from destack.core import (
    Entity,
    EnumType,
    Float32,
    NodeType,
    OptionEnum,
    Struct,
    StructType,
    TraitType,
    declare_entity,
    declare_enum,
    declare_option,
    declare_property,
    declare_struct,
)

if TYPE_CHECKING:
    pass


@declare_enum(EnumType.TRANSITION_TYPE)
class TransitionType(OptionEnum):
    """Built-in transition types."""

    TWEEN = declare_option(10)
    SPRING = declare_option(11)


@declare_enum(EnumType.SPRING_TYPE)
class SpringType(OptionEnum):
    """Built-in spring types."""

    TIME = declare_option(1)
    PHYSICAL = declare_option(2)


@declare_struct(StructType.TRANSITION)
class Transition(Struct):
    """A transition value."""

    type: TransitionType = declare_property(100, default=TransitionType.TWEEN, is_repr=True)
    template: Optional["TransitionTemplate"] = declare_property(101, is_repr=True)
    delay: Optional[Float32] = declare_property(102, is_repr=True)
    duration: Optional[Float32] = declare_property(103, is_repr=True)
    ease: list[Float32] = declare_property(104, is_repr=True)
    stiffness: Optional[Float32] = declare_property(105, is_repr=True)
    damping: Optional[Float32] = declare_property(106, is_repr=True)
    mass: Optional[Float32] = declare_property(107, is_repr=True)
    bounce: Optional[Float32] = declare_property(108, is_repr=True)
    spring_type: Optional[SpringType] = declare_property(109, is_repr=True)


@declare_entity(
    NodeType.TRANSITION_TEMPLATE,
    base_struct_type=StructType.TRANSITION,
    traits=(TraitType.TEMPLATE,),
)
class TransitionTemplate(Entity):
    """A transition template."""

    type: TransitionType = declare_property(100, default=TransitionType.TWEEN, is_repr=True)
    delay: Optional[Float32] = declare_property(102, is_repr=True)
    duration: Optional[Float32] = declare_property(103, is_repr=True)
    ease: list[Float32] = declare_property(104, is_repr=True)
    stiffness: Optional[Float32] = declare_property(105, is_repr=True)
    damping: Optional[Float32] = declare_property(106, is_repr=True)
    mass: Optional[Float32] = declare_property(107, is_repr=True)
    bounce: Optional[Float32] = declare_property(108, is_repr=True)
    spring_type: Optional[SpringType] = declare_property(109, is_repr=True)
