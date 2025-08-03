from collections.abc import Generator
from contextlib import contextmanager
from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union, dataclass_transform

from destack.utils.env import IS_DEV, IS_TEST
from destack.utils.uuid import UUID

from ..common.relation import NodeReference
from .builtin import EnumType, NodeType, StructType
from .const import ACTIVE_EVENT, UNSET
from .declaration import TagDeclaration, builtin_method
from .entity import Entity
from .enum import Enum, builtin_enum
from .node import Node, _process_node_cls
from .property import _PROPERTY_SPECIFIERS, ValueFactory, builtin_property
from .types import UInt128

if TYPE_CHECKING:
    from destack.language import Branch, Client, Snapshot, Space

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.EVENT_STATUS)
class EventStatus(Enum):
    """The (forever) status of an Event."""

    # client
    PENDING = 1, "Pending", "Pending application on client"
    STAGED = 2, "Staged", "Optimistically staged on client"
    # PREDICTED, SUPERSEDED, ...
    # system hot
    APPROVED = 10, "Completed", "Successfully applied in system"
    SKIPPED = 11, "Skipped", "Skipped and ignored in system"
    FAILED = 12, "Failed", "Could not apply in system"
    REJECTED = 13, "Rejected", "Denied by the system"
    # system cold
    # COMPACTED, ...


@dataclass_transform(
    kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS, frozen_default=True
)
def builtin_event(
    # meta
    event_type: NodeType,
    *,
    is_abstract: bool = False,
    is_final: bool = False,
    # inheritance
    tags: tuple["TagDeclaration", ...] = (),
    # associations
    event_types: tuple[NodeType, ...] = (),
    enum_types: tuple[EnumType, ...] = (),
    message_types: tuple[StructType, ...] = (),
):
    """Register a class as a concrete Event for the given Event type."""

    def decorate(cls: type) -> type:
        cls = _process_node_cls(
            # meta
            cls=cls,
            node_type=event_type,
            is_abstract=is_abstract,
            is_final=is_final,
            is_singleton=False,
            is_frozen=False,
            # inheritance
            traits=(),
            # content
            indexes=(),
            constraints=(),
            permissions=(),
            tags=tags,
            # tree
            expected_parent_types=(),
            expected_child_types=(),
            expected_ancestor_types=(),
            expected_descendant_types=(),
            # associations
            event_types=event_types,
            enum_types=enum_types,
            message_types=message_types,
        )
        if IS_DEV or IS_TEST:
            assert event_type.name.endswith("EVENT"), f"Event {cls.__name__} must end with 'EVENT'"
            assert event_type == NodeType.EVENT or NodeType.EVENT in cls.__declaration__.inherits, (
                f"Event {cls.__name__} must inherit from Event"
            )
        return cls

    return decorate


@builtin_event(
    NodeType.EVENT,
    is_abstract=True,
    tags=(
        TagDeclaration(id=20, name="system", description="System authority"),
        TagDeclaration(id=21, name="client", description="Client authority"),
    ),
)  # type: ignore (frozen can't inherit from non-frozen usually, but it's fine here)
class Event(Node):
    """
    An Event is an immutable datum of something happening to an Entity.

    Events are proposed by Clients, then approved or discarded by the system.
    All Events are stored, even rejected ones.
    The system Event log is append-only and immutable.

    The client_* data is as-is provided by Clients, and can not be verified by the system.
    """

    # 10-20: Event identity
    definition: Union["Entity", None] = builtin_property(
        11,
        is_internal=True,
        is_readonly=True,
        is_identity=True,
        description="The definition this Event is an instance of.",
        tags=("identity",),
    )
    branch: "Branch" = builtin_property(
        12,
        is_readonly=True,
        is_internal=True,
        is_identity=True,
        default_factory=ValueFactory.BRANCH,
        description="The Branch this Event originated from.",
        tags=("identity",),
    )
    snapshot: "Snapshot" = builtin_property(
        13,
        is_readonly=True,
        is_internal=True,
        is_identity=True,
        default_factory=ValueFactory.SNAPSHOT,
        description="The Snapshot this Event originated from.",
        tags=("identity",),
    )
    preceded_by: Optional["Event"] = builtin_property(
        14,
        is_eq=False,
        is_hash=False,
        is_readonly=True,
        is_internal=True,
        is_identity=True,
        description="The previous Event that this Event follows.",
        tags=("identity",),
    )
    caused_by: Optional["Event"] = builtin_property(
        15,
        is_eq=False,
        is_hash=False,
        is_readonly=True,
        is_internal=True,
        is_identity=True,
        description="The Event that caused this Event (if any).",
        tags=("identity",),
    )
    # 20-40: Event tracking
    created_at: datetime = builtin_property(
        20,
        is_internal=True,
        is_eq=False,
        is_hash=False,
        is_readonly=True,
        default_factory=ValueFactory.NOW,
        description="The time this Event was created (system).",
        tags=("tracking", "system"),
    )
    created_epoch: UInt128 = builtin_property(
        21,
        is_internal=True,
        is_eq=False,
        is_hash=False,
        is_repr=True,
        is_readonly=True,
        default_factory=ValueFactory.REMOTE_EPOCH,
        description="The logical time this Event was created (system).",
        tags=("tracking", "system"),
    )
    created_by: "Entity" = builtin_property(
        22,
        is_internal=True,
        is_eq=False,
        is_hash=False,
        is_repr=False,
        is_readonly=True,
        default_factory=ValueFactory.ACTOR,
        description="The Actor that created this Event.",
        tags=("tracking",),
    )
    client: "Client" = builtin_property(
        23,
        is_internal=True,
        is_eq=False,
        is_hash=False,
        is_repr=True,
        is_readonly=True,
        default_factory=ValueFactory.CLIENT,
        description="The Client that created this Event (client, but verified).",
        tags=("tracking", "client"),
    )
    client_nonce: UUID = builtin_property(
        24,
        is_eq=False,
        is_hash=False,
        is_repr=False,
        is_internal=True,
        is_readonly=True,
        default_factory=ValueFactory.CLIENT_NONCE,
        description="The nonce of the Client that created this Event (client).",
        tags=("tracking", "client"),
    )
    client_created_at: datetime = builtin_property(
        25,
        is_eq=False,
        is_hash=False,
        is_repr=False,
        is_internal=True,
        is_readonly=True,
        default_factory=ValueFactory.NOW,
        description="The time in the Client when it created this Event (client).",
        tags=("tracking", "client"),
    )
    client_remote_epoch: UInt128 = builtin_property(
        26,
        is_eq=False,
        is_hash=False,
        is_repr=True,
        is_internal=True,
        is_readonly=True,
        default_factory=ValueFactory.REMOTE_EPOCH,
        description="The logical time last seen from the system in the Client for this space (client).",
        tags=("tracking", "client"),
    )
    client_local_epoch: UInt128 = builtin_property(
        27,
        is_eq=False,
        is_hash=False,
        is_repr=True,
        is_internal=True,
        is_readonly=True,
        default_factory=ValueFactory.LOCAL_EPOCH,
        description="The logical time in the Client when it created this Event (client).",
        tags=("tracking", "client"),
    )
    status: "EventStatus" = builtin_property(
        30,
        is_eq=False,
        is_hash=False,
        is_repr=True,
        default=EventStatus.PENDING,
        description="The status of the Event (system).",
        tags=("tracking", "system"),
    )
    if TYPE_CHECKING:
        branch_ptr: NodeReference = UNSET
        snapshot_ptr: NodeReference = UNSET
        preceded_by_ptr: Optional[NodeReference] = None
        created_by_ptr: NodeReference = UNSET
        client_ptr: NodeReference = UNSET

    # 100+: content

    @property
    def parent(self) -> "Space":
        """The Space this Event is in."""
        return self.space

    @property
    def parent_ptr(self) -> "NodeReference":
        """The NodeReference to the parent of this Event."""
        return self.space_ptr

    @builtin_method(2)
    def to_ref(self) -> "NodeReference":
        """Gets a reference to this Node."""
        if self._ref is None:
            self._ref = NodeReference(
                type=self.metatype,
                id=self.id,
                space_id=self.space_ptr.id,
                branch_id=self.branch_ptr.id,
                snapshot_id=self.snapshot_ptr.id,
            )
        return self._ref

    @contextmanager
    def active(self) -> Generator["Event", None, None]:
        """Set this Event as the active Event."""
        token = ACTIVE_EVENT.set(self)
        try:
            ACTIVE_EVENT.set(self)
            yield self
        finally:
            ACTIVE_EVENT.reset(token)
