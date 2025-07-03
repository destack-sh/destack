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
from destack.proto import AnyObjectProto
from destack.utils.fractional import INTEGER_ZERO
from destack.utils.func import get_superclasses
from destack.utils.uuid import UUID

from .common import (
    Enum,
    EnumType,
    NodeType,
    StoreType,
    TraitType,
    builtin_enum,
)
from .const import UNSET
from .constant import register_constant
from .object import BuiltinObjectMutable, _process_object_cls
from .property import (
    _PROPERTY_SPECIFIERS,
    PropertyDeclaration,
    _resolve_trait_type,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import (
        CustomEntityDefinition,
        CustomEventDefinition,
        Node,
        NodeDefinition,
        NodeDefinitionReference,
        NodeReference,
        Script,
        Space,
        Value,
    )

# pyright: reportIncompatibleVariableOverride=false


TRAIT_PREFIXES = ("Is", "Has", "Like")
# traits you must have at least one of
AT_LEAST_ONE_TRAITS = ((TraitType.GLOBAL, TraitType.SPATIAL),)
# traits you can have at most one of
AT_MOST_ONE_TRAITS = ()
# traits where every descendant must have the trait
INFECTIOUS_TRAITS = (TraitType.DELETABLE,)

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
):
    """Register a class as a node trait."""

    def decorate(cls: type["NodeBase"]) -> type["NodeBase"]:
        cls, _ = _process_object_cls(
            cls=cls,
            object_type=None,
            is_concrete=False,
            is_node=True,
            is_abstract=True,
            is_frozen=pretend_frozen,
        )
        cls.__is_trait__ = True
        cls.__is_abstract__ = True
        traits = set()
        base_traits = set()
        for superclass in get_superclasses(cls):
            if superclass == cls:
                continue
            if super_trait_type := _resolve_trait_type(superclass.__name__):
                traits.add(super_trait_type)
            if super_trait_type := _resolve_trait_type(superclass.__name__):
                base_traits.add(super_trait_type)
        cls.__traits__ = tuple(traits)
        cls.__base_traits__ = tuple(base_traits)
        cast(type["Trait"], cls).__is_extensible__ = is_extensible

        # register
        if trait_type is not None:
            cls.metatype = trait_type
            TRAIT_CLASS_BY_TYPE[trait_type] = cls
            TRAIT_TYPE_BY_CLASS[cast(type["Trait"], cls)] = trait_type

        return cls

    return decorate


@builtin_trait(trait_type=None)  # type: ignore
class NodeBase[NodeProtoT: AnyObjectProto](BuiltinObjectMutable[NodeProtoT]):
    """A Node with Properties and a persistent identity."""

    metatype: ClassVar[TraitType | NodeType]
    __is_node__: ClassVar[bool] = True

    __definition__: ClassVar["NodeDefinition"]

    """Whether this class is an actual Node (not a Trait)."""
    __is_node__: ClassVar[bool] = True
    """Whether this class is a Trait (not a Node)."""
    __is_trait__: ClassVar[bool] = False  # override Trait.__is_trait__
    """Indexes for this Node."""
    __indexes__: ClassVar[tuple[IndexIn, ...]] = ()

    """Whether this class is abstract (not concrete)."""
    __is_abstract__: ClassVar[bool] = False
    """The base type this Node extends (directly)."""
    __base_type__: ClassVar[NodeType | None] = None
    """Nodes that extend this Node type (directly)."""
    __extended_by__: ClassVar[tuple[NodeType, ...]] = ()
    """Nodes that this Node extends (directly and indirectly)."""
    __inherits__: ClassVar[tuple[NodeType, ...]] = ()
    """Nodes that extend this Node type (directly and indirectly)."""
    __inherited_by__: ClassVar[tuple[NodeType, ...]] = ()
    """Traits directly inherited by this Node (directly)."""
    __base_traits__: ClassVar[tuple[TraitType, ...]] = ()
    """Traits directly and indirectly inherited by this Node (directly and indirectly)."""
    __traits__: ClassVar[tuple[TraitType, ...]] = ()
    """The main StoreTypes this Node is primarily stored in."""
    __primary_store_types__: ClassVar[tuple[StoreType, ...]] = ()

    """The root ancestor type of this Node type (if any)."""
    __root_type__: ClassVar[NodeType | None] = None
    """The parent type of this Node type (directly)."""
    __parent_property__: ClassVar[PropertyDeclaration] = UNSET
    """The parent classes of this Node type (directly)."""
    __parent_classes__: ClassVar[tuple[type["Node"], ...]] = ()
    """The parent types of this Node type (directly)."""
    __parent_types__: ClassVar[tuple[NodeType, ...]] = ()
    """The child types of this Node type (directly)."""
    __child_types__: ClassVar[tuple[NodeType, ...]] = ()
    """The ancestor types of this Node type (directly and indirectly)."""
    __ancestor_types__: ClassVar[tuple[NodeType, ...]] = ()
    """The descendant types of this Node type (directly and indirectly)."""
    __descendant_types__: ClassVar[tuple[NodeType, ...]] = ()

    # 20-40: node tracking
    # IsTracked.created_at/created_by/updated_at/updated_by: 20-23
    # IsArchivable.archived_at: 24
    # IsDeletable.deleted_at: 25
    # IsCustomizable.custom_values: 26
    # IsOrdered.order_key: 27
    # IsOwnable.owned_by: 28
    # ...managed_by/controlled_by?

    # 40-100: internal properties
    # ...

    # 100+ for general properties
    # ...


@builtin_trait(None)
class Trait(Node if TYPE_CHECKING else NodeBase):
    """A Node trait."""

    metatype: ClassVar[TraitType]
    __is_node__: ClassVar[bool] = True
    __is_trait__: ClassVar[bool] = True

    # 1-20: node identity
    #  (repeat common Node properties here so Trait NodeDefinitionReferences can reference them,
    #   since Trait doesn't actually inherit from Node for circularity reasons;
    #   but it is still useful to pretend so for typing since Python doesn't support `Trait & Node`)
    id: UUID = builtin_property(2, is_managed=True, is_eq=False, can_write=None)
    parent: Optional["Node"] = builtin_property_parent(node_is_extensible=True)
    if TYPE_CHECKING:
        parent_ptr: Optional[NodeReference] = None

    __is_extensible__: ClassVar[bool] = False


@builtin_trait(TraitType.ARCHIVABLE, is_extensible=True)
class IsArchivable(Trait):
    """A Node that can be archived."""

    archived_at: Optional[datetime] = builtin_property(24, is_managed=True, is_eq=False)

    @property
    def is_archived(self) -> bool:
        return self.archived_at is not None

    def archive(self):
        """Archive this Node."""
        assert not self.archived_at, f"{self!r} is already archived"
        self._session.archive(self)

    def unarchive(self):
        """Unarchive this Node."""
        assert self.archived_at, f"{self!r} is not archived"
        self._session.unarchive(self)


@builtin_trait(TraitType.DELETABLE, is_extensible=True)
class IsDeletable(Trait):
    """A Node that can be deleted."""

    deleted_at: Optional[datetime] = builtin_property(25, is_managed=True, is_eq=False)

    def delete(self):
        """Delete this Node."""
        assert not self.deleted_at, f"{self!r} is already deleted"
        self._session.delete(self)

    def restore(self):
        """Restore this deleted Node from the trash."""
        assert self.deleted_at, f"{self!r} is not deleted"
        self._session.restore(self)


@builtin_trait(TraitType.CUSTOMIZABLE)
class IsCustomizable(Trait):
    """A Node that can be customized with custom Properties."""

    custom_values: dict[UUID, "Value"] = builtin_property(
        26,
        description="The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.",
    )


@builtin_trait(TraitType.EXTENSIBLE)
class IsExtensible(IsCustomizable):
    """A Node that be extended by custom Nodes (i.e. used as a base type)."""

    definition: Union["CustomEntityDefinition", "CustomEventDefinition", None] = builtin_property(
        6,
        is_managed=True,
        is_readonly=True,
        description="The definitionthis CustomEntity is an instance of.",
    )
    base_type: "NodeDefinitionReference | None" = builtin_property(
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


@builtin_trait(TraitType.ORDERED, is_extensible=True)
class IsOrdered(Trait):
    """A Node that can be ordered."""

    order_key: str = builtin_property(
        27,
        is_eq=False,
        is_managed=True,
        default=INTEGER_ZERO,
        description="The absolute order key of this Node in its parent.",
    )


@builtin_trait(TraitType.REACTABLE, is_extensible=True)
class IsReactable(Trait):
    """A Node that can be reacted to (with Reactions)."""

    pass


@builtin_trait(TraitType.STARABLE, is_extensible=True)
class IsStarable(Trait):
    """A Node that can be starred (with Stars)."""

    pass


@builtin_trait(TraitType.FOLLOWABLE, is_extensible=True)
class IsFollowable(Trait):
    """A Node that can be followed (with Follows)."""

    pass


@builtin_trait(TraitType.SOURCEABLE)
class IsSourceable(IsOrdered):
    """A Node that can be sourced from / defined by a Script."""

    source: Optional["Script"] = builtin_property(60, is_managed=True)
    # token_range, ...


@builtin_trait(TraitType.SCRIPTABLE)
class IsScriptable(Trait):
    """A Node that can be scripted."""

    script: Optional["Script"] = builtin_property(
        70, description="The main / root Script of this Node."
    )


@builtin_trait(TraitType.RUNNABLE)
class IsRunnable(Trait):
    """A Node that can be run (with Runs)."""

    pass


@builtin_trait(TraitType.OWNABLE, is_extensible=True)
class IsOwnable(Trait):
    """A Node that can be owned by another Node."""

    owned_by: Optional["IsOwner"] = builtin_property(28, is_repr=True)
    if TYPE_CHECKING:
        owned_by_ptr: Optional[NodeReference] = None


@builtin_enum(EnumType.JOINABLE_PERMISSION)
class JoinablePermission(Enum):
    """A Permission for a Joinable."""

    INVITE = 1
    REMOVE = 2
    KICK = 3
    BAN = 4


@builtin_trait(TraitType.JOINABLE, is_extensible=True)
class IsJoinable(Trait):
    """A Node that can be joined by Subjects."""

    pass


@builtin_trait(TraitType.SUBJECT)
class IsSubject(Trait):
    """A Node that can be a Subject."""

    pass


@builtin_trait(TraitType.OWNER)
class IsOwner(Trait):
    """A Node that can be an Owner."""

    pass


@builtin_trait(TraitType.TAGGABLE, is_extensible=True)
class IsTaggable(Trait):
    """A Node that can be tagged (with a Tag)."""

    pass


#
# Bare Traits (main type/location)
#


@builtin_trait(TraitType.GLOBAL)
class IsGlobal(Trait):
    """A Node that is global."""

    pass


@builtin_trait(TraitType.SPATIAL)
class IsSpatial(Trait):
    """A Node in a Space."""

    space: "Space | None" = builtin_property(
        5,
        is_managed=True,
        description="The Space this Node is in.",
    )
    if TYPE_CHECKING:
        space_ptr: Optional[NodeReference] = None
