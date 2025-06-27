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
from destack.proto import SanctionGrantedEventProto, SanctionProto, SanctionRequestedEventProto

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.SANCTION_REQUESTED_EVENT)
class SanctionRequestedEvent(
    Event["Sanction"],
    Node[SanctionRequestedEventProto],
):
    node: "Sanction" = property_(35)
    target: "IsSubject" = property_(40)


@builtin_node(NodeType.SANCTION_GRANTED_EVENT)
class SanctionGrantedEvent(
    Event["Sanction"],
    Node[SanctionGrantedEventProto],
):
    node: "Sanction" = property_(35)
    target: "IsSubject" = property_(40)


@builtin_node(NodeType.SANCTION_REVOKED_EVENT)
class SanctionRevokedEvent(
    Event["Sanction"],
    Node[SanctionGrantedEventProto],
):
    node: "Sanction" = property_(35)
    target: "IsSubject" = property_(40)


@builtin_node(NodeType.SANCTION_EXPIRED_EVENT)
class SanctionExpiredEvent(
    Event["Sanction"],
    Node[SanctionGrantedEventProto],
):
    node: "Sanction" = property_(35)
    target: "IsSubject" = property_(40)


@builtin_enum(EnumType.SANCTION_TYPE)
class SanctionType(Enum):
    """A Type of Sanction."""

    BAN = 1
    MUTE = 2


@builtin_node(NodeType.SANCTION)
class Sanction(
    Spatial,
    Entity,
    IsDeletable,
    Node[SanctionProto],
):
    """A Sanction on some Subject."""

    parent: Union["IsSubject", "IsJoinable", None] = property_parent_(node_is_customizable=True)
    type: SanctionType = property_(30)
    expires_at: Optional[datetime] = property_(40)
    target: IsSubject = property_(41)
    if TYPE_CHECKING:
        target_ptr: NodeReference = UNSET
