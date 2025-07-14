from dataclasses import dataclass
from datetime import datetime
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
from .constant import register_constant
from .object import BuiltinObject, _process_object_cls
from .property import (
    _PROPERTY_SPECIFIERS,
    _resolve_trait_type,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import (
        CustomEntityDefinition,
        CustomEventDefinition,
        Entity,
        Node,
        NodeDefinitionReference,
        NodeReference,
        Script,
        Value,
    )

# pyright: reportIncompatibleVariableOverride=false


TRAIT_PREFIXES = ("Is",)

# traits where all matching nodes are ordered together
INTER_ORDER_TYPES = (NodeType.VIEW, NodeType.STYLE)
register_constant("INTER_ORDER_TYPES", INTER_ORDER_TYPES)


@dataclass(slots=True, frozen=True)
class IndexIn:
    """Index to be turned into a SQL Index."""

    columns: tuple[str, ...]
    cover: tuple[str, ...] = ()
    is_unique: bool = False
    condition: str | None = None
    name: str | None = None


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def builtin_trait(
    trait_type: TraitType | None,
    pretend_frozen: bool = False,  # :PretendFrozen
    is_extensible: bool = False,
    event_types: tuple[NodeType, ...] = (),
):
    """Register a class as a node trait."""

    def decorate(cls: type) -> type:
        cls, _ = _process_object_cls(
            cls=cast(type["Trait"], cls),
            object_type=None,
            is_concrete=False,
            is_node=True,
            is_abstract=True,
            is_frozen=pretend_frozen,
        )
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

        # register
        if trait_type is not None:
            cls.metatype = trait_type
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
    #  (repeat common Node properties here so Trait NodeDefinitionReferences can reference them,
    #   since Trait doesn't actually inherit from Node for circularity reasons;
    #   but it is still useful to pretend so for typing since Python doesn't support `Trait & Node`)
    id: UUID = builtin_property(2, is_managed=True, is_eq=False, can_write=None)
    parent: Optional["Entity"] = builtin_property_parent()
    if TYPE_CHECKING:
        parent_ptr: Optional[NodeReference] = None

    """Whether this trait can be extended by custom Traits."""
    __is_extensible__: ClassVar[bool] = False

    """The base event types of this Trait (directly)."""
    __base_event_types__: ClassVar[tuple[NodeType, ...]] = ()
    """The event types of this trait (directly and indirectly)."""
    __event_types__: ClassVar[tuple[NodeType, ...]] = ()


@builtin_trait(TraitType.ORDERED, is_extensible=True)
class IsOrdered(Trait):
    """An Entity that can be ordered."""

    order_key: str = builtin_property(
        27,
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
    """An Entity that can be owned by another Entity."""

    owned_by: Optional["IsSubject"] = builtin_property(28, is_repr=True)
    if TYPE_CHECKING:
        owned_by_ptr: Optional[NodeReference] = None


@builtin_trait(TraitType.OWNED)
class IsOwned(IsOwnable):
    """An Entity that must be owned by another Entity."""

    owned_by: "IsSubject" = builtin_property(28, is_repr=True)
    if TYPE_CHECKING:
        owned_by_ptr: NodeReference = UNSET


@builtin_trait(TraitType.JOINABLE, is_extensible=True)
class IsJoinable(Trait):
    """An Entity that can be joined by Subjects."""

    pass


@builtin_trait(TraitType.SUBJECT)
class IsSubject(Trait):
    """An Entity that can be a Subject."""

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

    name: str = builtin_property(
        101,
        is_repr=True,
        description="The name of this Node.",
    )


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


@builtin_trait(TraitType.ARCHIVABLE, is_extensible=True)
class IsArchivable(Trait):
    """An Entity that can be archived."""

    archived_at: Optional[datetime] = builtin_property(24, is_managed=True, is_eq=False)

    @property
    def is_archived(self) -> bool:
        return self.archived_at is not None

    def archive(self):
        """Archive this Node."""
        from destack.language.core.builtin import Entity

        assert isinstance(self, Entity), f"{self!r} is not an Entity"
        assert not self.archived_at, f"{self!r} is already archived"
        self._session.archive(self)

    def unarchive(self):
        """Unarchive this Node."""
        from destack.language.core.builtin import Entity

        assert isinstance(self, Entity), f"{self!r} is not an Entity"
        assert self.archived_at, f"{self!r} is not archived"
        self._session.unarchive(self)


@builtin_trait(TraitType.DELETABLE, is_extensible=True)
class IsDeletable(Trait):
    """An Entity that can be deleted."""

    deleted_at: Optional[datetime] = builtin_property(25, is_managed=True, is_eq=False)

    def delete(self):
        """Delete this Node."""
        from destack.language.core.builtin import Entity

        assert isinstance(self, Entity), f"{self!r} is not an Entity"
        assert not self.deleted_at, f"{self!r} is already deleted"
        self._session.delete(self)

    def restore(self):
        """Restore this deleted Node from the trash."""
        from destack.language.core.builtin import Entity

        assert isinstance(self, Entity), f"{self!r} is not an Entity"
        assert self.deleted_at, f"{self!r} is not deleted"
        self._session.restore(self)


@builtin_trait(TraitType.CUSTOMIZABLE)
class IsCustomizable(Trait):
    """An Entity that can be customized with custom Properties."""

    custom_values: dict[UUID, "Value"] = builtin_property(
        26,
        description="The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.",
    )


@builtin_trait(TraitType.EXTENSIBLE)
class IsExtensible(IsCustomizable, IsScriptable):
    """A Node that be extended by custom Nodes (i.e. used as a base type)."""

    definition: Union["CustomEntityDefinition", "CustomEventDefinition", None] = builtin_property(
        6,
        is_managed=True,
        is_readonly=True,
        description="The definitionthis CustomEntity is an instance of.",
    )
    base_type: Union["NodeDefinitionReference", None] = builtin_property(
        7,
        is_readonly=True,
        is_managed=True,
        description="Inlined base type of this extensible Node (if extended).",
    )
    # base_node_type?
    # inherits?
    # base_traits/base_trait_types?
    if TYPE_CHECKING:
        definition_ptr: Optional[NodeReference] = None

    @property
    def is_custom(self) -> bool:
        """Whether this Node is a custom Node."""
        return self.definition is not None


@builtin_trait(TraitType.IRREVERSIBLE, is_extensible=True)
class IsIrreversible(Trait):
    """An Entity that cannot be rewound in spacetime."""

    pass
