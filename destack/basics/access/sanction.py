from datetime import datetime
from typing import TYPE_CHECKING, Optional

from destack.core import (
    Entity,
    EnumType,
    Event,
    NodeType,
    OptionEnum,
    ReferenceType,
    declare_entity,
    declare_enum,
    declare_event,
    declare_option,
    declare_property,
)

if TYPE_CHECKING:
    pass


@declare_event(NodeType.SANCTION_EVENT, is_abstract=True)
class SanctionEvent(Event):
    sanction: "Sanction" = declare_property(
        101,
        reference_type=ReferenceType.LOCATION,
    )
    target: "Entity" = declare_property(
        110,
        reference_type=ReferenceType.LOCATION,
    )


@declare_event(NodeType.SANCTION_REQUESTED_EVENT)
class SanctionRequestedEvent(SanctionEvent):
    pass


@declare_event(NodeType.SANCTION_GRANTED_EVENT)
class SanctionGrantedEvent(SanctionEvent):
    pass


@declare_event(NodeType.SANCTION_REVOKED_EVENT)
class SanctionRevokedEvent(SanctionEvent):
    pass


@declare_event(NodeType.SANCTION_EXPIRED_EVENT)
class SanctionExpiredEvent(SanctionEvent):
    pass


@declare_enum(EnumType.SANCTION_TYPE)
class SanctionType(OptionEnum):
    """A Type of Sanction."""

    BAN = declare_option(1, "Ban", description="A Ban")
    MUTE = declare_option(2, "Mute", description="A Mute")


@declare_entity(
    NodeType.SANCTION,
    event_types=(NodeType.SANCTION_EVENT,),
)
class Sanction(
    Entity,
):
    """A Sanction on some Actor."""

    type: SanctionType = declare_property(100)
    expires_at: Optional[datetime] = declare_property(110)
    target: "Entity" = declare_property(
        111,
        reference_type=ReferenceType.LOCATION,
    )
