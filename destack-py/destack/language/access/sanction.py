from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union

from destack.language.core import (
    UNSET,
    Entity,
    Enum,
    EnumType,
    Event,
    IsActor,
    IsExtensible,
    IsJoinable,
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


@builtin_node(NodeType.SANCTION_EVENT, frozen=True, is_abstract=True)
class SanctionEvent(Event["Sanction"]):
    node: "Sanction" = builtin_property(101)
    target: "IsActor" = builtin_property(110)


@builtin_node(NodeType.SANCTION_REQUESTED_EVENT, frozen=True)
class SanctionRequestedEvent(SanctionEvent):
    pass


@builtin_node(NodeType.SANCTION_GRANTED_EVENT, frozen=True)
class SanctionGrantedEvent(SanctionEvent):
    pass


@builtin_node(NodeType.SANCTION_REVOKED_EVENT, frozen=True)
class SanctionRevokedEvent(SanctionEvent):
    pass


@builtin_node(NodeType.SANCTION_EXPIRED_EVENT, frozen=True)
class SanctionExpiredEvent(SanctionEvent):
    pass


@builtin_enum(EnumType.SANCTION_TYPE)
class SanctionType(Enum):
    """A Type of Sanction."""

    BAN = 1
    MUTE = 2


@builtin_node(
    NodeType.SANCTION,
    event_types=(NodeType.SANCTION_EVENT,),
)
class Sanction(
    IsExtensible,
    Entity,
):
    """A Sanction on some Actor."""

    parent: Union["IsActor", "IsJoinable", None] = builtin_property_parent()
    type: SanctionType = builtin_property(100)
    expires_at: Optional[datetime] = builtin_property(110)
    target: IsActor = builtin_property(111)
    if TYPE_CHECKING:
        target_ptr: NodeReference = UNSET
