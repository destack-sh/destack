from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union

from destack.language.core import (
    UNSET,
    Entity,
    Enum,
    EnumType,
    Event,
    IsDeletable,
    IsJoinable,
    IsSubject,
    Node,
    NodeReference,
    NodeType,
    Spatial,
    builtin_enum,
    builtin_node,
    property_,
    property_parent_,
)
from destack.pb2 import EntitlementData, EntitlementEventData
from destack.utils.uuid import UUID

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.ENTITLEMENT_EVENT_TYPE)
class EntitlementEventType(Enum):
    """A Type of Entitlement Event."""

    REQUESTED = 1
    GRANTED = 2
    REVOKED = 3
    EXPIRED = 4


@builtin_node(NodeType.ENTITLEMENT_EVENT)
class EntitlementEvent(
    Event["Entitlement"],
    Node[EntitlementEventData],
):
    node: "Entitlement" = property_(35)


@builtin_enum(EnumType.ENTITLEMENT_TYPE)
class EntitlementType(Enum):
    """A Type of Entitlement."""

    PERMISSION = 1
    ROLE = 2


@builtin_node(NodeType.ENTITLEMENT)
class Entitlement(
    Spatial,
    Entity,
    IsDeletable,
    Node[EntitlementData],
):
    """A Entitlement to some Subject."""

    parent: Union["IsSubject", "IsJoinable", None] = property_parent_(node_is_customizable=True)
    type: EntitlementType = property_(30)
    expires_at: Optional[datetime] = property_(40)
    target: IsSubject = property_(41)
    if TYPE_CHECKING:
        target_ptr: NodeReference = UNSET
        target_id: UUID = UNSET
        target_type: NodeType = UNSET
