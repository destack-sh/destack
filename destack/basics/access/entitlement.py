from datetime import datetime
from typing import TYPE_CHECKING, Optional

from destack.core import (
    UNSET,
    Entity,
    Enum,
    EnumType,
    Event,
    NodeReference,
    NodeType,
    declare_entity,
    declare_enum,
    declare_event,
    declare_property,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@declare_event(NodeType.ENTITLEMENT_EVENT, is_abstract=True)
class EntitlementEvent(Event):
    entitlement: "Entitlement" = declare_property(101)
    target: "Entity" = declare_property(110)


@declare_event(NodeType.ENTITLEMENT_REQUESTED_EVENT)
class EntitlementRequestedEvent(EntitlementEvent):
    pass


@declare_event(NodeType.ENTITLEMENT_GRANTED_EVENT)
class EntitlementGrantedEvent(EntitlementEvent):
    pass


@declare_event(NodeType.ENTITLEMENT_REVOKED_EVENT)
class EntitlementRevokedEvent(EntitlementEvent):
    pass


@declare_event(NodeType.ENTITLEMENT_EXPIRED_EVENT)
class EntitlementExpiredEvent(EntitlementEvent):
    pass


@declare_enum(EnumType.ENTITLEMENT_TYPE)
class EntitlementType(Enum):
    """A Type of Entitlement."""

    PERMISSION = 1
    ROLE = 2


@declare_entity(
    NodeType.ENTITLEMENT,
    event_types=(NodeType.ENTITLEMENT_EVENT,),
)
class Entitlement(
    Entity,
):
    """A Entitlement to some Actor."""

    type: EntitlementType = declare_property(100)
    expires_at: Optional[datetime] = declare_property(110)
    target: "Entity" = declare_property(111)
    if TYPE_CHECKING:
        target_ptr: NodeReference = UNSET
