from collections.abc import Collection
from dataclasses import dataclass
from datetime import datetime, timedelta
from typing import (
    TYPE_CHECKING,
    Any,
    ClassVar,
    Optional,
    Self,
    Union,
    assert_never,
    cast,
    dataclass_transform,
)

from fastuuid import UUID

from bench.language.registry import (
    NODE_CLASS_BY_TRAIT,
    NODE_TRAIT_BY_CLASS,
    NODE_TYPES_BY_TRAIT,
    RELATION_REF_BY_CLASS,
)
from bench.pb2 import AnyObjectData
from bench.utils.fractional import INTEGER_ZERO
from bench.utils.tenacity import RetryOptions

from .const import (
    EdgeType,
    EnvironmentType,
    NodeType,
    ProcessStatus,
    ResourceStatus,
    TraitType,
)
from .object import BuiltinObjectMutable, _process_object_cls
from .property import (
    _PROPERTY_SPECIFIERS,
    property_,
    property_ancestor_,
    property_parent_,
    property_runtime_,
)

if TYPE_CHECKING:
    from bench.language import (
        AggregationType,
        Bench,
        Block,
        Condition,
        Error,
        Expression,
        ExpressionIn,
        Icon,
        Interruption,
        ModelDeveloper,
        ModelProvider,
        Node,
        NodeReference,
        Package,
        Page,
        Query,
        Script,
        Sort,
        TextLine,
        TraitInfo,
        Value,
    )

    from ..definition.query import JoinIn

# pyright: reportIncompatibleVariableOverride=false

#
# Trait/Node base
#


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
            node_types.update(NODE_TYPES_BY_TRAIT.get(typ, ()))
        else:
            assert_never(typ)
    return tuple(node_types)


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def trait_(
    node_trait: TraitType | None,
):
    """Register a class as a node trait."""

    def decorate(cls: type["NodeBase"]) -> type["NodeBase"]:
        cls, _ = _process_object_cls(
            cls=cls,
            object_type=None,
            is_concrete=False,
            is_node=True,
        )
        if node_trait is not None:
            cls.metatype = node_trait
        if node_trait is not None:
            NODE_CLASS_BY_TRAIT[node_trait] = cls
            NODE_TRAIT_BY_CLASS[cast(type["Trait"], cls)] = node_trait
        return cls

    return decorate


@trait_(node_trait=None)  # type: ignore
class NodeBase[NodeDataT: AnyObjectData](BuiltinObjectMutable[NodeDataT]):
    """A base class for all Nodes."""

    metatype: ClassVar[TraitType | NodeType]
    __is_node__: ClassVar[bool] = True

    @classmethod
    def get(
        cls: type["Self"],
        where: Optional["Condition"] = None,
        *,
        name: str | None = None,
        join: Optional["JoinIn"] = None,
        **subqueries: "Query",
    ) -> "Query[Self]":  # type: ignore
        from ..definition.query import Query, QueryType, to_subqueries
        from ..definition.query import join as to_join

        query = Query(
            type=QueryType.NODE,
            relation=RELATION_REF_BY_CLASS[cls],
            name=name or cls.metatype.bench_name,
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
        group_by: Optional[list["Expression"]] = None,
        limit: Optional[int] = None,
        offset: Optional[int] = None,
        **subqueries: "Query",
    ) -> "Query[Self]":  # type: ignore
        from ..definition.query import Query, QueryType, to_subqueries
        from ..definition.query import join as to_join

        query = Query(
            type=QueryType.NODE if group_by is None else QueryType.GROUPED_NODE,
            relation=RELATION_REF_BY_CLASS[cls],
            name=name or cls.metatype.bench_name,
            join=to_join(join) if join is not None else None,
            where=where,
            having=having,
            group_by=group_by or [],
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
        group_by: Optional[list["Expression"]] = None,
        sort: Optional[list["Sort"]] = None,
    ) -> "Query[Self]":  # type: ignore
        from ..definition.query import Aggregation, Query, QueryType
        from ..definition.query import expression as to_expression
        from ..definition.query import join as to_join

        query = Query(
            type=QueryType.SCALAR if group_by is None else QueryType.GROUPED_SCALAR,
            relation=RELATION_REF_BY_CLASS[cls],
            name=name or cls.metatype.bench_name,
            join=to_join(join) if join is not None else None,
            where=where,
            group_by=group_by or [],
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
        from ..definition.query import Aggregation, AggregationType, Query, QueryType
        from ..definition.query import join as to_join

        query = Query(
            type=QueryType.SCALAR,
            relation=RELATION_REF_BY_CLASS[cls],
            name=name or cls.metatype.bench_name,
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
        group_by: Optional[list["ExpressionIn"]] = None,
        having: Optional["Condition"] = None,
    ) -> "Query[Self]":  # type: ignore
        from ..definition.query import Aggregation, AggregationType, Query, QueryType
        from ..definition.query import expression as to_expression
        from ..definition.query import join as to_join

        query = Query(
            type=QueryType.SCALAR if group_by is None else QueryType.GROUPED_SCALAR,
            relation=RELATION_REF_BY_CLASS[cls],
            name=name or cls.metatype.bench_name,
            join=to_join(join) if join is not None else None,
            where=where,
            having=having,
            aggregation=Aggregation(type=AggregationType.COUNT),
            group_by=[to_expression(expr) for expr in group_by or ()],
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
    ) -> "Query[Self]":  # type: ignore
        from ..definition.query import Aggregation, AggregationType, Query, QueryType
        from ..definition.query import expression as to_expression
        from ..definition.query import join as to_join

        query = Query(
            type=QueryType.SCALAR if group_by is None else QueryType.GROUPED_SCALAR,
            relation=RELATION_REF_BY_CLASS[cls],
            name=name or cls.metatype.bench_name,
            join=to_join(join) if join is not None else None,
            where=where,
            having=having,
            group_by=[to_expression(expr) for expr in group_by or ()],
            aggregation=Aggregation(type=AggregationType.MIN, expression=to_expression(expression)),
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
    ) -> "Query[Self]":  # type: ignore
        from ..definition.query import Aggregation, AggregationType, Query, QueryType
        from ..definition.query import expression as to_expression
        from ..definition.query import join as to_join

        query = Query(
            type=QueryType.SCALAR if group_by is None else QueryType.GROUPED_SCALAR,
            relation=RELATION_REF_BY_CLASS[cls],
            name=name or cls.metatype.bench_name,
            join=to_join(join) if join is not None else None,
            where=where,
            having=having,
            group_by=[to_expression(expr) for expr in group_by or ()],
            aggregation=Aggregation(type=AggregationType.MAX, expression=to_expression(expression)),
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
    ) -> "Query[Self]":  # type: ignore
        from ..definition.query import Aggregation, AggregationType, Query, QueryType
        from ..definition.query import expression as to_expression
        from ..definition.query import join as to_join

        query = Query(
            type=QueryType.SCALAR if group_by is None else QueryType.GROUPED_SCALAR,
            relation=RELATION_REF_BY_CLASS[cls],
            name=name or cls.metatype.bench_name,
            join=to_join(join) if join is not None else None,
            where=where,
            having=having,
            group_by=[to_expression(expr) for expr in group_by or ()],
            aggregation=Aggregation(
                type=AggregationType.AVERAGE, expression=to_expression(expression)
            ),
        )
        return query  # type: ignore


@trait_(None)
class Trait(Node if TYPE_CHECKING else NodeBase):
    """A Node trait."""

    metatype: ClassVar[TraitType]
    info: ClassVar["TraitInfo"]

    __is_node__: ClassVar[bool] = True
    __is_trait__: ClassVar[bool] = True
    __traits__: ClassVar[tuple[TraitType, ...]] = ()
    __indexes__: ClassVar[tuple[IndexIn, ...]] = ()


#
# Traits
#


@trait_(TraitType.HAS_NAME)
class HasName(Trait):
    """A Node with a plain name."""

    name: str = property_(31)


@trait_(TraitType.HAS_TITLE)
class HasTitle(Trait):
    """A Node with a rich title."""

    title: Optional["TextLine"] = property_(32)


@trait_(TraitType.HAS_SLUG)
class HasSlug(Trait):
    """A Node with a slug."""

    slug: str | None = property_(33, is_repr=True)


@trait_(TraitType.HAS_ICON)
class HasIcon(Trait):
    """A Node with an icon."""

    icon: Optional["Icon"] = property_(34)


@trait_(TraitType.HAS_ENVIRONMENT)
class HasEnvironment(Trait):
    """A Node that can be in different environments."""

    environment_type: EnvironmentType = property_(
        17, is_eq=False, default=EnvironmentType.PRODUCTION
    )
    # environment: "Environment | None", ...


@trait_(TraitType.GLOBAL)
class IsGlobal(Trait):
    """A Node that is global."""

    pass


@trait_(TraitType.ORDERED)
class IsOrdered(Trait):
    """A Node that can be ordered."""

    order_key: str | None = property_(23, is_eq=False, default=INTEGER_ZERO)


@trait_(TraitType.FROZEN)
class IsFrozen(Trait):
    """A Node that is frozen."""

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


@trait_(TraitType.EXTENSIBLE)
class IsExtensible(Trait):
    """A Node that can be extended with custom Values (one Value per Field)."""

    value: dict[UUID, "Value"] = property_(24)


@trait_(TraitType.IN_BENCH)
class IsInBench(Trait):
    """A Node inside a Bench."""

    bench: "Bench | None" = property_ancestor_(6, is_required=True)
    if TYPE_CHECKING:
        bench_ptr: Optional[NodeReference] = None
    _bench_ptr: Optional["NodeReference"] = property_runtime_(default=None)  # :CachedAncestors


@trait_(TraitType.IN_PACKAGE)
class IsInPackage(IsInBench):
    """A Node in a Package."""

    package: "Package | None" = property_ancestor_(7, is_required=False)
    if TYPE_CHECKING:
        package_ptr: Optional[NodeReference] = None
    _package_ptr: Optional["NodeReference"] = property_runtime_(default=None)  # :CachedAncestors


@trait_(TraitType.BLOCKABLE)
class IsBlockable(IsOrdered, IsInPackage):
    """A Node that can (but may not be) be inline on a Page as a Block."""

    parent: Union["Page", None] = property_parent_()
    block: "Block | None" = property_(
        35,
        node_bench_from="self",
        description="The Block where this Node is 'defined'.",
    )
    if TYPE_CHECKING:
        block_id: Optional[UUID] = None
        block_ptr: Optional[NodeReference] = None

    def wrap_in_block(self) -> "Block":
        """Wrap this Node in a *new* Block."""
        from bench.language import Block

        return Block.wrap(self)


@trait_(TraitType.COMPUTABLE)
class IsComputable(Trait):
    """A Node that can have computation applied to it somehow."""

    # model
    # NOTE :Incomplete: IsComputable.model_id should probably be plural (model_ids?)
    model_developer: Optional["ModelDeveloper"] = property_(100)
    model_provider: Optional["ModelProvider"] = property_(101)
    model_id: Optional[str] = property_(102)
    model_name: Optional[str] = property_(103)
    # compute/cost/effort/budget/'juice'...?


@trait_(TraitType.SCRIPTABLE)
class IsScriptable(Trait):
    """A Node that can be scripted."""

    script: Optional["Script"] = property_(104)


@trait_(TraitType.RUNNABLE)
class IsRunnable(IsComputable):
    """A Node that can be run (at runtime in a Run)."""

    # control
    max_attempts: Optional[int] = property_(110)
    retry_interval: Optional[timedelta] = property_(111)
    backoff: Optional[float] = property_(112)

    def to_retry(self) -> RetryOptions:
        """Turns the options into our RetryOptions."""
        retry_interval = self.retry_interval.total_seconds() if self.retry_interval else 1
        return RetryOptions(
            max_attempts=self.max_attempts or 1,
            retry_interval=retry_interval,
            backoff=self.backoff or 22,
            max_retry_interval=max(30, retry_interval * 5),
        )


@trait_(TraitType.PROCESSABLE)
class IsProcessable(Trait):
    """A Node that can be processed somehow."""

    status: ProcessStatus = property_(80, default=ProcessStatus.CREATED, is_repr=True)
    duration: Optional[timedelta] = property_(
        81,
        default=None,
        description="Duration from first attempt start to last attempt termination.",
        is_repr=True,
    )
    error: Optional["Error"] = property_(82, is_repr=True)
    interruption: Optional["Interruption"] = property_(
        83,
        node_bench_from="self",
        description="The latest Interruption concerning the Node.",
        is_repr=True,
    )
    scheduled_at: Optional[datetime] = property_(
        85, description="When the Node is scheduled to start."
    )
    started_at: Optional[datetime] = property_(
        86, description="When the Node first started.", is_repr=True
    )
    active_at: Optional[datetime] = property_(87, description="When the Node was last active.")
    interrupted_at: Optional[datetime] = property_(88, description="When the Node was interrupted.")
    terminated_at: Optional[datetime] = property_(
        89, description="When the Node was last terminated."
    )
    requested_stop_at: Optional[datetime] = property_(
        90, description="When the Node was requested to stop."
    )
    requested_pause_at: Optional[datetime] = property_(
        91, description="When the Node was requested to pause."
    )
    requested_resume_at: Optional[datetime] = property_(
        92, description="When the Node was requested to resume."
    )
    if TYPE_CHECKING:
        interruption_ptr: Optional[NodeReference] = None
        interruption_id: Optional[UUID] = None

    def touch(self) -> None:
        """'Touch' the Node to update the active_at timestamp."""
        self.active_at = self._session.oracle.utc()

    @property
    def should_stop(self) -> bool:
        return not (self.status.is_terminal) and (self.requested_stop_at is not None)

    @property
    def should_pause(self) -> bool:
        return not (self.status.is_terminal or self.requested_stop_at is not None) and (
            self.requested_pause_at is not None
            and (
                self.requested_resume_at is None
                or self.requested_pause_at > self.requested_resume_at
            )
        )

    @property
    def should_resume(self) -> bool:
        return not (self.status.is_terminal or self.requested_stop_at is not None) and (
            self.requested_resume_at is not None
            and (
                self.requested_pause_at is None
                or self.requested_pause_at < self.requested_resume_at
            )
            and (self.interrupted_at is None or self.interrupted_at < self.requested_resume_at)
        )


@trait_(TraitType.INSTRUMENT)
class IsInstrument(Trait):
    """A Node that represents an Instrument."""

    pass


@trait_(TraitType.MEASUREMENT)
class IsMeasurement(Trait):
    """A Node that represents a Measurement."""

    pass


@trait_(TraitType.LOG)
class IsLog(Trait):
    """A Node that represents a Log."""

    pass


@trait_(TraitType.OWNABLE)
class IsOwnable(Trait):
    """A Node that can be owned by another Node."""

    owned_by: Optional["IsSubject"] = property_(19, node_bench_from="self")
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


@trait_(TraitType.MEMBERSHIP)
class IsMembership(Trait):
    """A Node that represents a Membership."""

    member: "IsSubject" = property_(40)


@trait_(TraitType.INVITE)
class IsInvite(Trait):
    """A Node that represents an Invite."""

    member: "IsSubject" = property_(40)


@trait_(TraitType.ROLE)
class IsRole(Trait):
    """A Node that represents a Role."""

    pass


@trait_(TraitType.RESOURCE)
class IsResource(HasEnvironment, IsOwnable, HasName):
    """
    A Resource in a Bench, typically representing some external object.
    """

    pass


@trait_(TraitType.PROVISIONABLE)
class IsProvisionable(IsResource):
    """
    A Resource that can be provisioned.
    """

    # status
    status: ResourceStatus = property_(40, default=ResourceStatus.PENDING)
    requested_activate_at: Optional[datetime] = property_(41)
    requested_deactivate_at: Optional[datetime] = property_(42)
    requested_reset_at: Optional[datetime] = property_(43)
    requested_suspend_at: Optional[datetime] = property_(44)
    requested_decommission_at: Optional[datetime] = property_(45)
    active_at: Optional[datetime] = property_(46, can_write="system")
    failed_at: Optional[datetime] = property_(47, can_write="system")
    failed_attempts: int = property_(48, default=0, can_write="system")
    if TYPE_CHECKING:
        scaler_ptr: Optional[NodeReference] = None
        scaler_id: Optional[UUID] = None

    @property
    def should_retry(self) -> bool:
        """Whether this Resource should be retried."""
        return self.failed_at is None or (
            self.requested_reset_at is not None and self.requested_reset_at > self.failed_at
        )

    @property
    def should_reset(self) -> bool:
        """Whether this Resource should be reset."""
        return (
            self.status.is_extant
            and self.requested_reset_at is not None
            and self.requested_activate_at is not None
            and self.requested_reset_at > self.requested_activate_at
        )

    @property
    def target_status(self) -> ResourceStatus:
        """The implied target status of this Resource."""
        if self.requested_decommission_at is not None:
            return ResourceStatus.OFFLINE
        elif self.requested_suspend_at is not None and not (
            self.requested_activate_at is not None
            and self.requested_activate_at > self.requested_suspend_at
        ):
            return ResourceStatus.SLEEPING
        elif self.requested_deactivate_at is not None and not (
            self.requested_activate_at is not None
            and self.requested_activate_at > self.requested_deactivate_at
        ):
            return ResourceStatus.UNAVAILABLE
        else:
            return ResourceStatus.AVAILABLE

    def provision(self) -> None:
        """Request to provision this Resource."""
        self.requested_activate_at = self._session.oracle.utc()

    def decommission(self) -> None:
        """Request to decommission this Resource."""
        self.requested_decommission_at = self._session.oracle.utc()

    def update_status(self, status: ResourceStatus) -> None:
        """Set the actual current status of this Resource."""
        self.status = status
        if status == ResourceStatus.FAILED or status == ResourceStatus.RETRYING:
            self.failed_at = self._session.oracle.utc()
            self.failed_attempts += 1
        elif status.is_extant:
            self.active_at = self._session.oracle.utc()
            self.failed_attempts = 0
