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
    builtin_entity,
    builtin_enum,
    builtin_event,
    builtin_property,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_event(NodeType.ENTITLEMENT_EVENT, is_abstract=True)
class EntitlementEvent(Event["Entitlement"]):
    node: "Entitlement" = builtin_property(101)
    target: "Entity" = builtin_property(110)


@builtin_event(NodeType.ENTITLEMENT_REQUESTED_EVENT)
class EntitlementRequestedEvent(EntitlementEvent):
    pass


@builtin_event(NodeType.ENTITLEMENT_GRANTED_EVENT)
class EntitlementGrantedEvent(EntitlementEvent):
    pass


@builtin_event(NodeType.ENTITLEMENT_REVOKED_EVENT)
class EntitlementRevokedEvent(EntitlementEvent):
    pass


@builtin_event(NodeType.ENTITLEMENT_EXPIRED_EVENT)
class EntitlementExpiredEvent(EntitlementEvent):
    pass


@builtin_enum(EnumType.ENTITLEMENT_TYPE)
class EntitlementType(Enum):
    """A Type of Entitlement."""

    PERMISSION = 1
    ROLE = 2


@builtin_entity(
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
