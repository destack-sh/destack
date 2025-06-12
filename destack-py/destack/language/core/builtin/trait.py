from collections.abc import Collection
from dataclasses import dataclass
from datetime import datetime
from typing import (
    TYPE_CHECKING,
    Any,
    ClassVar,
    Optional,
    Self,
    assert_never,
    cast,
    dataclass_transform,
)

from fastuuid import UUID

from destack.language.registry import (
    NODE_TYPES_BY_TRAIT_TYPE,
    RELATION_REF_BY_CLASS,
    TRAIT_CLASS_BY_TRAIT,
    TRAIT_TYPE_BY_CLASS,
)
from destack.pb2 import AnyObjectData
from destack.utils.fractional import INTEGER_ZERO

from .const import (
    UNSET,
    EdgeType,
    NodeType,
    ResourceStatus,
    TraitType,
)
from .object import BuiltinObjectMutable, _process_object_cls
from .property import (
    _PROPERTY_SPECIFIERS,
    Property,
    property_,
    property_ancestor_,
    property_parent_,
)

if TYPE_CHECKING:
    from destack.language import (
        AggregationType,
        Condition,
        ExpressionIn,
        Folder,
        Icon,
        Node,
        NodeInfo,
        NodeReference,
        Query,
        Script,
        Sort,
        Space,
        TraitInfo,
        Value,
    )

    from ..common.query import JoinIn

# pyright: reportIncompatibleVariableOverride=false

#
# NOTE: Traits follow the following naming scheme:
#  - Bare (e.g., Spatial, Global, Entity): the main type & location
#  - Like (e.g., LikeTag, LikeMembership): the trait mimics a specific builtin node type
#  - Has (e.g., HasName, HasSlug): the trait has specific properties
#  - Is (e.g., IsTaggable, IsOwnable): the trait ascribes some behavior
#

AT_LEAST_ONE_TRAITS = (
    (TraitType.GLOBAL, TraitType.SPATIAL),
    (TraitType.ENTITY, TraitType.PARTICLE, TraitType.ANALYTIC, TraitType.INDEXED),
)
AT_MOST_ONE_TRAITS = ((TraitType.ENTITY, TraitType.PARTICLE),)
INFECTIOUS_TRAITS = (
    TraitType.TEMPLATABLE,
    TraitType.ARCHIVABLE,
    TraitType.DELETABLE,
)
INTER_ORDER_TRAITS = (TraitType.VIEW, TraitType.STYLE)


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
def trait_(
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
            is_frozen=pretend_frozen,
        )

        # register
        if trait_type is not None:
            cls.metatype = trait_type
            TRAIT_CLASS_BY_TRAIT[trait_type] = cls
            TRAIT_TYPE_BY_CLASS[cast(type["Trait"], cls)] = trait_type

        return cls

    return decorate


@trait_(trait_type=None)  # type: ignore
class NodeBase[NodeDataT: AnyObjectData](BuiltinObjectMutable[NodeDataT]):
    """A Node with Properties and a persistent identity."""

    metatype: ClassVar[TraitType | NodeType]
    __is_node__: ClassVar[bool] = True

    __info__: ClassVar["NodeInfo"]

    __is_node__: ClassVar[bool] = True
    __is_trait__: ClassVar[bool] = False  # override Trait.__is_trait__
    __traits__: ClassVar[tuple[TraitType, ...]] = ()
    __indexes__: ClassVar[tuple[IndexIn, ...]] = ()

    __root_type__: ClassVar[NodeType | None] = None
    __parent_property__: ClassVar[Property] = UNSET
    __parent_types__: ClassVar[tuple[NodeType, ...]] = ()
    __child_types__: ClassVar[tuple[NodeType, ...]] = ()
    __ancestor_types__: ClassVar[tuple[NodeType, ...]] = ()
    __descendant_types__: ClassVar[tuple[NodeType, ...]] = ()

    # 10-29: node tracking
    # IsTracked.created_at/created_by/updated_at/updated_by: 10-13
    # IsArchivable.archived_at: 14
    # IsDeletable.deleted_at: 15
    # IsTemplatable.template: 16
    # IsCustomNode.definition: 17
    # IsExtensible.value: 18
    # IsOrdered.order_key: 19
    # IsTaggable.: 20/21
    # IsOwnable.owned_by: 22
    # ...managed_by/controlled_by?

    # 30+ for general properties
    # ...

    @classmethod
    def get(
        cls: type["Self"],
        where: Optional["Condition"] = None,
        *,
        name: str | None = None,
        join: Optional["JoinIn"] = None,
        **subqueries: "Query",
    ) -> "Query[Self]":  # type: ignore
        from ..common.query import Query, QueryType, to_subqueries
        from ..common.query import join as to_join

        query = Query(
            type=QueryType.NODE,
            relation=RELATION_REF_BY_CLASS[cls],
            name=name or cls.metatype.camel_name,
            join=to_join(join) if join is not None else None,
            where=where,
            subqueries=to_subqueries(subqueries),
            # limit=1?
        )
        return query  # type: ignore

    @classmethod
    def search(
        cls: type["Self"],
        where: Optional["Condition"] = None,
        *,
        name: str | None = None,
        join: Optional["JoinIn"] = None,
        having: Optional["Condition"] = None,
        sort: Optional[list["Sort"]] = None,
        group_by: Optional[list["ExpressionIn"]] = None,
        limit: Optional[int] = None,
        offset: Optional[int] = None,
        **subqueries: "Query",
    ) -> "Query[Self]":  # type: ignore
        from ..common.query import Query, QueryType, to_subqueries
        from ..common.query import expression as to_expression
        from ..common.query import join as to_join

        query = Query(
            type=QueryType.NODE if not group_by else QueryType.GROUPED_NODE,
            relation=RELATION_REF_BY_CLASS[cls],
            name=name or cls.metatype.camel_name,
            join=to_join(join) if join is not None else None,
            where=where,
            having=having,
            group_by=[to_expression(expr) for expr in group_by or ()],
            sort=sort or [],
            limit=limit,
            offset=offset,
            subqueries=to_subqueries(subqueries),
        )
        return query  # type: ignore

    @classmethod
    def scalar(
        cls: type["Self"],
        type: "AggregationType",
        *,
        name: str | None = None,
        join: Optional["JoinIn"] = None,
        expression: "ExpressionIn | None" = None,
        where: Optional["Condition"] = None,
        sort: Optional[list["Sort"]] = None,
        group_by: Optional[list["ExpressionIn"]] = None,
        having: Optional["Condition"] = None,
    ) -> "Query[Self]":  # type: ignore
        from ..common.query import Aggregation, Query, QueryType
        from ..common.query import expression as to_expression
        from ..common.query import join as to_join

        query = Query(
            type=QueryType.SCALAR if not group_by else QueryType.GROUPED_SCALAR,
            relation=RELATION_REF_BY_CLASS[cls],
            name=name or cls.metatype.camel_name,
            join=to_join(join) if join is not None else None,
            where=where,
            having=having,
            group_by=[to_expression(expr) for expr in group_by or ()],
            aggregation=Aggregation(
                type=type, expression=to_expression(expression) if expression else None
            ),
            sort=sort or [],
        )
        return query  # type: ignore

    @classmethod
    def exists(
        cls: type["Self"],
        where: Optional["Condition"] = None,
        *,
        name: str | None = None,
        join: Optional["JoinIn"] = None,
    ) -> "Query[Self]":  # type: ignore
        from ..common.query import Aggregation, AggregationType, Query, QueryType
        from ..common.query import join as to_join

        query = Query(
            type=QueryType.SCALAR,
            relation=RELATION_REF_BY_CLASS[cls],
            name=name or cls.metatype.camel_name,
            join=to_join(join) if join is not None else None,
            where=where,
            aggregation=Aggregation(type=AggregationType.EXISTS),
        )
        return query  # type: ignore

    @classmethod
    def count(
        cls: type["Self"],
        where: Optional["Condition"] = None,
        *,
        name: str | None = None,
        join: Optional["JoinIn"] = None,
        sort: Optional[list["Sort"]] = None,
        group_by: Optional[list["ExpressionIn"]] = None,
        having: Optional["Condition"] = None,
    ) -> "Query[Self]":  # type: ignore
        from ..common.query import Aggregation, AggregationType, Query, QueryType
        from ..common.query import expression as to_expression
        from ..common.query import join as to_join

        query = Query(
            type=QueryType.SCALAR if not group_by else QueryType.GROUPED_SCALAR,
            relation=RELATION_REF_BY_CLASS[cls],
            name=name or cls.metatype.camel_name,
            join=to_join(join) if join is not None else None,
            where=where,
            having=having,
            group_by=[to_expression(expr) for expr in group_by or ()],
            aggregation=Aggregation(type=AggregationType.COUNT),
            sort=sort or [],
        )
        return query  # type: ignore

    @classmethod
    def min(
        cls: type["Self"],
        expression: "ExpressionIn",
        *,
        name: str | None = None,
        join: Optional["JoinIn"] = None,
        where: Optional["Condition"] = None,
        having: Optional["Condition"] = None,
        group_by: Optional[list["ExpressionIn"]] = None,
        sort: Optional[list["Sort"]] = None,
    ) -> "Query[Self]":  # type: ignore
        from ..common.query import Aggregation, AggregationType, Query, QueryType
        from ..common.query import expression as to_expression
        from ..common.query import join as to_join

        query = Query(
            type=QueryType.SCALAR if not group_by else QueryType.GROUPED_SCALAR,
            relation=RELATION_REF_BY_CLASS[cls],
            name=name or cls.metatype.camel_name,
            join=to_join(join) if join is not None else None,
            where=where,
            having=having,
            group_by=[to_expression(expr) for expr in group_by or ()],
            aggregation=Aggregation(type=AggregationType.MIN, expression=to_expression(expression)),
            sort=sort or [],
        )
        return query  # type: ignore

    @classmethod
    def max(
        cls: type["Self"],
        expression: "ExpressionIn",
        *,
        name: str | None = None,
        join: Optional["JoinIn"] = None,
        where: Optional["Condition"] = None,
        having: Optional["Condition"] = None,
        group_by: Optional[list["ExpressionIn"]] = None,
        sort: Optional[list["Sort"]] = None,
    ) -> "Query[Self]":  # type: ignore
        from ..common.query import Aggregation, AggregationType, Query, QueryType
        from ..common.query import expression as to_expression
        from ..common.query import join as to_join

        query = Query(
            type=QueryType.SCALAR if not group_by else QueryType.GROUPED_SCALAR,
            relation=RELATION_REF_BY_CLASS[cls],
            name=name or cls.metatype.camel_name,
            join=to_join(join) if join is not None else None,
            where=where,
            having=having,
            group_by=[to_expression(expr) for expr in group_by or ()],
            aggregation=Aggregation(type=AggregationType.MAX, expression=to_expression(expression)),
            sort=sort or [],
        )
        return query  # type: ignore

    @classmethod
    def average(
        cls: type["Self"],
        expression: "ExpressionIn",
        *,
        name: str | None = None,
        join: Optional["JoinIn"] = None,
        where: Optional["Condition"] = None,
        having: Optional["Condition"] = None,
        group_by: Optional[list["ExpressionIn"]] = None,
        sort: Optional[list["Sort"]] = None,
    ) -> "Query[Self]":  # type: ignore
        from ..common.query import Aggregation, AggregationType, Query, QueryType
        from ..common.query import expression as to_expression
        from ..common.query import join as to_join

        query = Query(
            type=QueryType.SCALAR if not group_by else QueryType.GROUPED_SCALAR,
            relation=RELATION_REF_BY_CLASS[cls],
            name=name or cls.metatype.camel_name,
            join=to_join(join) if join is not None else None,
            where=where,
            having=having,
            group_by=[to_expression(expr) for expr in group_by or ()],
            aggregation=Aggregation(
                type=AggregationType.AVERAGE, expression=to_expression(expression)
            ),
            sort=sort or [],
        )
        return query  # type: ignore


@trait_(None)
class Trait(Node if TYPE_CHECKING else NodeBase):
    """A Node trait."""

    metatype: ClassVar[TraitType]
    info: ClassVar["TraitInfo"]

    # 1-9: node identity
    #  (repeat common Node properties here so trait RelationReferences can reference them,
    #   since Trait doesn't inherit from Node directly for circularity reasons)
    id: UUID = property_(2, is_managed=True, is_eq=False, can_write="system")
    parent: Optional["Node"] = property_parent_(node_is_customizable=True)
    if TYPE_CHECKING:
        parent_type: NodeType | None = None
        parent_id: Optional[UUID] = None
        parent_ptr: Optional[NodeReference] = None

    __is_node__: ClassVar[bool] = True
    __is_trait__: ClassVar[bool] = True
    __traits__: ClassVar[tuple[TraitType, ...]] = ()
    __indexes__: ClassVar[tuple[IndexIn, ...]] = ()


#
# Has* Traits (has specific properties)
#


@trait_(TraitType.HAS_NAME)
class HasName(Trait):
    """A Node with a plain name."""

    name: str = property_(31, is_repr=True)


@trait_(TraitType.HAS_SLUG)
class HasSlug(Trait):
    """A Node with a slug."""

    slug: str | None = property_(33, is_repr=True)


@trait_(TraitType.HAS_ICON)
class HasIcon(Trait):
    """A Node with an icon."""

    icon: Optional["Icon"] = property_(34)


#
# Is* Traits (ascribes some behavior)
#


@trait_(TraitType.TRACKED)
class IsTracked(Trait):
    """A Node that is "tracked" on create/update."""

    created_at: datetime = property_(10, is_managed=True, is_eq=False, can_write="system")
    created_by: Optional["IsSubject"] = property_(
        11,
        default=None,
        is_managed=True,
        is_eq=False,
        node_space_from="self",
        node_is_customizable=False,
        can_write="system",
    )
    updated_at: datetime = property_(12, is_managed=True, is_eq=False, can_write="system")
    updated_by: Optional["IsSubject"] = property_(
        13,
        default=None,
        is_managed=True,
        is_eq=False,
        node_space_from="self",
        node_is_customizable=False,
        can_write="system",
    )
    if TYPE_CHECKING:
        created_by_id: Optional[UUID] = None
        created_by_type: NodeType | None = None
        created_by_ptr: Optional[NodeReference] = None
        updated_by_id: Optional[UUID] = None
        updated_by_type: NodeType | None = None
        updated_by_ptr: Optional[NodeReference] = None


@trait_(TraitType.VISUAL)
class IsVisual(IsTracked):
    """A Node that is a visual in some sense (views, styles, drawings, ...)."""

    pass


@trait_(TraitType.FROZEN, pretend_frozen=True)
class IsFrozen(Trait):
    """
    A Node that is frozen (read-only).
    TODO :Cleanup: Nodes don't set 'real' frozen=True (like StructFrozen) :PretendFrozen
     (because that would require two separate inheritance chains for NodeMutable and NodeFrozen,
      which would have to include copies of every relevant trait and .. ughh no)
    """

    pass


@trait_(TraitType.ARCHIVABLE)
class IsArchivable(Trait):
    """A Node that can be archived."""

    archived_at: Optional[datetime] = property_(14, is_managed=True, is_eq=False)

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


@trait_(TraitType.DELETABLE)
class IsDeletable(Trait):
    """A Node that can be deleted."""

    deleted_at: Optional[datetime] = property_(15, is_managed=True, is_eq=False)

    def delete(self):
        """Delete this Node."""
        assert not self.deleted_at, f"{self!r} is already deleted"
        self._session.delete(self)

    def restore(self):
        """Restore this deleted Node from the trash."""
        assert self.deleted_at, f"{self!r} is not deleted"
        self._session.restore(self)


@trait_(TraitType.TEMPLATABLE)
class IsTemplatable(Trait):
    """A Node that can become a template (we can create Nodes derived from 'templates')."""

    template: Optional["Node"] = property_(16, edge_type=EdgeType.TEMPLATE)
    if TYPE_CHECKING:
        template_id: Optional[UUID] = None
        template_ptr: Optional["NodeReference"] = None
    # instance_of/overlay_of?

    def instance(
        self,
        recursive: bool = True,
        detach: bool = True,
        _map: bool | dict[UUID, "Node"] = True,
        _is_nested: bool = False,
        **kwargs: Any,
    ) -> Self:
        """
        Creates a new instance of this Node.
        Similar to Node.clone, but sets the original Nodes as the template source.
        """

        raise NotImplementedError


@trait_(TraitType.CUSTOM_NODE_DEFINITION)
class IsCustomNodeDefinition(Trait):
    """A Node that defines a Custom Node type."""

    pass


@trait_(TraitType.CUSTOM_NODE)
class IsCustomNode(Trait):
    """A Node that is asome Custom Node."""

    definition: "IsCustomNodeDefinition" = property_(17)
    if TYPE_CHECKING:
        definition_id: Optional[UUID] = None
        definition_ptr: Optional[NodeReference] = None


@trait_(TraitType.EXTENSIBLE)
class IsExtensible(Trait):
    """A Node that can be extended with custom Values (one Value per Field)."""

    value: dict[UUID, "Value"] = property_(18)


@trait_(TraitType.ORDERED)
class IsOrdered(Trait):
    """A Node that can be ordered."""

    order_key: str = property_(19, is_eq=False, default=INTEGER_ZERO)


@trait_(TraitType.REACTABLE)
class IsReactable(Trait):
    """A Node that can be reacted to."""

    pass


@trait_(TraitType.STARABLE)
class IsStarable(Trait):
    """A Node that can be starred."""

    pass


@trait_(TraitType.FOLLOWABLE)
class IsFollowable(Trait):
    """A Node that can be followed."""

    pass


@trait_(TraitType.SOURCEABLE)
class IsSourceable(IsOrdered):
    """A Node that can be sourced from / defined by a Script."""

    source: Optional["Script"] = property_(210)
    # token_range, ...


@trait_(TraitType.SCRIPTABLE)
class IsScriptable(Trait):
    """A Node that can be scripted."""

    script: Optional["Script"] = property_(200)


@trait_(TraitType.RUNNABLE)
class IsRunnable(Trait):
    """A Node that can be run (at runtime, in a Run)."""

    pass


@trait_(TraitType.ACTIONABLE)
class IsActionable(Trait):
    """A Node that can define an Action."""

    pass


@trait_(TraitType.OWNABLE)
class IsOwnable(Trait):
    """A Node that can be owned by another Node."""

    owned_by: Optional["IsSubject"] = property_(22, is_repr=True)
    if TYPE_CHECKING:
        owned_by_id: Optional[UUID] = None
        owned_by_type: Optional[NodeType] = None
        owned_by_ptr: Optional[NodeReference] = None


@trait_(TraitType.JOINABLE)
class IsJoinable(Trait):
    """A Node that can be joined by a Subject."""

    pass


@trait_(TraitType.SUBJECT)
class IsSubject(Trait):
    """A Node that can be a Subject."""

    pass


@trait_(TraitType.TAGGABLE)
class IsTaggable(Trait):
    """A Node that can be tagged."""

    pass


#
# Like* Traits (mimic for a specific builtin node type)
#


@trait_(TraitType.MEMBERSHIP)
class LikeMembership(Trait):
    """A Node that represents a Membership."""

    member: "IsSubject" = property_(40)


@trait_(TraitType.INVITE)
class LikeInvite(Trait):
    """A Node that represents an Invite."""

    member: "IsSubject" = property_(40)


@trait_(TraitType.TAG)
class LikeTag(Trait):
    """A Node that represents a Tag."""

    pass


@trait_(TraitType.FOLLOW)
class LikeFollow(Trait):
    """A Node that represents a Follow."""

    pass


#
# Bare Traits (main type/location)
#


@trait_(TraitType.GLOBAL)
class Global(Trait):
    """A Node that is global."""

    pass


@trait_(TraitType.SPATIAL)
class Spatial(Trait):
    """A Node in a Space."""

    parent: Optional["Space"] = property_parent_(node_is_customizable=False)
    space: "Space | None" = property_ancestor_(5, is_required=True)
    if TYPE_CHECKING:
        space_ptr: Optional[NodeReference] = None


@trait_(TraitType.ENTITY)
class Entity(IsTracked):
    """An Entity is a Node in primary relational storage (OLTP)."""

    # nocheckin: support Entity variants/branching (use id+variant as primary key?)
    #  (or maybe for some subset of Entities?)
    #  (idea: 'materialized' base frames and Edit streams so we don't need a copy for each edit?)
    #  (or maybe support it for all Nodes to support staging Changes but only expose it for Entities?)
    #  (IsBranch trait and Node.variant_id for Forks/Branches/Variants/Templates/.....?)
    pass


@trait_(TraitType.PARTICLE)
class Particle(IsTracked):
    """A Particle is a Node in primary document storage (OLTP, high volume)."""

    pass


@trait_(TraitType.ANALYTIC)
class Analytic(IsTracked):
    """An Analytic is stored in primary or secondary warehouse storage (OLAP, bulk)."""

    pass


@trait_(TraitType.INDEXED)
class Indexed(IsTracked):
    """A Node that is indexed in secondary search storage (OLTP)."""

    pass


@trait_(TraitType.RESOURCE)
class Resource(Entity):
    """
    A Resource represents an external asset, and may be managed by some provisioner.
    """

    parent: Optional["Folder"] = property_parent_(node_is_customizable=False)
    status: ResourceStatus = property_(40, default=ResourceStatus.PENDING)
    target_status: Optional[datetime] = property_(41)
    failed_at: Optional[datetime] = property_(47, can_write="system")
    failed_attempts: int = property_(48, default=0, can_write="system")


@trait_(TraitType.METRIC)
class Metric(Entity, IsSourceable, IsCustomNodeDefinition):
    """A Node that represents a Metric."""

    pass


@trait_(TraitType.MEASUREMENT)
class Measurement(IsCustomNode, Analytic):
    """A Node that represents a Measurement."""

    definition: "Metric" = property_(17)


@trait_(TraitType.EVENT, pretend_frozen=True)
class Event(Particle, Indexed, Analytic, IsFrozen):
    """A Node that represents an Event."""

    pass
