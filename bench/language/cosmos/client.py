from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.core import (
    TITLE_CONSTRAINT,
    ClientType,
    Node,
    NodeReference,
    NodeType,
    Struct,
    StructType,
    node_,
    p_internal,
    p_kernel,
    p_node_ancestor,
    p_node_parent,
    p_regular,
    p_system,
    struct_,
)
from bench.pb2 import ClientData, ClientOriginData

if TYPE_CHECKING:
    from bench.language import Bench, Machine, Space, User

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.CLIENT, roots=(NodeType.USER, NodeType.BENCH), has_subtypes=True)
class Client(Node[ClientData]):
    """A Client to connect with the system."""

    parent: Union["User", "Bench", None] = p_node_parent(4, NodeType.USER, NodeType.BENCH)
    bench: "Bench | None" = p_node_ancestor(5, NodeType.BENCH, require=False, store=True, wire=True)
    user: "User | None" = p_node_ancestor(7, NodeType.USER, require=False, store=True, wire=True)
    if TYPE_CHECKING:
        bench_id: Optional[UUID] = None
        bench_ptr: Optional[NodeReference] = None
        user_id: Optional[UUID] = None
        user_ptr: Optional[NodeReference] = None

    type: ClientType = p_regular(30)
    name: str = p_regular(32, constraint=TITLE_CONSTRAINT)

    device_type: Optional[str] = p_regular(40, default=None)
    device_name: Optional[str] = p_regular(41, default=None)
    operating_system: Optional[str] = p_regular(42, default=None)
    browser_name: Optional[str] = p_regular(43, default=None)
    browser_version: Optional[str] = p_regular(44, default=None)
    place_id: Optional[str] = p_regular(45, default=None)

    access_token: Optional[str] = p_kernel(
        50, default=None, defer=True, unique=True, sensitive=True
    )
    seen_at: Optional[datetime] = p_system(51, default=None)
    logged_in_at: Optional[datetime] = p_system(52, default=None)

    space: Optional["Space"] = p_system(60, array=False, require=False, references=NodeType.SPACE)
    machine: Optional["Machine"] = p_system(
        62, array=False, require=False, references=NodeType.MACHINE
    )

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

    def to_origin(self, *, nonce: UUID | None) -> "ClientOrigin":
        return ClientOrigin(type=self.type, id=self.id, nonce=nonce or self.id)


@struct_(StructType.CLIENT_ORIGIN)
class ClientOrigin(Struct[ClientOriginData]):
    """Information to identify a Client."""

    type: ClientType = p_internal(30, require=True)
    id: Optional[UUID] = p_internal(31, default=None)
    nonce: Optional[UUID] = p_internal(32, default=None)
