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

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.ENTITLEMENT_REQUESTED_EVENT)
class EntitlementRequestedEvent(
    Event["Entitlement"],
    Node,
):
    node: "Entitlement" = property_(35)
    target: "IsSubject" = property_(40)


@builtin_node(NodeType.ENTITLEMENT_GRANTED_EVENT)
class EntitlementGrantedEvent(
    Event["Entitlement"],
    Node,
):
    node: "Entitlement" = property_(35)
    target: "IsSubject" = property_(40)


@builtin_node(NodeType.ENTITLEMENT_REVOKED_EVENT)
class EntitlementRevokedEvent(
    Event["Entitlement"],
    Node,
):
    node: "Entitlement" = property_(35)
    target: "IsSubject" = property_(40)


@builtin_node(NodeType.ENTITLEMENT_EXPIRED_EVENT)
class EntitlementExpiredEvent(
    Event["Entitlement"],
    Node,
):
    node: "Entitlement" = property_(35)
    target: "IsSubject" = property_(40)


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
    Node,
):
    """A Entitlement to some Subject."""

    parent: Union["IsSubject", "IsJoinable", None] = property_parent_(node_is_customizable=True)
    type: EntitlementType = property_(30)
    expires_at: Optional[datetime] = property_(40)
    target: IsSubject = property_(41)
    if TYPE_CHECKING:
        target_ptr: NodeReference = UNSET
