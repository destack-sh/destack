from datetime import datetime
from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from bench.language.core import (
    ClientType,
    IsNamed,
    Node,
    NodeReference,
    NodeType,
    Struct,
    StructType,
    node_,
    p_internal,
    p_kernel,
    p_node_ancestor,
    p_regular,
    p_system,
    struct_,
)
from bench.pb2 import ClientData, OriginData

if TYPE_CHECKING:
    from bench.language import Bench, Computer, Cursor, Space, User

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.CLIENT)
class Client(IsNamed, Node[ClientData]):
    """A Client to connect with the system."""

    # meta
    bench: "Bench | None" = p_node_ancestor(5, NodeType.BENCH, require=False, store=True, wire=True)
    user: "User | None" = p_node_ancestor(7, NodeType.USER, require=False, store=True, wire=True)
    type: ClientType = p_regular(30)
    space: Optional["Space"] = p_system(35)
    computer: Optional["Computer"] = p_system(36)
    if TYPE_CHECKING:
        bench_id: Optional[UUID] = None
        bench_ptr: Optional[NodeReference] = None
        user_id: Optional[UUID] = None
        user_ptr: Optional[NodeReference] = None
        space_id: Optional[UUID] = None
        space_ptr: Optional[NodeReference] = None
        computer_id: Optional[UUID] = None
        computer_ptr: Optional[NodeReference] = None

    # status
    access_token: Optional[str] = p_kernel(50, unique=True, sensitive=True)
    seen_at: Optional[datetime] = p_system(51)
    logged_in_at: Optional[datetime] = p_system(52)
    cursor: Optional["Cursor"] = p_regular(55)
    if TYPE_CHECKING:
        cursor_id: Optional[UUID] = None
        cursor_ptr: Optional[NodeReference] = None

    # details
    device_type: Optional[str] = p_regular(40)
    device_name: Optional[str] = p_regular(41)
    operating_system: Optional[str] = p_regular(42)
    browser_name: Optional[str] = p_regular(43)
    browser_version: Optional[str] = p_regular(44)

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
