import abc
from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Any, Optional, Self, Union, cast, override

from fastuuid import UUID

from bench.language.registry import CHILD_NODE_TYPES
from bench.pb2 import AnyNodeData, NodeReferenceData
from bench.utils.fractional import INTEGER_ZERO
from bench.utils.tenacity import RetryOptions

from .const import (
    NodeMode,
    NodeReferenceKind,
    NodeTrait,
    NodeType,
    ProcessStatus,
)
from .graph import attach_node
from .node import node_trait_
from .object import BuiltinObject, object_
from .property import (
    p_internal,
    p_node_ancestor_with_self,
    p_node_parent,
    p_node_template,
    p_regular,
    p_system,
)
from .type import StringFormat

if TYPE_CHECKING:
    from bench.language import (
        Action,
        Agent,
        Bench,
        Block,
        Channel,
        Claim,
        Computer,
        Error,
        Flow,
        FlowEdge,
        Icon,
        Interruption,
        ModelDeveloper,
        ModelProvider,
        Node,
        NodeReference,
        Organization,
        Package,
        Page,
        Run,
        Span,
        Task,
        TextLine,
        Thread,
        User,
        Value,
    )

Subject = Union["User", "Organization", "Computer", "Agent"]

Runnable = Union["Agent", "Flow", "Action", "FlowEdge"]

Processable = Union["Channel", "Thread", "Agent", "Task", "Run", "Span"]


@object_()
class IsTemplatable(BuiltinObject):
    """A Node that can be templated (we can create Nodes that are derived from 'templates')."""

    template: Optional["Node"] = p_node_template(16)
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
            for child_type in CHILD_NODE_TYPES[self.metatype]:
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


@object_()
class IsInstantiable(IsTemplatable):
    """A Node that can be instanced (we can create Nodes that are 'instances' of this Node)."""

    ck: UUID = p_system(3, autoset=True)  # type: ignore

    # nocheckin: proper templating/instancing (for views/Variants)

    @property
    def is_instance(self) -> bool:
        return self.ck != cast("Node", self).id


@object_()
class IsSubject(BuiltinObject):
    """A Node that can be a Subject."""

    pass


@object_()
class IsOwnable(BuiltinObject):
    """A Node that can be owned by another Node."""

    owned_by: Optional[Subject] = p_internal(
        17,
        node_bench_from="self",
        node_exclude=("ck", "base_id"),
    )
    if TYPE_CHECKING:
        owned_by_id: Optional[UUID] = None
        owned_by_type: Optional[NodeType] = None
        owned_by_ptr: Optional[NodeReference] = None


@object_()
class IsJoinable(BuiltinObject):
    """A Node that can be joined by a Subject."""

    pass


@object_()
class IsClaimable(BuiltinObject):
    """A Node that can be claimed with a Claim."""

    claimed_by: Optional["Claim"] = p_internal(18, node_bench_from="self")
    if TYPE_CHECKING:
        claimed_by_id: Optional[UUID] = None
        claimed_by_ptr: Optional[NodeReference] = None


@object_()
class IsBased(BuiltinObject):
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


@object_()
class IsNamed(BuiltinObject):
    """A Node with a plain name."""

    name: str | None = p_regular(31, format=StringFormat.NAME)


@object_()
class IsTitled(BuiltinObject):
    """A Node with a rich title."""

    title: Optional["TextLine"] = p_regular(32)


@object_()
class IsOrdered(BuiltinObject):
    """A Node that can be ordered."""

    order_key: str = p_internal(33, default=INTEGER_ZERO)


@object_()
class IsModal(BuiltinObject):
    """A Node that can be in different modes."""

    mode: NodeMode = p_internal(25, default=NodeMode.MAIN)

    @property
    def is_active(self) -> bool:
        return (
            self.deleted_at is None  # type: ignore
            and self.archived_at is None  # type: ignore
            and self.mode < NodeMode.TEMPLATE
        )


@object_()
class IsExtensible(BuiltinObject):
    """A Node that can be extended with Fields."""

    # nocheckin: IsExtensible.value
    value: "Value | None" = p_regular(26)


@object_()
class IsProcessable(BuiltinObject):
    """A Node that can be processed somehow."""

    status: ProcessStatus = p_regular(80, default=ProcessStatus.CREATED)
    duration: Optional[timedelta] = p_internal(
        81,
        default=None,
        description="Duration from first attempt start to last attempt termination.",
    )
    error: Optional["Error"] = p_internal(82)
    interruption: Optional["Interruption"] = p_internal(
        83,
        node_bench_from="self",
        description="The latest Interruption concerning the Node.",
    )
    scheduled_at: Optional[datetime] = p_internal(
        85, description="When the Node is scheduled to start."
    )
    started_at: Optional[datetime] = p_internal(86, description="When the Node first started.")
    active_at: Optional[datetime] = p_internal(87, description="When the Node was last active.")
    interrupted_at: Optional[datetime] = p_internal(
        88, description="When the Node was interrupted."
    )
    terminated_at: Optional[datetime] = p_internal(
        89, description="When the Node was last terminated."
    )
    requested_stop_at: Optional[datetime] = p_regular(
        90, description="When the Node was requested to stop."
    )
    requested_pause_at: Optional[datetime] = p_regular(
        91, description="When the Node was requested to pause."
    )
    requested_resume_at: Optional[datetime] = p_regular(
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


@object_()
class IsComputable(BuiltinObject):
    """A Node that can have computation applied to it somehow."""

    # model
    # NOTE :Incomplete: IsComputable.model_id should probably be plural (model_ids?)
    model_developer: Optional["ModelDeveloper"] = p_internal(100)
    model_provider: Optional["ModelProvider"] = p_internal(101)
    model_id: Optional[str] = p_internal(102)
    model_name: Optional[str] = p_internal(103)
    # compute/cost/effort/budget/'juice'...?


@object_()
class IsRunnable(IsComputable):
    """A Node that can be run (at runtime in a Run)."""

    # control
    max_attempts: Optional[int] = p_regular(110)
    retry_interval: Optional[timedelta] = p_regular(111)
    backoff: Optional[float] = p_regular(112)

    def to_retry(self) -> RetryOptions:
        """Turns the options into our RetryOptions."""
        retry_interval = self.retry_interval.total_seconds() if self.retry_interval else 1
        return RetryOptions(
            max_attempts=self.max_attempts or 1,
            retry_interval=retry_interval,
            backoff=self.backoff or 22,
            max_retry_interval=max(30, retry_interval * 5),
        )


@node_trait_(NodeTrait.IN_BENCH)
class IsInBench(Node if TYPE_CHECKING else object):
    """A Node inside a Bench."""

    bench: "Bench | None" = p_node_ancestor_with_self(6, require=True, store=True, wire=True)
    if TYPE_CHECKING:
        bench_id: Optional[UUID] = None
        bench_ptr: Optional[NodeReference] = None

    @property
    def is_attached(self) -> bool:
        return self.parent_ptr is not None and self.bench is not None


@node_trait_(NodeTrait.IN_PACKAGE)
class IsInPackage(IsInBench):
    """A Node in a Package."""

    package: "Package | None" = p_node_ancestor_with_self(7, require=True, store=True, wire=True)
    if TYPE_CHECKING:
        package_id: Optional[UUID] = None
        package_ptr: Optional[NodeReference] = None


@node_trait_(NodeTrait.BLOCKABLE)
class IsBlockable(IsOrdered, IsInPackage):
    """A Node that can (but may not be) be inline on a Page as a Block."""

    parent: Union["Page", None] = p_node_parent(4)
    # name: 31
    # title: 32
    # order_key: 33
    icon: Optional["Icon"] = p_regular(34)
    definition: "Block | None" = p_internal(
        35,
        node_bench_from="self",
        description="The Block where this Node is 'defined'.",
    )
    if TYPE_CHECKING:
        definition_id: Optional[UUID] = None
        definition_ck: Optional[UUID] = None
        definition_ptr: Optional[NodeReference] = None

    @override
    def delete(self, _now: datetime | None = None):
        super().delete(_now=_now)
        # also delete defining Block (if any)
        if (
            (definition := self.definition) is not None
            and definition.node_id == self.id
            and not definition.is_deleted
        ):
            definition.delete(_now=_now)

    @override
    def restore(self, _now: datetime | None = None):
        super().restore(_now=_now)
        # also restore defining Block (if any)
        if (
            (definition := self.definition) is not None
            and definition.node_id == self.id
            and definition.is_deleted
        ):
            definition.restore(_now=_now)

    def wrap_in_block(self) -> "Block":
        """Wrap this Node in a *new* Block."""
        from bench.language import Block

        return Block.wrap(self)
