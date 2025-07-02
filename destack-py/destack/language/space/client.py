from datetime import datetime
from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    ClientType,
    Entity,
    HasName,
    IsDeletable,
    IsGlobal,
    IsSubject,
    NodeReference,
    NodeType,
    Origin,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)
from destack.utils.uuid import UUID

if TYPE_CHECKING:
    from destack.language import Cursor, Machine, User

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.CLIENT)
class Client(
    HasName,
    IsGlobal,
    IsDeletable,
    Entity,
):
    """A Client to connect with the system."""

    # meta
    parent: Optional[IsSubject] = builtin_property_parent(node_is_extensible=False)
    type: ClientType = builtin_property(30)
    machine: Optional["Machine"] = builtin_property(36)
    user: Optional["User"] = builtin_property(37)
    if TYPE_CHECKING:
        machine_ptr: Optional[NodeReference] = None
        user_ptr: Optional[NodeReference] = None

    # status
    access_token: Optional[str] = builtin_property(50, is_unique=True)
    seen_at: Optional[datetime] = builtin_property(51)
    logged_in_at: Optional[datetime] = builtin_property(52)
    cursor: Optional["Cursor"] = builtin_property(55)

    # details
    device_type: Optional[str] = builtin_property(40)
    device_name: Optional[str] = builtin_property(41)
    operating_system: Optional[str] = builtin_property(42)
    browser_name: Optional[str] = builtin_property(43)
    browser_version: Optional[str] = builtin_property(44)

    def to_origin(self, *, nonce: UUID | None) -> "Origin":
        return Origin(type=self.type, id=self.id, nonce=nonce or self.id)
