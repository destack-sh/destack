from typing import (
    TYPE_CHECKING,
    ClassVar,
    Optional,
    Union,
    cast,
    dataclass_transform,
)

from destack.language.registry import (
    TRAIT_CLASS_BY_TYPE,
    TRAIT_TYPE_BY_CLASS,
)
from destack.utils.fractional import INTEGER_ZERO
from destack.utils.func import get_superclasses
from destack.utils.uuid import UUID

from .common import NodeType, TraitType
from .const import UNSET
from .object import BuiltinObject, _process_object_cls
from .property import (
    _PROPERTY_SPECIFIERS,
    _resolve_trait_type,
    builtin_property,
)

if TYPE_CHECKING:
    from destack.language import (
        ActionDefinition,
        ConstraintDefinition,
        Entity,
        IndexDefinition,
        MethodDefinition,
        Node,
        NodeReference,
        PermissionDeclaration,
        PermissionDefinition,
        Script,
        Value,
    )

# pyright: reportIncompatibleVariableOverride=false


TRAIT_PREFIXES = ("Is",)

# traits where all matching nodes are ordered together


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def builtin_trait(
    trait_type: TraitType | None,
    is_extensible: bool = False,
    event_types: tuple[NodeType, ...] = (),
    permissions: tuple["PermissionDeclaration", ...] = (),
):
    """Register a class as a node trait."""

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
            from ..common import PermissionDefinition

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
class Trait(Node if TYPE_CHECKING else BuiltinObject):
    """A Node trait."""

    metatype: ClassVar[TraitType]
    __is_node__: ClassVar[bool] = True
    __is_trait__: ClassVar[bool] = True

    # 1-20: node identity
    id: UUID = builtin_property(
        2,
        is_managed=True,
        is_eq=False,
        is_readonly=True,
        description="The universally unique identifier of this Node.",
    )

    """Whether this trait can be extended by custom Traits."""
    __is_extensible__: ClassVar[bool] = False

    """The base event types of this Trait (directly)."""
    __base_event_types__: ClassVar[tuple[NodeType, ...]] = ()
    """The event types of this trait (directly and indirectly)."""
    __event_types__: ClassVar[tuple[NodeType, ...]] = ()

    """The indexes for this Node type."""
    __indexes__: ClassVar[tuple["IndexDefinition", ...]] = ()
    """The constraints for this Node type."""
    __constraints__: ClassVar[tuple["ConstraintDefinition", ...]] = ()
    """The permissions for this Node type."""
    __permissions__: ClassVar[tuple["PermissionDefinition", ...]] = ()
    """The methods for this Node type."""
    __methods__: ClassVar[tuple["MethodDefinition", ...]] = ()
    """The actions for this Node type."""
    __actions__: ClassVar[tuple["ActionDefinition", ...]] = ()


@builtin_trait(TraitType.ORDERED, is_extensible=True)
class IsOrdered(Trait):
    """An Entity that can be ordered."""

    order_key: str = builtin_property(
        31,
        is_eq=False,
        is_managed=True,
        default=INTEGER_ZERO,
        description="The absolute order key of this Node in its parent.",
    )


#
# Access
#


@builtin_trait(TraitType.OWNABLE, is_extensible=True)
class IsOwnable(Trait):
    """An Entity that can be owned by an Actor."""

    owned_by: Optional["IsActor"] = builtin_property(32, is_repr=True)
    if TYPE_CHECKING:
        owned_by_ptr: Optional[NodeReference] = None


@builtin_trait(TraitType.OWNED)
class IsOwned(IsOwnable):
    """An Entity that must be owned by an Actor."""

    owned_by: "IsActor" = builtin_property(32, is_repr=True)
    if TYPE_CHECKING:
        owned_by_ptr: NodeReference = UNSET


@builtin_trait(TraitType.JOINABLE, is_extensible=True)
class IsJoinable(Trait):
    """An Entity that can be joined by Actors."""

    pass


@builtin_trait(TraitType.ACTOR)
class IsActor(Trait):
    """An Entity that can be an Actor (can do something)."""

    pass


#
# Space
#


@builtin_trait(TraitType.TAGGABLE, is_extensible=True)
class IsTaggable(Trait):
    """An Entity that can be tagged (with a Tag)."""

    pass


#
# Social
#


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


#
# View
#


@builtin_trait(TraitType.VIEWABLE)
class IsViewable(Trait):
    """An Entity that can be presented visually."""

    pass


#
# Logic
#


@builtin_trait(TraitType.SOURCEABLE)
class IsSourceable(IsOrdered):
    """An Entity that can be defined in a Script."""

    source: Optional["Script"] = builtin_property(
        60,
        is_managed=True,
        description="The Script that defines this Node.",
    )
    # source_type: ScriptType?
    # token_range, ...
    key: str | None = builtin_property(
        70,
        description="The key to uniquely identify this Node in reconciliation. If not set, name is used.",
    )
    # aliases: list[str]?


@builtin_trait(TraitType.SCRIPTABLE)
class IsScriptable(Trait):
    """An Entity that can be scripted."""

    script: Optional["Script"] = builtin_property(
        80, description="The main / root Script of this Node."
    )


@builtin_trait(TraitType.RUNNABLE)
class IsRunnable(Trait):
    """An Entity that can be (directly, with Runs)."""

    pass


#
# Common
#


@builtin_trait(TraitType.CUSTOMIZABLE)
class IsCustomizable(Trait):
    """An Entity that can be customized with custom Properties."""

    custom_values: dict[UUID, "Value"] = builtin_property(
        30,
        description="The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.",
    )


@builtin_trait(TraitType.EXTENSIBLE)
class IsExtensible(IsCustomizable, IsScriptable):
    """A Node that be extended by custom Nodes (i.e. used as a base type)."""

    definition: Union["Entity", None] = builtin_property(
        6,
        is_managed=True,
        is_readonly=True,
        description="The definition this CustomEntity is an instance of.",
    )
    # inherits?
    # base_traits/base_trait_types?
    if TYPE_CHECKING:
        definition_ptr: Optional[NodeReference] = None

    is_extensible: bool = builtin_property(
        90,
        default=False,
        is_managed=True,
        is_readonly=True,
        description="Whether this Node is extensible (whether it can be instanced).",
    )
    # is_trait? is_abstract?

    @property
    def is_custom(self) -> bool:
        """Whether this Node is a custom Node."""
        return self.definition is not None


@builtin_trait(TraitType.IRREVERSIBLE, is_extensible=True)
class IsIrreversible(Trait):
    """An Entity that is fixed / forward-only in spacetime."""

    pass
