from collections.abc import Generator
from contextlib import contextmanager
from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union

from destack.utils.uuid import UUID

from .builtin import EnumType, NodeType
from .common import GraphDomain, UInt128
from .const import ACTIVE_EVENT, UNSET
from .entity import Entity
from .enum import Enum, builtin_enum
from .meta import TagDeclaration
from .node import Node, builtin_node
from .property import ValueFactory, builtin_property

if TYPE_CHECKING:
    from destack.language import (
        Branch,
        Client,
        Icon,
        NodeDefinitionReference,
        NodeReference,
        Snapshot,
        Space,
    )

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.EVENT_STATUS)
class EventStatus(Enum):
    """The (forever) status of an Event."""

    # client
    PENDING = 1, "Pending", "Pending application on client"
    STAGED = 2, "Staged", "Optimistically staged on client"
    # PREDICTED, SUPERSEDED, ...
    # system
    APPROVED = 10, "Completed", "Successfully applied in system"
    SKIPPED = 11, "Skipped", "Skipped and ignored in system"
    FAILED = 12, "Failed", "Could not apply in system"
    REJECTED = 13, "Rejected", "Denied by the system"
    # COMPACTED, ...


@builtin_node(
    NodeType.EVENT,
    frozen=True,  # type: ignore (frozen can't inherit from non-frozen usually, but it's fine for us)
    is_abstract=True,
    tags=(
        TagDeclaration(id=20, name="system", description="System authority"),
        TagDeclaration(id=21, name="client", description="Client authority"),
    ),
)
class Event[N: Node = Node](Node):
    """
    An Event is an immutable datum of something happening to an Entity.
    Events are proposed by Clients as pending Events, then approved or rejected by the system.
    """

    __domain__ = GraphDomain.EVENT

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
        is_readonly=True,
        is_internal=True,
        is_identity=True,
        description="The previous Event that this Event follows.",
        tags=("identity",),
    )
    caused_by: Optional["Event"] = builtin_property(
        15,
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
        default_factory=ValueFactory.EPOCH,
        description="The logical time this Event was created (system).",
        tags=("tracking", "system"),
    )
    created_by: "Entity" = builtin_property(
        22,
        is_internal=True,
        is_eq=False,
        is_readonly=True,
        default_factory=ValueFactory.ACTOR,
        description="The Actor that created this Event.",
        tags=("tracking",),
    )
    client: "Client" = builtin_property(
        23,
        is_internal=True,
        is_readonly=True,
        default_factory=ValueFactory.CLIENT,
        description="The Client that created this Event (client).",
        tags=("tracking", "client"),
    )
    client_nonce: UUID = builtin_property(
        24,
        is_internal=True,
        is_readonly=True,
        default_factory=ValueFactory.CLIENT_NONCE,
        description="The nonce of the Client that created this Event (client).",
        tags=("tracking", "client"),
    )
    client_created_at: datetime = builtin_property(
        25,
        is_internal=True,
        is_readonly=True,
        default_factory=ValueFactory.NOW,
        description="The time in the Client when it created this Event (client).",
        tags=("tracking", "client"),
    )
    client_epoch: UInt128 = builtin_property(
        26,
        is_internal=True,
        is_readonly=True,
        default_factory=ValueFactory.EPOCH,
        description="The logical time in the Client when it created this Event (client).",
        tags=("tracking", "client"),
    )
    status: "EventStatus" = builtin_property(
        30,
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
    if TYPE_CHECKING:
        node: Optional[N] = None
        node_ptr: Optional[NodeReference] = None
    else:
        node: Optional["Node"] = builtin_property(101, description="The Node this Event is about.")

    @property
    def parent(self) -> "Space":
        """The Space this Event is in."""
        return self.space

    @property
    def parent_ptr(self) -> "NodeReference":
        """The NodeReference to the parent of this Event."""
        return self.space_ptr

    @contextmanager
    def active(self: "Event[Node]") -> Generator["Event[Node]", None, None]:
        """Set this Event as the active Event."""
        token = ACTIVE_EVENT.set(self)
        try:
            ACTIVE_EVENT.set(self)
            yield self
        finally:
            ACTIVE_EVENT.reset(token)


@builtin_node(NodeType.CUSTOM_EVENT)
class CustomEvent(
    Entity,
):
    """A CustomEvent defines a custom Event with custom Properties."""

    icon: "Icon | None" = builtin_property(102)

    base_type: Optional["NodeDefinitionReference"] = builtin_property(110)
    self_traits: list["NodeDefinitionReference"] = builtin_property(111)
    is_abstract: bool = builtin_property(112, default=False)


@builtin_node(
    NodeType.SIGNAL_EVENT,
    frozen=True,  # type: ignore (frozen)
    is_extensible=True,
    is_abstract=True,
)
class SignalEvent(Event):
    """
    A generic Event of a CustomEvent.
    """

    definition: "CustomEvent" = builtin_property(
        6,
        is_internal=True,
        is_readonly=True,
        description="The CustomEvent this Signal is an instance of.",
    )
    if TYPE_CHECKING:
        definition_ptr: Optional[NodeReference] = None
