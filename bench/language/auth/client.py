from datetime import datetime
from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from bench.language.core import (
    ClientType,
    IsGlobal,
    IsInBench,
    IsNamed,
    Node,
    NodeReference,
    NodeType,
    Struct,
    StructType,
    node_,
    property_,
    struct_,
)
from bench.pb2 import ClientData, OriginData

if TYPE_CHECKING:
    from bench.language import Cursor, Machine, Space, User

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.CLIENT)
class Client(IsNamed, IsInBench, IsGlobal, Node[ClientData]):
    """A Client to connect with the system."""

    # meta
    type: ClientType = property_(30)
    space: Optional["Space"] = property_(35, can_write="system")
    machine: Optional["Machine"] = property_(36, can_write="system")
    user: Optional["User"] = property_(37, can_write="system")
    if TYPE_CHECKING:
        space_id: Optional[UUID] = None
        space_ptr: Optional[NodeReference] = None
        machine_id: Optional[UUID] = None
        machine_ptr: Optional[NodeReference] = None
        user_id: Optional[UUID] = None
        user_ptr: Optional[NodeReference] = None

    # status
    access_token: Optional[str] = property_(
        50, is_unique=True, can_read="system", can_write="system"
    )
    seen_at: Optional[datetime] = property_(51, can_write="system")
    logged_in_at: Optional[datetime] = property_(52, can_write="system")
    cursor: Optional["Cursor"] = property_(55)

    # details
    device_type: Optional[str] = property_(40)
    device_name: Optional[str] = property_(41)
    operating_system: Optional[str] = property_(42)
    browser_name: Optional[str] = property_(43)
    browser_version: Optional[str] = property_(44)

    def to_origin(self, *, nonce: UUID | None) -> "Origin":
        return Origin(type=self.type, id=self.id, nonce=nonce or self.id)


@struct_(StructType.ORIGIN)
class Origin(Struct[OriginData]):
    """Origin of something."""

    type: ClientType = property_(30)
    id: Optional[UUID] = property_(31)
    ck: Optional[UUID] = property_(32)
    nonce: Optional[UUID] = property_(33)
