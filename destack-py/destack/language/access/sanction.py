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


@builtin_node(NodeType.SANCTION_EVENT, is_abstract=True)
class SanctionEvent(Event["Sanction"]):
    node: "Sanction" = builtin_property(101)
    target: "IsSubject" = builtin_property(110)


@builtin_node(NodeType.SANCTION_REQUESTED_EVENT)
class SanctionRequestedEvent(SanctionEvent):
    pass


@builtin_node(NodeType.SANCTION_GRANTED_EVENT)
class SanctionGrantedEvent(SanctionEvent):
    pass


@builtin_node(NodeType.SANCTION_REVOKED_EVENT)
class SanctionRevokedEvent(SanctionEvent):
    pass


@builtin_node(NodeType.SANCTION_EXPIRED_EVENT)
class SanctionExpiredEvent(SanctionEvent):
    pass


@builtin_enum(EnumType.SANCTION_TYPE)
class SanctionType(Enum):
    """A Type of Sanction."""

    BAN = 1
    MUTE = 2


@builtin_node(NodeType.SANCTION)
class Sanction(IsSpatial, IsDeletable, Entity):
    """A Sanction on some Subject."""

    parent: Union["IsSubject", "IsJoinable", None] = builtin_property_parent(
        node_is_extensible=True
    )
    type: SanctionType = builtin_property(100)
    expires_at: Optional[datetime] = builtin_property(110)
    target: IsSubject = builtin_property(111)
    if TYPE_CHECKING:
        target_ptr: NodeReference = UNSET
