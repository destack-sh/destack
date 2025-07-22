from datetime import datetime
from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    UNSET,
    Entity,
    Enum,
    EnumType,
    Event,
    NodeReference,
    NodeType,
    builtin_enum,
    builtin_node,
    builtin_property,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.ENTITLEMENT_EVENT, frozen=True, is_abstract=True)
class EntitlementEvent(Event["Entitlement"]):
    node: "Entitlement" = builtin_property(101)
    target: "Entity" = builtin_property(110)


@builtin_node(NodeType.ENTITLEMENT_REQUESTED_EVENT, frozen=True)
class EntitlementRequestedEvent(EntitlementEvent):
    pass


@builtin_node(NodeType.ENTITLEMENT_GRANTED_EVENT, frozen=True)
class EntitlementGrantedEvent(EntitlementEvent):
    pass


@builtin_node(NodeType.ENTITLEMENT_REVOKED_EVENT, frozen=True)
class EntitlementRevokedEvent(EntitlementEvent):
    pass


@builtin_node(NodeType.ENTITLEMENT_EXPIRED_EVENT, frozen=True)
class EntitlementExpiredEvent(EntitlementEvent):
    pass


@builtin_enum(EnumType.ENTITLEMENT_TYPE)
class EntitlementType(Enum):
    """A Type of Entitlement."""

    PERMISSION = 1
    ROLE = 2


@builtin_node(
    NodeType.ENTITLEMENT,
    event_types=(NodeType.ENTITLEMENT_EVENT,),
)
class Entitlement(
    Entity,
):
    """A Entitlement to some Actor."""

    type: EntitlementType = builtin_property(100)
    expires_at: Optional[datetime] = builtin_property(110)
    target: "Entity" = builtin_property(111)
    if TYPE_CHECKING:
        target_ptr: NodeReference = UNSET
