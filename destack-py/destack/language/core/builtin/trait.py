from typing import (
    TYPE_CHECKING,
    ClassVar,
    cast,
    dataclass_transform,
)

from destack.language.registry import (
    TRAIT_CLASS_BY_TYPE,
    TRAIT_TYPE_BY_CLASS,
)
from destack.utils.func import get_superclasses

from .common import EnumType, NodeType, TraitType
from .object import BuiltinObject, _process_object_cls
from .property import (
    _PROPERTY_SPECIFIERS,
    _resolve_trait_type,
)

if TYPE_CHECKING:
    from destack.language import (
        ActionDefinition,
        Entity,
        MethodDefinition,
        PermissionDeclaration,
        PermissionDefinition,
    )

# pyright: reportIncompatibleVariableOverride=false


TRAIT_PREFIXES = ("Is",)

# traits where all matching nodes are ordered together


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def builtin_trait(
    trait_type: TraitType | None,
    *,
    is_extensible: bool = False,
    event_types: tuple[NodeType, ...] = (),
    permissions: tuple["PermissionDeclaration", ...] = (),
):
    """Register a class as an Entity trait."""

    def decorate(cls: type) -> type:
        cls, _ = _process_object_cls(
            cls=cast(type["Trait"], cls),
            object_type=None,
            is_frozen=False,
            is_concrete=False,
            is_struct=False,
            is_node=True,
            is_entity=False,
            is_root_node=False,
            is_abstract=True,
            base_type=None,
            traits=(),
        )
        if trait_type is not None:
            cls.metatype = trait_type
        cls.__is_trait__ = True
        cls.__is_abstract__ = True

        # traits cannot have properties
        real_properties = [
            p for p in cls.__properties__.values() if p.is_wired and p.name != "metatype"
        ]
        assert not real_properties, f"Trait {cls.__name__} has properties: {real_properties}"

        # traits
        traits: list[TraitType] = []
        base_traits: list[TraitType] = []
        for base in cls.__bases__:
            if trait := _resolve_trait_type(base.__name__):
                if trait not in base_traits:
                    base_traits.append(trait)
        for superclass in get_superclasses(cls):
            if superclass is cls:
                continue
            if trait := _resolve_trait_type(superclass.__name__):
                if trait not in traits:
                    traits.append(trait)
        cls.__traits__ = tuple(traits)
        cls.__base_traits__ = tuple(base_traits)
        cast(type["Trait"], cls).__is_extensible__ = is_extensible

        # event types
        cls.__base_event_types__ = tuple(event_types)

        # meta
        if permissions:
            from .definition import PermissionDefinition

            cls.__permissions__ = tuple(
                PermissionDefinition.from_declaration(permission) for permission in permissions
            )

        # register
        if trait_type is not None:
            TRAIT_CLASS_BY_TYPE[trait_type] = cls
            TRAIT_TYPE_BY_CLASS[cast(type["Trait"], cls)] = trait_type

        return cls

    return decorate


@builtin_trait(None)
class Trait(Entity if TYPE_CHECKING else BuiltinObject):
    """
    An Entity trait that ascribes some behavior to an Entity.

    Traits may define logic and access, but no properties.
    """

    # meta
    metatype: ClassVar[TraitType]

    # flags
    __is_node__: ClassVar[bool] = True
    __is_trait__: ClassVar[bool] = True
    """Whether this trait can be extended by custom Traits."""
    __is_extensible__: ClassVar[bool] = False

    # content
    """The permissions defined for this Trait."""
    __permissions__: ClassVar[tuple["PermissionDefinition", ...]] = ()
    """The methods defined for this Trait."""
    __methods__: ClassVar[tuple["MethodDefinition", ...]] = ()
    """The actions defined for this Trait."""
    __actions__: ClassVar[tuple["ActionDefinition", ...]] = ()

    # event
    """The base event types of this Trait (directly)."""
    __base_event_types__: ClassVar[tuple[NodeType, ...]] = ()
    """The event types of this trait (directly and indirectly)."""
    __event_types__: ClassVar[tuple[NodeType, ...]] = ()

    # enum
    """The base enum types of this trait (directly)."""
    __base_enum_types__: ClassVar[tuple[EnumType, ...]] = ()
    """The enum types of this trait (directly and indirectly)."""
    __enum_types__: ClassVar[tuple[EnumType, ...]] = ()


@builtin_trait(TraitType.ORDERED, is_extensible=True)
class IsOrdered(Trait):
    """An Entity that can be ordered."""

    pass


@builtin_trait(TraitType.OWNABLE, is_extensible=True)
class IsOwnable(Trait):
    """An Entity that can be owned by an Actor."""

    pass


@builtin_trait(TraitType.OWNED)
class IsOwned(IsOwnable):
    """An Entity that must be owned by an Actor."""

    pass


@builtin_trait(TraitType.JOINABLE, is_extensible=True)
class IsJoinable(Trait):
    """An Entity that can be joined by Actors."""

    pass


@builtin_trait(TraitType.ACTOR)
class IsActor(Trait):
    """An Entity that can be an Actor (can do something)."""

    pass


@builtin_trait(
    TraitType.REACTABLE,
    is_extensible=True,
    event_types=(NodeType.REACTION_EVENT,),
)
class IsReactable(Trait):
    """An Entity that can be reacted to (with Reactions)."""

    pass


@builtin_trait(
    TraitType.STARABLE,
    is_extensible=True,
    event_types=(NodeType.STAR_EVENT,),
)
class IsStarable(Trait):
    """An Entity that can be starred (with Stars)."""

    pass


@builtin_trait(
    TraitType.FOLLOWABLE,
    is_extensible=True,
    event_types=(NodeType.FOLLOW_EVENT,),
)
class IsFollowable(Trait):
    """An Entity that can be followed (with Follows)."""

    pass


@builtin_trait(TraitType.INTERACTIVE, is_extensible=True)
class IsInteractive(Trait):
    """An Entity that can be interacted with."""

    pass


@builtin_trait(TraitType.DRAGGABLE, is_extensible=True)
class IsDraggable(Trait):
    """An Entity that can be dragged."""

    pass


@builtin_trait(TraitType.SELECTABLE, is_extensible=True)
class IsSelectable(Trait):
    """An Entity that can be selected."""

    pass


@builtin_trait(TraitType.RUNNABLE)
class IsRunnable(Trait):
    """An Entity that can be (directly, with Runs)."""

    pass
