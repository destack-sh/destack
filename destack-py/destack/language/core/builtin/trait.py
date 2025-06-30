from collections.abc import Collection
from dataclasses import dataclass
from datetime import datetime
from typing import (
    TYPE_CHECKING,
    ClassVar,
    Optional,
    assert_never,
    cast,
    dataclass_transform,
)

from destack.language.registry import (
    NODE_TYPES_BY_TRAIT_TYPE,
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
    property_,
    property_parent_,
)

if TYPE_CHECKING:
    from destack.language import (
        Icon,
        Node,
        NodeDefinition,
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
INFECTIOUS_TRAITS = (TraitType.ARCHIVABLE, TraitType.DELETABLE)

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


def expand_node_types(types: Collection[NodeType | TraitType]) -> tuple[NodeType, ...]:
    """Expand a collection of NodeTypes and Traits into a flat collection of NodeTypes."""
    node_types: set[NodeType] = set()
    for typ in types:
        if isinstance(typ, NodeType):
            node_types.add(typ)
        elif isinstance(typ, TraitType):
            node_types.update(NODE_TYPES_BY_TRAIT_TYPE.get(typ, ()))
        else:
            assert_never(typ)
    return tuple(node_types)


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def builtin_trait(
    trait_type: TraitType | None,
    pretend_frozen: bool = False,  # :PretendFrozen
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

    __is_node__: ClassVar[bool] = True
    __is_trait__: ClassVar[bool] = False  # override Trait.__is_trait__
    __indexes__: ClassVar[tuple[IndexIn, ...]] = ()
    __is_abstract__: ClassVar[bool] = False

    """The base type this Node extends (directly)."""
    __base_type__: ClassVar[NodeType | None] = None
    """Nodes that this Node extends (directly and indirectly)."""
    __extends__: ClassVar[tuple[NodeType, ...]] = ()
    """Nodes that extend this Node type (directly)."""
    __extended_by__: ClassVar[tuple[NodeType, ...]] = ()
    """Nodes that extend this Node type (directly and indirectly)."""
    __inherited_by__: ClassVar[tuple[NodeType, ...]] = ()
    """Traits directly inherited by this Node (directly)."""
    __base_traits__: ClassVar[tuple[TraitType, ...]] = ()
    """Traits directly and indirectly inherited by this Node (directly and indirectly)."""
    __traits__: ClassVar[tuple[TraitType, ...]] = ()

    __root_type__: ClassVar[NodeType | None] = None
    __parent_property__: ClassVar[PropertyDeclaration] = UNSET
    __parent_types__: ClassVar[tuple[NodeType, ...]] = ()
    __child_types__: ClassVar[tuple[NodeType, ...]] = ()
    __ancestor_types__: ClassVar[tuple[NodeType, ...]] = ()
    __descendant_types__: ClassVar[tuple[NodeType, ...]] = ()

    # 15-29: node tracking
    # IsTracked.created_at/created_by/updated_at/updated_by: 15-18
    # IsArchivable.archived_at: 19
    # IsDeletable.deleted_at: 20
    # IsExtensible.value: 21
    # IsOrdered.order_key: 22
    # IsOwnable.owned_by: 25
    # ...managed_by/controlled_by?

    # 30+ for general properties
    # ...


@builtin_trait(None)
class Trait(Node if TYPE_CHECKING else NodeBase):
    """A Node trait."""

    metatype: ClassVar[TraitType]

    # 1-9: node identity
    #  (repeat common Node properties here so Trait NodeDefinitionReferences can reference them,
    #   since Trait doesn't actually inherit from Node for circularity reasons;
    #   but it is still useful to pretend so for typing since Python doesn't support `Trait & Node`)
    id: UUID = property_(2, is_managed=True, is_eq=False, can_write=None)
    parent: Optional["Node"] = property_parent_(node_is_customizable=True)
    if TYPE_CHECKING:
        parent_ptr: Optional[NodeReference] = None

    __is_node__: ClassVar[bool] = True
    __is_trait__: ClassVar[bool] = True
    __traits__: ClassVar[tuple[TraitType, ...]] = ()
    __indexes__: ClassVar[tuple[IndexIn, ...]] = ()


#
# Has* Traits (has specific properties)
#


@builtin_trait(TraitType.HAS_NAME)
class HasName(Trait):
    """A Node with a plain name."""

    name: str = property_(31, is_repr=True)


@builtin_trait(TraitType.HAS_SLUG)
class HasSlug(Trait):
    """A Node with a slug."""

    slug: str | None = property_(33, is_repr=True)


@builtin_trait(TraitType.HAS_ICON)
class HasIcon(Trait):
    """A Node with an icon."""

    icon: Optional["Icon"] = property_(34)


#
# Is* Traits (ascribes some behavior)
#


@builtin_trait(TraitType.ARCHIVABLE)
class IsArchivable(Trait):
    """A Node that can be archived."""

    archived_at: Optional[datetime] = property_(19, is_managed=True, is_eq=False)

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


@builtin_trait(TraitType.DELETABLE)
class IsDeletable(Trait):
    """A Node that can be deleted."""

    deleted_at: Optional[datetime] = property_(20, is_managed=True, is_eq=False)

    def delete(self):
        """Delete this Node."""
        assert not self.deleted_at, f"{self!r} is already deleted"
        self._session.delete(self)

    def restore(self):
        """Restore this deleted Node from the trash."""
        assert self.deleted_at, f"{self!r} is not deleted"
        self._session.restore(self)


@builtin_trait(TraitType.EXTENSIBLE)
class IsExtensible(Trait):
    # nocheckin: literally Extensible maybe? (can use as base type)
    #  related to is_abstract? (IsAbstract .. trait? or just flag?)
    #  (but then it can't be a trait because it would infect descendants..? Entity should be extensible?)
    """A Node that can be extended with custom Values (one Value per Field)."""

    value: dict[UUID, "Value"] = property_(21)


@builtin_trait(TraitType.ORDERED)
class IsOrdered(Trait):
    """A Node that can be ordered."""

    order_key: str = property_(22, is_eq=False, is_managed=True, default=INTEGER_ZERO)


@builtin_trait(TraitType.REACTABLE)
class IsReactable(Trait):
    """A Node that can be reacted to (with Reactions)."""

    pass


@builtin_trait(TraitType.STARABLE)
class IsStarable(Trait):
    """A Node that can be starred (with Stars)."""

    pass


@builtin_trait(TraitType.FOLLOWABLE)
class IsFollowable(Trait):
    """A Node that can be followed (with Follows)."""

    pass


@builtin_trait(TraitType.SOURCEABLE)
class IsSourceable(IsOrdered):
    """A Node that can be sourced from / defined by a Script."""

    source: Optional["Script"] = property_(210, is_managed=True)
    # token_range, ...


@builtin_trait(TraitType.SCRIPTABLE)
class IsScriptable(Trait):
    """A Node that can be scripted."""

    script: Optional["Script"] = property_(200, description="The main / root Script of this Node.")


@builtin_trait(TraitType.RUNNABLE)
class IsRunnable(Trait):
    """A Node that can be run (with Runs)."""

    pass


@builtin_trait(TraitType.OWNABLE)
class IsOwnable(Trait):
    """A Node that can be owned by another Node."""

    owned_by: Optional["IsOwner"] = property_(25, is_repr=True)
    if TYPE_CHECKING:
        owned_by_ptr: Optional[NodeReference] = None


@builtin_enum(EnumType.JOINABLE_PERMISSION)
class JoinablePermission(Enum):
    """A Permission for a Joinable."""

    INVITE = 1
    REMOVE = 2
    KICK = 3
    BAN = 4


@builtin_trait(TraitType.JOINABLE)
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


@builtin_trait(TraitType.TAGGABLE)
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

    space: "Space | None" = property_(
        5,
        is_managed=True,
        description="The Space this Node is in.",
    )
    if TYPE_CHECKING:
        space_ptr: Optional[NodeReference] = None
