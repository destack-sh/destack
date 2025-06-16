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
from destack.proto import SanctionData, SanctionEventData
from destack.utils.uuid import UUID

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.SANCTION_EVENT_TYPE)
class SanctionEventType(Enum):
    """A Type of Sanction Event."""

    REQUESTED = 1
    GRANTED = 2
    REVOKED = 3
    EXPIRED = 4


@builtin_node(NodeType.SANCTION_EVENT)
class SanctionEvent(
    Event["Sanction"],
    Node[SanctionEventData],
):
    node: "Sanction" = property_(35)


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
    Node[SanctionData],
):
    """A Sanction on some Subject."""

    parent: Union["IsSubject", "IsJoinable", None] = property_parent_(node_is_customizable=True)
    type: SanctionType = property_(30)
    expires_at: Optional[datetime] = property_(40)
    target: IsSubject = property_(41)
    if TYPE_CHECKING:
        target_ptr: NodeReference = UNSET
        target_id: UUID = UNSET
        target_type: NodeType = UNSET
