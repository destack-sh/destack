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
    StructFrozen,
    StructType,
    builtin_node,
    builtin_struct,
    property_,
    property_parent_,
)
from destack.proto import OriginProto
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
    parent: Optional[IsSubject] = property_parent_(node_is_extensible=False)
    type: ClientType = property_(30)
    machine: Optional["Machine"] = property_(36)
    user: Optional["User"] = property_(37)
    if TYPE_CHECKING:
        machine_ptr: Optional[NodeReference] = None
        user_ptr: Optional[NodeReference] = None

    # status
    access_token: Optional[str] = property_(50, is_unique=True)
    seen_at: Optional[datetime] = property_(51)
    logged_in_at: Optional[datetime] = property_(52)
    cursor: Optional["Cursor"] = property_(55)

    # details
    device_type: Optional[str] = property_(40)
    device_name: Optional[str] = property_(41)
    operating_system: Optional[str] = property_(42)
    browser_name: Optional[str] = property_(43)
    browser_version: Optional[str] = property_(44)

    def to_origin(self, *, nonce: UUID | None) -> "Origin":
        return Origin(type=self.type, id=self.id, nonce=nonce or self.id)


@builtin_struct(StructType.ORIGIN, frozen=True)
class Origin(StructFrozen[OriginProto]):
    """Origin of something."""

    type: ClientType = property_(30)
    id: Optional[UUID] = property_(31)
    ck: Optional[UUID] = property_(32)
    nonce: Optional[UUID] = property_(33)
