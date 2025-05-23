from datetime import datetime
from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from bench.language.core import (
    ClientType,
    IsInBench,
    IsNamed,
    Node,
    NodeReference,
    NodeType,
    Struct,
    StructType,
    node_,
    p_internal,
    p_kernel,
    p_system,
    property_,
    struct_,
)
from bench.pb2 import ClientData, OriginData

if TYPE_CHECKING:
    from bench.language import Computer, Cursor, Space, User

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.CLIENT)
class Client(IsNamed, IsInBench, Node[ClientData]):
    """A Client to connect with the system."""

    # meta
    type: ClientType = property_(30)
    space: Optional["Space"] = p_system(35)
    computer: Optional["Computer"] = p_system(36)
    user: Optional["User"] = property_(37)
    if TYPE_CHECKING:
        space_id: Optional[UUID] = None
        space_ptr: Optional[NodeReference] = None
        computer_id: Optional[UUID] = None
        computer_ptr: Optional[NodeReference] = None
        user_id: Optional[UUID] = None
        user_ptr: Optional[NodeReference] = None

    # status
    access_token: Optional[str] = p_kernel(50, unique=True, sensitive=True)
    seen_at: Optional[datetime] = p_system(51)
    logged_in_at: Optional[datetime] = p_system(52)
    cursor: Optional["Cursor"] = property_(55)

    # details
    device_type: Optional[str] = property_(40)
    device_name: Optional[str] = property_(41)
    operating_system: Optional[str] = property_(42)
    browser_name: Optional[str] = property_(43)
    browser_version: Optional[str] = property_(44)

    @property
    def is_attached(self) -> bool:
        parent = self.parent
        if parent is None:
            return False
        return parent.is_attached

    def __content_str__(self) -> str:
        value_parts = []
        for prop in (
            "device_type",
            "device_name",
            "operating_system",
            "browser_name",
            "browser_version",
        ):
            value = getattr(self, prop)
            if value is not None:
                value_parts.append(value)
        return ", ".join(value_parts)

    def to_origin(self, *, nonce: UUID | None) -> "Origin":
        return Origin(type=self.type, id=self.id, nonce=nonce or self.id)


@struct_(StructType.ORIGIN)
class Origin(Struct[OriginData]):
    """Origin of something."""

    type: ClientType = p_internal(30)
    id: Optional[UUID] = p_internal(31)
    ck: Optional[UUID] = p_internal(32)
    nonce: Optional[UUID] = p_internal(33)
