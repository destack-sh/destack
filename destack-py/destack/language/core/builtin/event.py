from datetime import datetime
from typing import TYPE_CHECKING, Optional

from destack.utils.uuid import UUID

from .common import EnumType, RoleType, StoreDomain
from .entity import Entity
from .enum import Enum, builtin_enum
from .node import Node, NodeType, builtin_node
from .property import builtin_property, builtin_property_parent
from .trait import IsCustomizable, IsExtensible, IsSourceable, IsSpatial

if TYPE_CHECKING:
    from destack.language import (
        Client,
        Icon,
        IsSubject,
        NodeDefinitionReference,
        NodeReference,
        Snapshot,
        Space,
    )

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.EVENT_STATUS)
class EventStatus(Enum):
    """The (forever) status of an Event."""

    PENDING = 1, "Pending", "Pending application"
    COMPLETED = 10, "Completed", "Successfully applied"
    SKIPPED = 11, "Skipped", "Skipped and ignored"
    FAILED = 12, "Failed", "Could not apply"
    REJECTED = 13, "Rejected", "Denied by the system"


@builtin_node(
    NodeType.EVENT,
    frozen=True,  # type: ignore (frozen can't inherit from non-frozen usually, but it's fine for us)
    is_abstract=True,
)
class Event[N: Node = Node](IsSpatial, Node):
    """
    An Event is an immutable datum of something happening to an Entity.
    Events are proposed by Clients as pending Events, then applied or refused by the system.
    """

    __store_domain__ = StoreDomain.EVENT

    parent: Optional["Space"] = builtin_property_parent(is_readonly=True)

    # 10-20: event identity
    snapshot: Optional["Snapshot"] = builtin_property(
        11,
        is_readonly=True,
        is_managed=True,
        node_space_from="self",
        description="The Snapshot this Event originated from.",
    )
    # 20-40: node tracking
    created_at: datetime = builtin_property(
        20,
        is_managed=True,
        is_eq=False,
        is_readonly=True,
        can_write=RoleType.SYSTEM,
    )
    created_by: Optional["IsSubject"] = builtin_property(
        21,
        default=None,
        is_managed=True,
        is_eq=False,
        is_readonly=True,
        node_space_from="self",
        can_write=RoleType.SYSTEM,
    )
    client: Optional["Client"] = builtin_property(22, is_managed=True, is_readonly=True)
    client_nonce: Optional[UUID] = builtin_property(23, is_managed=True, is_readonly=True)
    status: "EventStatus" = builtin_property(
        30,
        is_repr=True,
        default=EventStatus.PENDING,
        description="The status of the Event.",
    )
    if TYPE_CHECKING:
        snapshot_ptr: Optional[NodeReference] = None
        created_by_ptr: Optional[NodeReference] = None
        client_ptr: Optional[NodeReference] = None

    # 100+: content
    if TYPE_CHECKING:
        node: Optional[N] = None
        node_ptr: Optional[NodeReference] = None
    else:
        node: Optional["Node"] = builtin_property(101, description="The Node this Event is about.")


@builtin_node(NodeType.CUSTOM_EVENT_DEFINITION)
class CustomEventDefinition(
    IsSpatial,
    IsSourceable,
    IsCustomizable,
    Entity,
):
    """A CustomEventDefinition defines a kind of CustomEvent with custom Properties."""

    base_type: Optional["NodeDefinitionReference"] = builtin_property(40)
    base_traits: list["NodeDefinitionReference"] = builtin_property(41)
    is_abstract: bool = builtin_property(45, default=False)

    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)


@builtin_node(
    NodeType.SIGNAL,
    frozen=True,  # type: ignore (frozen can't inherit from non-frozen usually, but it's fine for us)
    is_abstract=True,
)
class Signal(Event, IsExtensible):
    """
    A generic Signal of a CustomEventDefinition.
    More specific base Event types will be instanced of that base type instead.
    """

    definition: "CustomEventDefinition" = builtin_property(
        6,
        is_managed=True,
        is_readonly=True,
        description="The CustomEventDefinition this Signal is an instance of.",
    )
    if TYPE_CHECKING:
        definition_ptr: Optional[NodeReference] = None
