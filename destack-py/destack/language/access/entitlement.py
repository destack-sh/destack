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
    IsSpatial,
    IsSubject,
    NodeReference,
    NodeType,
    builtin_enum,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.ENTITLEMENT_EVENT, frozen=True, is_abstract=True)
class EntitlementEvent(Event["Entitlement"]):
    node: "Entitlement" = builtin_property(101)
    target: "IsSubject" = builtin_property(110)


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


@builtin_node(NodeType.ENTITLEMENT)
class Entitlement(IsSpatial, IsDeletable, Entity):
    """A Entitlement to some Subject."""

    parent: Union["IsSubject", "IsJoinable", None] = builtin_property_parent()
    type: EntitlementType = builtin_property(100)
    expires_at: Optional[datetime] = builtin_property(110)
    target: IsSubject = builtin_property(111)
    if TYPE_CHECKING:
        target_ptr: NodeReference = UNSET
