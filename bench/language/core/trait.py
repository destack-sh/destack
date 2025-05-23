import abc
from datetime import datetime, timedelta
from typing import (
    TYPE_CHECKING,
    Any,
    Collection,
    NamedTuple,
    Optional,
    Self,
    Union,
    assert_never,
    cast,
    dataclass_transform,
)

from fastuuid import UUID

from bench.language.registry import NODE_CLASS_BY_TRAIT, NODE_TYPES_BY_TRAIT
from bench.pb2 import AnyNodeData, NodeReferenceData
from bench.utils.fractional import INTEGER_ZERO
from bench.utils.string import Casing, to_casing
from bench.utils.tenacity import RetryOptions

from .const import (
    REGION,
    NodeMode,
    NodeReferenceKind,
    NodeType,
    ProcessStatus,
    Region,
    ResourceStatus,
    Trait,
)
from .graph import attach_node
from .object import BuiltinObject, _process_object_cls
from .property import (
    _PROPERTY_SPECIFIERS,
    property_,
    property_ancestor_,
    property_parent_,
    property_runtime_,
)
from .type import StringFormat

if TYPE_CHECKING:
    from bench.language import (
        Bench,
        Block,
        Claim,
        Error,
        Icon,
        Interruption,
        ModelDeveloper,
        ModelProvider,
        Node,
        NodeReference,
        Package,
        Page,
        TextLine,
        Thread,
        Value,
    )

# pyright: reportIncompatibleVariableOverride=false


class IndexIn(NamedTuple):
    """Index to be turned into a SQL Index."""

    columns: tuple[str, ...]
    cover: tuple[str, ...] = ()
    is_unique: bool = False
    condition: str | None = None
    name: str | None = None


def get_trait_by_name(name: str) -> Trait:
    """Get a trait by name."""
    if name.startswith("Is"):
        name = name[2:]
    name = to_casing(name, Casing.ALL_CAPS)
    trait = Trait.__members__.get(name)
    if trait is None:
        raise LookupError(f"unknown trait: {name}")
    return trait


def expand_node_types(types: Collection[NodeType | Trait]) -> tuple[NodeType, ...]:
    """Expand a collection of NodeTypes and Traits into a flat collection of NodeTypes."""
    node_types: set[NodeType] = set()
    for typ in types:
        if isinstance(typ, NodeType):
            node_types.add(typ)
        elif isinstance(typ, Trait):
            node_types.update(NODE_TYPES_BY_TRAIT[typ])
        else:
            assert_never(typ)
    return tuple(node_types)


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def trait_(
    node_trait: Trait,
):
    """Register a class as a node trait."""

    def decorate(cls: type["Node"]) -> type["Node"]:
        cls, _ = _process_object_cls(
            cls=cls,
            object_type=None,
            is_concrete=False,
            is_node=True,
        )
        NODE_CLASS_BY_TRAIT[node_trait] = cls
        return cls

    return decorate


@trait_(Trait.GLOBAL)
class IsGlobal(Node if TYPE_CHECKING else BuiltinObject):
    """A Node that is global."""

    pass


@trait_(Trait.LOCAL)
class IsLocal(Node if TYPE_CHECKING else BuiltinObject):
    """A Node that is local."""

    pass


@trait_(Trait.MODAL)
class IsModal(Node if TYPE_CHECKING else BuiltinObject):
    """A Node that can be in different modes."""

    mode: NodeMode = property_(20, is_managed=True, default=NodeMode.MAIN)


@trait_(Trait.NAMED)
class IsNamed(Node if TYPE_CHECKING else BuiltinObject):
    """A Node with a plain name."""

    name: str | None = property_(31, format=StringFormat.NAME)


@trait_(Trait.TITLED)
class IsTitled(Node if TYPE_CHECKING else BuiltinObject):
    """A Node with a rich title."""

    title: Optional["TextLine"] = property_(32)


@trait_(Trait.SLUG)
class IsSlug(Node if TYPE_CHECKING else BuiltinObject):
    """A Node with a slug."""

    slug: str | None = property_(33, format=StringFormat.SLUG)


@trait_(Trait.ICON)
class IsIcon(Node if TYPE_CHECKING else BuiltinObject):
    """A Node with an icon."""

    icon: Optional["Icon"] = property_(34)


@trait_(Trait.ORDERED)
class IsOrdered(Node if TYPE_CHECKING else BuiltinObject):
    """A Node that can be ordered."""

    order_key: str | None = property_(22, is_managed=True, default=INTEGER_ZERO)


@trait_(Trait.ARCHIVABLE)
class IsArchivable(Node if TYPE_CHECKING else BuiltinObject):
    """A Node that can be archived."""

    archived_at: Optional[datetime] = property_(14, is_managed=True)

    @property
    def is_archived(self) -> bool:
        return self.archived_at is not None

    def archive(self, _now: datetime | None = None):
        """Archive this Node."""
        assert not self.archived_at, f"{self!r} is already archived"
        self.active_session._archive(self, _now=_now)

    def unarchive(self, _now: datetime | None = None):
        """Unarchive this Node."""
        assert self.archived_at, f"{self!r} is not archived"
        self.active_session._unarchive(self, _now=_now)


@trait_(Trait.DELETABLE)
class IsDeletable(Node if TYPE_CHECKING else BuiltinObject):
    """A Node that can be deleted."""

    deleted_at: Optional[datetime] = property_(15, is_managed=True)

    def delete(self, _now: datetime | None = None):
        """Delete this Node."""
        assert not self.deleted_at, f"{self!r} is already deleted"
        self.active_session._delete(self, _now=_now)

    def restore(self, _now: datetime | None = None):
        """Restore this deleted Node from the trash."""
        assert self.deleted_at, f"{self!r} is not deleted"
        self.active_session._restore(self, _now=_now)


@trait_(Trait.TEMPLATABLE)
class IsTemplatable(Node if TYPE_CHECKING else BuiltinObject):
    """A Node that can be templated (we can create Nodes that are derived from 'templates')."""

    template: Optional["Node"] = property_(
        16,
        node_kind=NodeReferenceKind.NODE_TEMPLATE,
        node_exclude=("base_id",),
    )
    if TYPE_CHECKING:
        template_id: Optional[UUID] = None
        template_ptr: Optional["NodeReference"] = None

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

        from .node import Node

        assert isinstance(self, Node), f"{self!r} must be a Node"
        assert isinstance(self, IsModal), f"{self!r} must be modal"

        # instance self
        instance_kwargs = self._clone_kwargs(reset=True)
        if self.mode == NodeMode.TEMPLATE:
            instance_kwargs["mode"] = self.active_session.mode
        else:
            instance_kwargs["mode"] = self.mode
        instance_kwargs.update(kwargs)
        if isinstance(self, IsInstantiable):
            instance_kwargs["ck"] = self.ck
        instance_kwargs["template"] = self
        instance = self.__class__(**instance_kwargs)

        # remember new identities
        if _map is True:
            _map = {self.id: instance}
        elif _map is not False:
            _map[self.id] = instance

        # NOTE: we don't instance definitions/inline nodes together (that seems meaningless?)

        # instance children and append to self
        if recursive:
            for child_type in self.__child_types__:
                for child in self._graph.iter_descendants(self, child_type):
                    if isinstance(child, IsTemplatable):
                        # instance child
                        child_clone = child.instance(
                            recursive=True,
                            detach=True,
                            _map=_map,
                            _is_nested=True,
                        )
                        attach_node(child_clone, instance, instance._graph)  # re-attach
                    else:
                        # clone if not instantiable
                        child_clone = child.clone(
                            reset=True,
                            recursive=True,
                            detach=True,
                            _map=_map,
                            _is_nested=True,
                        )
                        attach_node(child_clone, instance, instance._graph)  # re-attach

        # map new identities
        if not _is_nested and type(_map) is dict:
            for node in _map.values():
                node.replace_references(
                    _map,
                    exclude=(NodeReferenceKind.NODE_PARENT, NodeReferenceKind.NODE_TEMPLATE),
                )

        # append to our parent to re-attach
        parent = self.parent
        if detach:
            instance.parent_ptr = None
        elif parent:
            parent.add_child(instance)
        return instance


@trait_(Trait.INSTANTIABLE)
class IsInstantiable(IsTemplatable):
    """A Node that can be instanced (we can create Nodes that are 'instances' of this Node)."""

    ck: UUID = property_(3, is_managed=True)  # type: ignore
    # nocheckin: proper templating/instancing (for views/Variants/overrides/branches/...)

    @property
    def is_instance(self) -> bool:
        return self.ck != cast("Node", self).id


@trait_(Trait.EXTENSIBLE)
class IsExtensible(Node if TYPE_CHECKING else BuiltinObject):
    """A Node that can be extended with Fields."""

    # nocheckin: IsExtensible.value
    value: "Value | None" = property_(21)


@trait_(Trait.BASED)
class IsBased(Node if TYPE_CHECKING else BuiltinObject):
    """
    A Node which may have a 'base' in another Node (e.g., its type definition).
    We almost always want to load them together, so it's useful to have this relationship.
    """

    @property
    @abc.abstractmethod
    def base(self) -> Optional["IsInBench"]: ...

    @property
    def base_id(self) -> Optional[UUID]:
        return self.base.id if self.base is not None else None

    @staticmethod
    @abc.abstractmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]: ...

    @staticmethod
    @abc.abstractmethod
    def get_base_from_partial(data: dict[str, Any]) -> Optional["IsInBench"]: ...


@trait_(Trait.IN_BENCH)
class IsInBench(Node if TYPE_CHECKING else BuiltinObject):
    """A Node inside a Bench."""

    bench: "Bench | None" = property_ancestor_(6, is_required=True)
    if TYPE_CHECKING:
        bench_id: Optional[UUID] = None
        bench_ptr: Optional[NodeReference] = None
    _bench: Optional["Bench"] = property_runtime_(default=None)  # :CachedAncestors

    @property
    def is_attached(self) -> bool:
        return self.parent_ptr is not None and self.bench is not None


@trait_(Trait.IN_PACKAGE)
class IsInPackage(IsInBench):
    """A Node in a Package."""

    package: "Package | None" = property_ancestor_(7, is_required=False)
    if TYPE_CHECKING:
        package_id: Optional[UUID] = None
        package_ptr: Optional[NodeReference] = None


@trait_(Trait.PAGEABLE)
class IsPageable(Node if TYPE_CHECKING else BuiltinObject):
    """A Node that can be in a Page."""

    pass


@trait_(Trait.BLOCKABLE)
class IsBlockable(IsOrdered, IsInPackage):
    """A Node that can (but may not be) be inline on a Page as a Block."""

    parent: Union["Page", None] = property_parent_()
    definition: "Block | None" = property_(
        35,
        node_bench_from="self",
        description="The Block where this Node is 'defined'.",
    )
    if TYPE_CHECKING:
        definition_id: Optional[UUID] = None
        definition_ck: Optional[UUID] = None
        definition_ptr: Optional[NodeReference] = None

    def wrap_in_block(self) -> "Block":
        """Wrap this Node in a *new* Block."""
        from bench.language import Block

        return Block.wrap(self)


@trait_(Trait.COMPUTABLE)
class IsComputable(Node if TYPE_CHECKING else BuiltinObject):
    """A Node that can have computation applied to it somehow."""

    # model
    # NOTE :Incomplete: IsComputable.model_id should probably be plural (model_ids?)
    model_developer: Optional["ModelDeveloper"] = property_(100)
    model_provider: Optional["ModelProvider"] = property_(101)
    model_id: Optional[str] = property_(102)
    model_name: Optional[str] = property_(103)
    # compute/cost/effort/budget/'juice'...?


@trait_(Trait.RUNNABLE)
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


@trait_(Trait.PROCESSABLE)
class IsProcessable(Node if TYPE_CHECKING else BuiltinObject):
    """A Node that can be processed somehow."""

    status: ProcessStatus = property_(80, default=ProcessStatus.CREATED)
    duration: Optional[timedelta] = property_(
        81,
        default=None,
        description="Duration from first attempt start to last attempt termination.",
    )
    error: Optional["Error"] = property_(82)
    interruption: Optional["Interruption"] = property_(
        83,
        node_bench_from="self",
        description="The latest Interruption concerning the Node.",
    )
    scheduled_at: Optional[datetime] = property_(
        85, description="When the Node is scheduled to start."
    )
    started_at: Optional[datetime] = property_(86, description="When the Node first started.")
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
        self.active_at = self.active_session.oracle.utc()

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


@trait_(Trait.OWNABLE)
class IsOwnable(Node if TYPE_CHECKING else BuiltinObject):
    """A Node that can be owned by another Node."""

    owned_by: Optional["IsSubject"] = property_(
        17,
        node_bench_from="self",
        node_exclude=("ck", "base_id"),
    )
    if TYPE_CHECKING:
        owned_by_id: Optional[UUID] = None
        owned_by_type: Optional[NodeType] = None
        owned_by_ptr: Optional[NodeReference] = None


@trait_(Trait.CLAIMABLE)
class IsClaimable(Node if TYPE_CHECKING else BuiltinObject):
    """A Node that can be claimed with a Claim."""

    claimed_by: Optional["Claim"] = property_(18, node_bench_from="self")
    if TYPE_CHECKING:
        claimed_by_id: Optional[UUID] = None
        claimed_by_ptr: Optional[NodeReference] = None


@trait_(Trait.JOINABLE)
class IsJoinable(Node if TYPE_CHECKING else BuiltinObject):
    """A Node that can be joined by a Subject."""

    pass


@trait_(Trait.SUBJECT)
class IsSubject(Node if TYPE_CHECKING else BuiltinObject):
    """A Node that can be a Subject."""

    pass


@trait_(Trait.MEMBERSHIP)
class IsMembership(Node if TYPE_CHECKING else BuiltinObject):
    """A Node that represents a Membership."""

    member: "IsSubject" = property_(40)


@trait_(Trait.INVITE)
class IsInvite(Node if TYPE_CHECKING else BuiltinObject):
    """A Node that represents an Invite."""

    pass


@trait_(Trait.REGIONAL)
class IsRegional(Node if TYPE_CHECKING else BuiltinObject):
    """A Node that is regional."""

    region: Region | None = property_(23, default=REGION)


@trait_(Trait.RESOURCE)
class IsResource(IsModal, IsInstantiable, IsOwnable, IsNamed, IsClaimable, IsBlockable):
    """
    A Resource in a Bench, typically representing some external object.
    """

    parent: Union["Package", "Page", "Thread", None] = property_parent_()


@trait_(Trait.PROVISIONABLE)
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
        self.requested_activate_at = self.active_session.oracle.utc()

    def decommission(self) -> None:
        """Request to decommission this Resource."""
        self.requested_decommission_at = self.active_session.oracle.utc()

    def update_status(self, status: ResourceStatus) -> None:
        """Set the actual current status of this Resource."""
        self.status = status
        if status == ResourceStatus.FAILED or status == ResourceStatus.RETRYING:
            self.failed_at = self.active_session.oracle.utc()
            self.failed_attempts += 1
        elif status.is_extant:
            self.active_at = self.active_session.oracle.utc()
            self.failed_attempts = 0

    async def wait_until_status(
        self, status: ResourceStatus, timeout: timedelta | None = None
    ) -> None:
        """Wait until this Resource reaches the given status."""
        await self.wait_until(lambda r: r.status == status, timeout=timeout)

    async def wait_until_ready(self, timeout: timedelta | None = None) -> None:
        """Wait until this Resource is ready."""
        await self.wait_until(lambda r: r.status == ResourceStatus.AVAILABLE, timeout=timeout)
