import abc
from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Any, Callable, ClassVar, Optional, Self, Union
from uuid import UUID

from bench.language.registry import CHILD_NODE_TYPES
from bench.pb2 import AnyNodeData, NodeReferenceData
from bench.utils.uuidt import UUIDT

from .const import RESOURCE_NODE_TYPES, NodeMode, NodeType, ReferenceKind, StructType, bittuple
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
        Choice,
        Claim,
        Class,
        Client,
        ComputedSourceIn,
        ComputedValue,
        ComputedValueMode,
        Computer,
        Database,
        Error,
        Flow,
        Interruption,
        Kit,
        Link,
        Node,
        NodeReference,
        Organization,
        Package,
        Page,
        PathIn,
        Plan,
        Record,
        Resource,
        Session,
        Space,
        Task,
        Team,
        TextLine,
        Thread,
        User,
        View,
    )

Ownable = Union[
    "Bench",
    "Resource",
    "Package",
    "Space",
    "Page",
    "Flow",
    "Kit",
    "View",
    "Database",
    "Thread",
    "Plan",
    "Task",
    "Claim",
    "Agent",
    "Record",
]
OWNABLE_NODE_TYPES = bittuple(
    NodeType.BENCH,
    *RESOURCE_NODE_TYPES.tuple,
    NodeType.PACKAGE,
    NodeType.SPACE,
    NodeType.PAGE,
    NodeType.FLOW,
    NodeType.KIT,
    NodeType.VIEW,
    NodeType.DATABASE,
    NodeType.THREAD,
    NodeType.PLAN,
    NodeType.TASK,
    NodeType.CLAIM,
    NodeType.AGENT,
    NodeType.RECORD,
)

Claimable = Union[
    "Resource",
    "Page",
    "Flow",
    "Action",
    "Kit",
    "View",
    "Database",
    "Plan",
    "Task",
    "Agent",
    "Record",
]
CLAIMABLE_NODE_TYPES = bittuple(
    *RESOURCE_NODE_TYPES.tuple,
    NodeType.PAGE,
    NodeType.FLOW,
    NodeType.ACTION,
    NodeType.KIT,
    NodeType.VIEW,
    NodeType.DATABASE,
    NodeType.PLAN,
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


Runnable = Union["Flow", "Action", "Link"]
RUNNABLE_NODE_TYPES = bittuple(NodeType.FLOW, NodeType.ACTION, NodeType.LINK)

FieldBaseNode = Union["Agent", "Flow", "Action", "Class", "Database"]
FIELD_BASE_NODE_TYPES = bittuple(
    NodeType.AGENT,
    NodeType.FLOW,
    NodeType.ACTION,
    NodeType.CLASS,
    NodeType.DATABASE,
)

TypeBaseNode = Union[FieldBaseNode, "Choice"]
TYPE_BASE_NODE_TYPES = bittuple(*FIELD_BASE_NODE_TYPES, NodeType.CHOICE)


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
            instance_kwargs["mode"] = NodeMode.MAIN
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
                    exclude=(
                        ReferenceKind.NODE_PARENT,
                        ReferenceKind.NODE_CHILDREN,
                        ReferenceKind.NODE_TEMPLATE,
                    ),
                )

        # append to our parent to re-attach
        parent = self.parent
        if detach:
            instance.parent_ptr = None
        elif parent:
            parent.append(instance)
        return instance


@object_()
class IsInstantiable(IsTemplatable):
    """A Node that can be instanced (we can create Nodes that are 'instances' of this Node)."""

    ck: UUID = p_system(3, default=None, require=True, autoset=True)  # type: ignore


@object_()
class IsSubject(BuiltinObject):
    """A Node that can be a Subject."""

    pass


@object_()
class IsOwnable(BuiltinObject):
    """A Node that can be owned by another Node."""

    owned_by: Optional[Subject] = p_regular(
        17,
        require=False,
        array=False,
        references=SUBJECT_NODE_TYPES.tuple,
        same_bench=True,
        baseless=True,
        ckless=True,
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

    claimed_by: Optional["Claim"] = p_regular(
        18, require=False, array=False, references=NodeType.CLAIM, same_bench=True
    )
    if TYPE_CHECKING:
        claimed_by_id: Optional[UUID] = None
        claimed_by_ptr: Optional[NodeReference] = None


@object_()
class IsComputable(BuiltinObject):
    """A Node that can be computed at runtime."""

    computed_values: list["ComputedValue"] = p_internal(
        28, require=False, array=True, struct=StructType.COMPUTED_VALUE
    )

    def set_computed(
        self,
        target: "PathIn",
        source: "ComputedSourceIn",
        *,
        mode: "ComputedValueMode | None" = None,
        is_active: bool = True,
    ):
        """Sets and overrides the computed value for the target path."""
        from .expression import ComputedValue, ComputedValueMode

        computed_value = ComputedValue.new(
            target=target, source=source, mode=mode or ComputedValueMode.ALWAYS, is_active=is_active
        )
        self.computed_values = [
            *(cv for cv in self.computed_values if cv.target_path != target),
            computed_value,
        ]

    def clear_computed(self, target: "PathIn"):
        """Clears the computed value for the target path."""
        from .path import to_path

        target = to_path(target)
        self.computed_values = [
            *(cv for cv in self.computed_values if cv.target_path != target),
        ]


@object_()
class IsBased(BuiltinObject, abc.ABC):
    """A Node which may have a 'base' in another Node (e.g., its type definition)."""

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
class IsTimed(BuiltinObject, abc.ABC):
    """A Node with a time-based identity."""

    __id_factory__: ClassVar[Callable[[], UUID]] = UUIDT


@object_()
class IsNamed(BuiltinObject):
    """A Node with a plain name."""

    name: str | None = p_regular(31, constraint=NAME_CONSTRAINT)


@object_()
class IsTitled(BuiltinObject):
    """A Node with a rich title."""

    title: Optional["TextLine"] = p_regular(32, struct=StructType.TEXT_LINE)


@object_()
class IsRunnable(BuiltinObject):
    """A Node that can be run."""

    pass


@object_()
class IsTypeBase(BuiltinObject):
    """A Node that can be a Type base."""

    pass


@object_()
class IsFieldBase(IsTypeBase):
    """A Node that can be a Field base."""

    pass


@object_()
class IsModal(BuiltinObject):
    """A Node that can be in different modes."""

    mode: NodeMode = p_internal(20, default=NodeMode.MAIN, default_sql=str(NodeMode.MAIN.value))

    @property
    def is_active(self) -> bool:
        return (
            self.deleted_at is None  # type: ignore
            and self.archived_at is None  # type: ignore
            and self.mode < NodeMode.TEMPLATE
        )


@object_()
class IsRuntime(BuiltinObject):
    """Context for a Node that's relevant at runtime."""

    # NOTE :Security: session context properties are p_internal (not p_system) so we can update
    #   them in all Clients. But this also means Users could mess with them if they really want to.
    session: Optional["Session"] = p_internal(
        91, require=False, array=False, references=NodeType.SESSION, same_bench=True
    )
    client: Optional["Client"] = p_internal(
        93, require=False, array=False, references=NodeType.CLIENT, same_bench=True
    )
    computer: Optional["Computer"] = p_internal(
        94, require=False, array=False, references=NodeType.COMPUTER, same_bench=True
    )
    user: Optional["User"] = p_internal(95, require=False, array=False, references=NodeType.USER)
    agent: Optional["Agent"] = p_internal(96, require=False, array=False, references=NodeType.AGENT)
    if TYPE_CHECKING:
        session_ptr: Optional[NodeReference] = None
        session_id: Optional[UUID] = None
        client_ptr: Optional[NodeReference] = None
        client_id: Optional[UUID] = None
        computer_ptr: Optional[NodeReference] = None
        computer_id: Optional[UUID] = None
        user_ptr: Optional[NodeReference] = None
        user_id: Optional[UUID] = None
        agent_ptr: Optional[NodeReference] = None
        agent_id: Optional[UUID] = None
        agent_ck: Optional[UUID] = None

    @property
    def runtime(self):
        """The Runtime associated with this context (if any)."""
        return self.session._runtime if self.session is not None else None


@object_()
class IsRuntimeControllable(IsRuntime):
    """A Node that can be 'controlled' (paused, resumed, stopped, etc.) at runtime."""

    # status: 80 ...
    duration: Optional[timedelta] = p_internal(
        81,
        default=None,
        description="Duration from first attempt start to last attempt termination.",
    )
    scheduled_at: Optional[datetime] = p_internal(
        82, default=None, description="When the Node is scheduled to start."
    )
    started_at: Optional[datetime] = p_internal(
        83, default=None, description="When the Node first started."
    )
    stopped_at: Optional[datetime] = p_internal(
        84, default=None, description="When the Node was requested to stop."
    )
    interrupted_at: Optional[datetime] = p_internal(
        85, default=None, description="When the Node was interrupted."
    )
    paused_at: Optional[datetime] = p_internal(
        86, default=None, description="When the Node was requested to pause."
    )
    resumed_at: Optional[datetime] = p_internal(
        87, default=None, description="When the Node was requested to resume."
    )
    terminated_at: Optional[datetime] = p_internal(
        88, default=None, description="When the Node was last terminated."
    )
    error: Optional["Error"] = p_internal(
        89, default=None, require=False, array=False, struct=StructType.ERROR
    )
    interruption: Optional["Interruption"] = p_internal(
        90,
        require=False,
        array=False,
        references=NodeType.INTERRUPTION,
        same_bench=True,
        description="The latest Interruption concerning the Node.",
    )
    if TYPE_CHECKING:
        interruption_ptr: Optional[NodeReference] = None
        interruption_id: Optional[UUID] = None
