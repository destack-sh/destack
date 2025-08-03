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
    builtin_entity,
    builtin_enum,
    builtin_event,
    builtin_property,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_event(NodeType.SANCTION_EVENT, is_abstract=True)
class SanctionEvent(Event):
    sanction: "Sanction" = builtin_property(101)
    target: "Entity" = builtin_property(110)


@builtin_event(NodeType.SANCTION_REQUESTED_EVENT)
class SanctionRequestedEvent(SanctionEvent):
    pass


@builtin_event(NodeType.SANCTION_GRANTED_EVENT)
class SanctionGrantedEvent(SanctionEvent):
    pass


@builtin_event(NodeType.SANCTION_REVOKED_EVENT)
class SanctionRevokedEvent(SanctionEvent):
    pass


@builtin_event(NodeType.SANCTION_EXPIRED_EVENT)
class SanctionExpiredEvent(SanctionEvent):
    pass


@builtin_enum(EnumType.SANCTION_TYPE)
class SanctionType(Enum):
    """A Type of Sanction."""

    BAN = 1
    MUTE = 2


@builtin_entity(
    NodeType.SANCTION,
    event_types=(NodeType.SANCTION_EVENT,),
)
class Sanction(
    Entity,
):
    """A Sanction on some Actor."""

    type: SanctionType = builtin_property(100)
    expires_at: Optional[datetime] = builtin_property(110)
    target: "Entity" = builtin_property(111)
    if TYPE_CHECKING:
        target_ptr: NodeReference = UNSET
