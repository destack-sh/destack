import abc
from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Any, Optional, Self, Union, cast

from fastuuid import UUID

from bench.language.registry import CHILD_NODE_TYPES
from bench.pb2 import AnyNodeData, NodeReferenceData
from bench.utils.fractional import INTEGER_ZERO
from bench.utils.tenacity import RetryOptions

from .const import (
    RESOURCE_NODE_TYPES,
    NodeMode,
    NodeReferenceKind,
    NodeType,
    ProcessStatus,
    bittuple,
)
from .list import attach_node
from .object import BuiltinObject, object_
from .property import p_internal, p_node_template, p_regular, p_system
from .validation import NAME_CONSTRAINT

if TYPE_CHECKING:
    from bench.language import (
        Action,
        Agent,
        Bench,
        BenchNode,
        Channel,
        Claim,
        Computer,
        Cursor,
        Error,
        Flow,
        FlowEdge,
        Interruption,
        Message,
        ModelDeveloper,
        ModelProvider,
        Node,
        NodeReference,
        Organization,
        Package,
        Page,
        Record,
        ResourceBase,
        Run,
        Service,
        Space,
        Span,
        Table,
        Task,
        Team,
        TextLine,
        Thread,
        User,
    )

Ownable = Union[
    "Bench",
    "ResourceBase",
    "Package",
    "Space",
    "Page",
    "Flow",
    "Service",
    "Table",
    "Thread",
    "Task",
    "Claim",
    "Agent",
    "Record",
    "Message",
    "Cursor",
]
OWNABLE_NODE_TYPES = bittuple(
    NodeType.BENCH,
    *RESOURCE_NODE_TYPES.tuple,
    NodeType.PACKAGE,
    NodeType.SPACE,
    NodeType.PAGE,
    NodeType.FLOW,
    NodeType.SERVICE,
    NodeType.TABLE,
    NodeType.THREAD,
    NodeType.TASK,
    NodeType.CLAIM,
    NodeType.AGENT,
    NodeType.RECORD,
    NodeType.MESSAGE,
    NodeType.CURSOR,
    NodeType.ROUTE,
    NodeType.SCENE,
)

Claimable = Union[
    "ResourceBase",
    "Page",
    "Flow",
    "Action",
    "Service",
    "Table",
    "Task",
    "Agent",
    "Record",
]
CLAIMABLE_NODE_TYPES = bittuple(
    *RESOURCE_NODE_TYPES.tuple,
    NodeType.PAGE,
    NodeType.FLOW,
    NodeType.ACTION,
    NodeType.SERVICE,
    NodeType.TABLE,
    NodeType.TASK,
    NodeType.AGENT,
    NodeType.RECORD,
)

Joinable = Union["Package", "Team", "Channel", "Thread"]
JOINABLE_NODE_TYPES = bittuple(
    NodeType.PACKAGE,
    NodeType.TEAM,
    NodeType.CHANNEL,
    NodeType.THREAD,
)

Subject = Union["User", "Organization", "Computer", "Agent"]
SUBJECT_NODE_TYPES = bittuple(
    NodeType.USER,
    NodeType.ORGANIZATION,
    NodeType.COMPUTER,
    NodeType.AGENT,
)

Runnable = Union["Agent", "Flow", "Action", "FlowEdge"]
RUNNABLE_NODE_TYPES = bittuple(NodeType.AGENT, NodeType.FLOW, NodeType.ACTION, NodeType.FLOW_EDGE)

Processable = Union["Channel", "Thread", "Agent", "Task", "Run", "Span"]
PROCESSABLE_NODE_TYPES = bittuple(
    NodeType.CHANNEL,
    NodeType.THREAD,
    NodeType.AGENT,
    NodeType.RUN,
    NodeType.TASK,
    NodeType.SPAN,
)


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
    def base(self) -> Optional["BenchNode"]: ...

    @property
    def base_id(self) -> Optional[UUID]:
        return self.base.id if self.base is not None else None

    @staticmethod
    @abc.abstractmethod
    def get_base_from_data(data: AnyNodeData) -> Optional[NodeReferenceData]: ...

    @staticmethod
    @abc.abstractmethod
    def get_base_from_partial(data: dict[str, Any]) -> Optional["BenchNode"]: ...


@object_()
class IsNamed(BuiltinObject):
    """A Node with a plain name."""

    name: str | None = p_regular(31, constraint=NAME_CONSTRAINT)


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

    def update_status_from(self, *children: Processable) -> None:
        """
        Update this Node as a parent of other Processables.
        """
        if not children:
            self.status = ProcessStatus.IDLE
            return

        # always 'touch' on update
        now = self.active_session.oracle.utc()
        self.active_at = now

        # start as soon as any child is started
        for child in children:
            if child.started_at is not None and (
                self.started_at is None or child.started_at < self.started_at
            ):
                self.started_at = child.started_at

        # status
        if any(c.status.is_bad for c in children):
            self.status = ProcessStatus.FAILING
        elif any(c.status.is_active or c.status.is_interrupted for c in children):
            self.status = ProcessStatus.RUNNING
        elif self.should_stop and all(c.status.is_terminal for c in children):
            self.terminated_at = now
            self.status = ProcessStatus.COMPLETED
        else:
            self.status = ProcessStatus.IDLE

        # duration
        if self.started_at is not None and self.terminated_at is not None:
            duration = self.terminated_at - self.started_at
            if self.duration != duration:
                self.duration = duration

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
