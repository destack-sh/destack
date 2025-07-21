from collections.abc import Generator
from contextlib import contextmanager
from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union

from destack.utils.uuid import UUID

from .common import EnumType, StoreDomain
from .const import ACTIVE_EVENT, UNSET
from .entity import Entity
from .enum import Enum, builtin_enum
from .node import Node, NodeType, builtin_node
from .property import ValueFactory, builtin_property

if TYPE_CHECKING:
    from destack.language import (
        Branch,
        Client,
        Icon,
        IsActor,
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
)
class Event[N: Node = Node](Node):
    """
    An Event is an immutable datum of something happening to an Entity.
    Events are proposed by Clients as pending Events, then approved or rejected by the system.
    """

    __store_domain__ = StoreDomain.EVENT

    # 10-20: event identity
    definition: Union["Entity", None] = builtin_property(
        11,
        is_internal=True,
        is_readonly=True,
        description="The definition this Event is an instance of.",
    )
    branch: "Branch" = builtin_property(
        12,
        is_readonly=True,
        is_internal=True,
        default_factory=ValueFactory.BRANCH,
        description="The Branch this Event originated from.",
    )
    snapshot: "Snapshot" = builtin_property(
        13,
        is_readonly=True,
        is_internal=True,
        default_factory=ValueFactory.SNAPSHOT,
        description="The Snapshot this Event originated from.",
    )
    preceded_by: Optional["Event"] = builtin_property(
        14,
        is_readonly=True,
        is_internal=True,
        description="The previous Event that this Event follows.",
    )
    caused_by: Optional["Event"] = builtin_property(
        15,
        is_readonly=True,
        is_internal=True,
        description="The Event that caused this Event (if any).",
    )
    # 20-40: node tracking
    created_at: datetime = builtin_property(
        20,
        is_internal=True,
        is_eq=False,
        is_readonly=True,
        description="The time this Event was created (system time).",
    )
    created_epoch: int = builtin_property(
        21,
        is_internal=True,
        is_eq=False,
        is_hash=False,
        is_repr=True,
        is_readonly=True,
        description="The logical time this Event was created (system time).",
    )
    created_by: Optional["IsActor"] = builtin_property(
        22,
        default=None,
        is_internal=True,
        is_eq=False,
        is_readonly=True,
        description="The Actor that created this Event.",
    )
    client: Optional["Client"] = builtin_property(
        23,
        is_internal=True,
        is_readonly=True,
        description="The Client that created this Event.",
    )
    client_nonce: Optional[UUID] = builtin_property(
        24,
        is_internal=True,
        is_readonly=True,
        description="The nonce of the Client that created this Event.",
    )
    client_created_at: datetime = builtin_property(
        25,
        is_internal=True,
        is_readonly=True,
        description="The time in the Client when it created this Event.",
    )
    client_epoch: int = builtin_property(
        26,
        is_internal=True,
        is_readonly=True,
        description="The logical time in the Client when it created this Event.",
    )
    status: "EventStatus" = builtin_property(
        40,
        is_repr=True,
        default=EventStatus.PENDING,
        description="The status of the Event.",
    )
    # caused_by/cascaded_from? (other Events that caused this event, like InputEvent or for cascading edits)
    if TYPE_CHECKING:
        branch_ptr: NodeReference = UNSET
        snapshot_ptr: NodeReference = UNSET
        preceded_by_ptr: Optional[NodeReference] = None
        created_by_ptr: Optional[NodeReference] = None
        client_ptr: Optional[NodeReference] = None

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
    NodeType.SIGNAL,
    frozen=True,  # type: ignore (frozen)
    is_extensible=True,
    is_abstract=True,
)
class Signal(Event):
    """
    A generic Event of a CustomEventDefinition.
    """

    definition: "CustomEvent" = builtin_property(
        6,
        is_internal=True,
        is_readonly=True,
        description="The CustomEventDefinition this Signal is an instance of.",
    )
    if TYPE_CHECKING:
        definition_ptr: Optional[NodeReference] = None
